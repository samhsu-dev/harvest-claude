use std::collections::{HashSet, VecDeque};

use rand::Rng;

use crate::constants::{
    REST_DURATION_MAX, REST_DURATION_MIN, TILE_SIZE, TYPE_FRAME_MS, WAITING_BUBBLE_SECS,
    WALK_FRAME_MS, WALK_SPEED, WANDER_MOVES_MAX, WANDER_MOVES_MIN, WANDER_PAUSE_MAX,
    WANDER_PAUSE_MIN,
};
use crate::engine::matrix::MatrixEffect;
use crate::engine::pathfind;
use crate::engine::seat::Seat;
use crate::types::{AnimType, BubbleKind, CharState, CompanionKind, Direction, TilePos};

/// Walk animation frame cycle: 4 frames at 150ms.
const WALK_CYCLE: [u8; 4] = [0, 1, 2, 1];

/// Sentinel value: skip next rest period after a turn ends.
const SKIP_REST_SENTINEL: f32 = -1.0;

/// Speech bubble state attached to a character.
#[derive(Debug, Clone)]
pub struct BubbleState {
    /// Kind of bubble (Permission or Waiting).
    pub kind: BubbleKind,
    /// Countdown timer (for Waiting auto-fade). Unused for Permission.
    pub timer: f32,
}

/// Companion animal following a character (visualizes a sub-agent).
#[derive(Debug, Clone)]
pub struct Companion {
    /// Tool ID of the background agent this companion represents.
    pub tool_id: String,
    /// Animal kind determines the sprite.
    pub kind: CompanionKind,
    /// Pixel offset from parent character position.
    pub offset: (f32, f32),
    /// Animation frame index (2-frame idle bob).
    pub anim_frame: u8,
    /// Animation frame timer accumulator.
    pub anim_timer: f32,
}

/// Companion animation frame timing (ms).
const COMPANION_FRAME_MS: f32 = 400.0;

/// An agent character on the office grid with FSM-driven animation.
#[derive(Debug, Clone)]
pub struct Character {
    /// Unique agent identifier.
    pub agent_id: usize,
    /// Current FSM state.
    pub state: CharState,
    /// True when the agent has active tools running.
    pub is_active: bool,
    /// Sub-tile pixel position (col_px, row_px).
    pub pos: (f32, f32),
    /// Current movement target tile.
    pub target: Option<TilePos>,
    /// Remaining path tiles to walk.
    pub path: VecDeque<TilePos>,
    /// Current facing direction.
    pub direction: Direction,
    /// Palette index for sprite coloring.
    pub palette: u8,
    /// Optional hue shift in degrees.
    pub hue_shift: Option<i16>,
    /// Assigned seat index, if any.
    pub seat_id: Option<usize>,
    /// Rest countdown at seat. -1.0 = skip next rest.
    pub seat_timer: f32,
    /// Current animation frame index.
    pub anim_frame: u8,
    /// Animation frame timer accumulator.
    pub anim_timer: f32,
    /// Timer until next wander path.
    pub wander_timer: f32,
    /// Number of completed wander paths since last rest.
    pub wander_count: u8,
    /// Max wander paths before returning to seat for rest.
    pub wander_limit: u8,
    /// Name of the currently active tool, if any.
    pub tool_name: Option<String>,
    /// Active speech bubble, if any.
    pub bubble: Option<BubbleState>,
    /// Active matrix spawn/despawn effect, if any.
    pub matrix_effect: Option<MatrixEffect>,
    /// Companion animals (from background sub-agents).
    pub companions: Vec<Companion>,
}

impl Character {
    /// Create a new character at the given tile position in Idle state.
    pub fn new(agent_id: usize, pos: TilePos, palette: u8, hue_shift: Option<i16>) -> Self {
        let px = (
            pos.0 as f32 * TILE_SIZE as f32,
            pos.1 as f32 * TILE_SIZE as f32,
        );
        let mut rng = rand::rng();
        Self {
            agent_id,
            state: CharState::Idle,
            is_active: false,
            pos: px,
            target: None,
            path: VecDeque::new(),
            direction: Direction::Down,
            palette,
            hue_shift,
            seat_id: None,
            seat_timer: 0.0,
            anim_frame: 0,
            anim_timer: 0.0,
            wander_timer: rng.random_range(WANDER_PAUSE_MIN..WANDER_PAUSE_MAX),
            wander_count: 0,
            wander_limit: rng.random_range(WANDER_MOVES_MIN..=WANDER_MOVES_MAX),
            tool_name: None,
            bubble: None,
            matrix_effect: None,
            companions: Vec::new(),
        }
    }

    /// Tick the character FSM: position, animation, wander logic.
    pub fn update(&mut self, dt: f64, walkable: &HashSet<TilePos>, seats: &[Seat]) {
        let dt_f32 = dt as f32;

        // Update bubble timer
        if let Some(ref mut bubble) = self.bubble
            && matches!(bubble.kind, BubbleKind::Waiting)
        {
            bubble.timer -= dt_f32;
            if bubble.timer <= 0.0 {
                self.bubble = None;
            }
        }

        // Update matrix effect
        if let Some(ref mut fx) = self.matrix_effect
            && fx.update(dt_f32)
        {
            self.matrix_effect = None;
        }

        // Update companion animations
        for comp in &mut self.companions {
            comp.anim_timer += dt_f32 * 1000.0;
            if comp.anim_timer >= COMPANION_FRAME_MS {
                comp.anim_timer -= COMPANION_FRAME_MS;
                comp.anim_frame = (comp.anim_frame + 1) % 2;
            }
        }

        match self.state {
            CharState::Idle => self.update_idle(dt_f32, walkable, seats),
            CharState::Walk => self.update_walk(dt_f32, seats),
            CharState::Type => self.update_type(dt_f32),
        }
    }

    /// Activate: walk to seat then type. Called when a tool starts.
    pub fn set_active(
        &mut self,
        tool_name: &str,
        seat_pos: Option<TilePos>,
        path: VecDeque<TilePos>,
    ) {
        self.is_active = true;
        self.tool_name = Some(tool_name.to_owned());

        if path.is_empty() {
            if let Some(sp) = seat_pos
                && self.current_tile() == sp
            {
                self.state = CharState::Type;
                self.anim_frame = 0;
                self.anim_timer = 0.0;
                return;
            }
            // No path: start typing immediately
            self.state = CharState::Type;
            self.anim_frame = 0;
            self.anim_timer = 0.0;
            return;
        }

        self.path = path;
        self.target = self.path.front().copied();
        self.state = CharState::Walk;
        self.anim_frame = 0;
        self.anim_timer = 0.0;
        self.wander_count = 0;
    }

    /// Transition to idle. Sets seat_timer sentinel to skip next rest.
    pub fn set_idle(&mut self) {
        self.is_active = false;
        self.tool_name = None;
        self.seat_timer = SKIP_REST_SENTINEL;
        if self.state == CharState::Type {
            self.state = CharState::Idle;
            self.anim_frame = 0;
            self.anim_timer = 0.0;
            let mut rng = rand::rng();
            self.wander_timer = rng.random_range(WANDER_PAUSE_MIN..WANDER_PAUSE_MAX);
        }
    }

    /// Show a waiting bubble with auto-fade timer.
    pub fn set_waiting(&mut self) {
        self.bubble = Some(BubbleState {
            kind: BubbleKind::Waiting,
            timer: WAITING_BUBBLE_SECS,
        });
    }

    /// Show a persistent permission bubble.
    pub fn set_permission(&mut self) {
        self.bubble = Some(BubbleState {
            kind: BubbleKind::Permission,
            timer: 0.0,
        });
    }

    /// Clear any active bubble.
    pub fn dismiss_bubble(&mut self) {
        self.bubble = None;
    }

    /// Spawn a companion animal for a background sub-agent.
    pub fn add_companion(&mut self, tool_id: String, kind: CompanionKind) {
        // Offset companions so they don't overlap
        let idx = self.companions.len();
        let offset_x = match idx % 3 {
            0 => 10.0,
            1 => -6.0,
            _ => 12.0,
        };
        let offset_y = match idx % 3 {
            0 => 4.0,
            1 => 6.0,
            _ => -2.0,
        };
        self.companions.push(Companion {
            tool_id,
            kind,
            offset: (offset_x, offset_y),
            anim_frame: 0,
            anim_timer: 0.0,
        });
    }

    /// Remove a companion by its tool ID (background agent finished).
    pub fn remove_companion(&mut self, tool_id: &str) {
        self.companions.retain(|c| c.tool_id != tool_id);
    }

    /// Current sprite lookup key: (direction, anim_type, frame).
    pub fn sprite_key(&self) -> (Direction, AnimType, u8) {
        match self.state {
            CharState::Idle => (self.direction, AnimType::Walk, 0),
            CharState::Walk => {
                let cycle_idx = (self.anim_frame as usize) % WALK_CYCLE.len();
                (self.direction, AnimType::Walk, WALK_CYCLE[cycle_idx])
            }
            CharState::Type => {
                let anim = if self.tool_name.as_deref().is_some_and(Self::is_reading_tool) {
                    AnimType::Read
                } else {
                    AnimType::Type
                };
                (self.direction, anim, self.anim_frame % 2)
            }
        }
    }

    /// True if the tool name indicates a reading action.
    pub fn is_reading_tool(tool_name: &str) -> bool {
        matches!(
            tool_name,
            "Read" | "Grep" | "Glob" | "WebFetch" | "WebSearch"
        )
    }

    /// Current tile position derived from pixel coordinates.
    pub fn current_tile(&self) -> TilePos {
        (
            (self.pos.0 / TILE_SIZE as f32) as u16,
            (self.pos.1 / TILE_SIZE as f32) as u16,
        )
    }

    fn update_idle(&mut self, dt: f32, walkable: &HashSet<TilePos>, seats: &[Seat]) {
        // Resting at seat
        if self.seat_timer > 0.0 {
            self.seat_timer -= dt;
            return;
        }

        self.wander_timer -= dt;
        if self.wander_timer > 0.0 {
            return;
        }

        self.try_start_wander(walkable, seats);
    }

    fn try_start_wander(&mut self, walkable: &HashSet<TilePos>, seats: &[Seat]) {
        let current = self.current_tile();

        // After wander_limit paths: return to seat for rest
        if self.wander_count >= self.wander_limit
            && let Some(seat_idx) = self.seat_id
            && let Some(seat) = seats.get(seat_idx)
        {
            let seat_pos = seat.tile_pos();
            if let Some(path_vec) = pathfind::bfs(walkable, current, seat_pos, Some(seat_pos)) {
                self.path = VecDeque::from(path_vec);
                self.target = self.path.front().copied();
                self.state = CharState::Walk;
                self.anim_frame = 0;
                self.anim_timer = 0.0;
                return;
            }
        }

        // Pick a random walkable tile
        let walkable_vec: Vec<TilePos> = walkable.iter().copied().collect();
        if walkable_vec.is_empty() {
            return;
        }

        let mut rng = rand::rng();
        let dest = walkable_vec[rng.random_range(0..walkable_vec.len())];

        let own_seat = self
            .seat_id
            .and_then(|si| seats.get(si))
            .map(|s| s.tile_pos());

        match pathfind::bfs(walkable, current, dest, own_seat) {
            Some(path_vec) if !path_vec.is_empty() => {
                self.path = VecDeque::from(path_vec);
                self.target = self.path.front().copied();
                self.state = CharState::Walk;
                self.anim_frame = 0;
                self.anim_timer = 0.0;
            }
            _ => {
                self.wander_timer = rng.random_range(WANDER_PAUSE_MIN..WANDER_PAUSE_MAX);
            }
        }
    }

    fn update_walk(&mut self, dt: f32, seats: &[Seat]) {
        // Animate walk cycle
        self.anim_timer += dt * 1000.0;
        if self.anim_timer >= WALK_FRAME_MS {
            self.anim_timer -= WALK_FRAME_MS;
            self.anim_frame = self.anim_frame.wrapping_add(1) % WALK_CYCLE.len() as u8;
        }

        let Some(target) = self.target else {
            self.arrive_at_destination(seats);
            return;
        };

        let target_px = (
            target.0 as f32 * TILE_SIZE as f32,
            target.1 as f32 * TILE_SIZE as f32,
        );

        let dx = target_px.0 - self.pos.0;
        let dy = target_px.1 - self.pos.1;

        // Update facing direction
        if dx.abs() > dy.abs() {
            self.direction = if dx > 0.0 {
                Direction::Right
            } else {
                Direction::Left
            };
        } else if dy.abs() > f32::EPSILON {
            self.direction = if dy > 0.0 {
                Direction::Down
            } else {
                Direction::Up
            };
        }

        let move_amount = WALK_SPEED * TILE_SIZE as f32 * dt;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= move_amount {
            // Snap to target tile
            self.pos = target_px;
            self.path.pop_front();
            self.target = self.path.front().copied();

            if self.target.is_none() {
                self.arrive_at_destination(seats);
            }
        } else {
            let ratio = move_amount / dist;
            self.pos.0 += dx * ratio;
            self.pos.1 += dy * ratio;
        }
    }

    fn arrive_at_destination(&mut self, seats: &[Seat]) {
        if self.is_active {
            // At seat with active tool: start typing
            self.state = CharState::Type;
            self.anim_frame = 0;
            self.anim_timer = 0.0;

            if let Some(seat) = self.seat_id.and_then(|si| seats.get(si)) {
                self.direction = seat.facing;
            }
            return;
        }

        // Finished a wander path
        self.wander_count += 1;
        self.state = CharState::Idle;
        self.anim_frame = 0;
        self.anim_timer = 0.0;

        let at_seat = self
            .seat_id
            .and_then(|si| seats.get(si))
            .is_some_and(|s| s.tile_pos() == self.current_tile());

        if at_seat && self.wander_count >= self.wander_limit {
            if self.seat_timer < -0.5 {
                // Sentinel: skip this rest
                self.seat_timer = 0.0;
            } else {
                let mut rng = rand::rng();
                self.seat_timer = rng.random_range(REST_DURATION_MIN..REST_DURATION_MAX);
            }
            self.wander_count = 0;
            let mut rng = rand::rng();
            self.wander_limit = rng.random_range(WANDER_MOVES_MIN..=WANDER_MOVES_MAX);

            if let Some(seat) = self.seat_id.and_then(|si| seats.get(si)) {
                self.direction = seat.facing;
            }
        }

        let mut rng = rand::rng();
        self.wander_timer = rng.random_range(WANDER_PAUSE_MIN..WANDER_PAUSE_MAX);
    }

    fn update_type(&mut self, dt: f32) {
        self.anim_timer += dt * 1000.0;
        if self.anim_timer >= TYPE_FRAME_MS {
            self.anim_timer -= TYPE_FRAME_MS;
            self.anim_frame = (self.anim_frame + 1) % 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{BubbleState, Character};
    use crate::types::{AnimType, BubbleKind, CharState, Direction};

    #[test]
    fn new_character_is_idle() {
        let ch = Character::new(0, (3, 4), 0, None);
        assert_eq!(ch.state, CharState::Idle);
        assert_eq!(ch.agent_id, 0);
    }

    #[test]
    fn sprite_key_idle_is_walk_frame_0() {
        let ch = Character::new(0, (0, 0), 0, None);
        let (dir, anim, frame) = ch.sprite_key();
        assert_eq!(dir, Direction::Down);
        assert_eq!(anim, AnimType::Walk);
        assert_eq!(frame, 0);
    }

    #[test]
    fn is_reading_tool_matches() {
        assert!(Character::is_reading_tool("Read"));
        assert!(Character::is_reading_tool("Grep"));
        assert!(Character::is_reading_tool("Glob"));
        assert!(Character::is_reading_tool("WebFetch"));
        assert!(Character::is_reading_tool("WebSearch"));
        assert!(!Character::is_reading_tool("Write"));
        assert!(!Character::is_reading_tool("Bash"));
    }

    #[test]
    fn set_active_transitions_to_walk() {
        let mut ch = Character::new(0, (0, 0), 0, None);
        let path = vec![(1, 0), (2, 0)].into_iter().collect();
        ch.set_active("Bash", Some((2, 0)), path);
        assert!(ch.is_active);
        assert_eq!(ch.state, CharState::Walk);
    }

    #[test]
    fn set_active_empty_path_goes_to_type() {
        let mut ch = Character::new(0, (2, 2), 0, None);
        ch.set_active("Bash", Some((2, 2)), std::collections::VecDeque::new());
        assert_eq!(ch.state, CharState::Type);
    }

    #[test]
    fn set_idle_clears_active() {
        let mut ch = Character::new(0, (0, 0), 0, None);
        ch.state = CharState::Type;
        ch.is_active = true;
        ch.tool_name = Some("Bash".to_owned());
        ch.set_idle();
        assert!(!ch.is_active);
        assert!(ch.tool_name.is_none());
        assert_eq!(ch.seat_timer, -1.0);
    }

    #[test]
    fn waiting_bubble_auto_fades() {
        let mut ch = Character::new(0, (0, 0), 0, None);
        ch.set_waiting();
        assert!(ch.bubble.is_some());

        let walkable = HashSet::new();
        ch.update(3.0, &walkable, &[]);
        assert!(ch.bubble.is_none());
    }

    #[test]
    fn permission_bubble_persists() {
        let mut ch = Character::new(0, (0, 0), 0, None);
        ch.set_permission();
        assert!(matches!(
            ch.bubble,
            Some(BubbleState {
                kind: BubbleKind::Permission,
                ..
            })
        ));

        let walkable = HashSet::new();
        ch.update(10.0, &walkable, &[]);
        assert!(ch.bubble.is_some());
    }

    #[test]
    fn dismiss_bubble_clears() {
        let mut ch = Character::new(0, (0, 0), 0, None);
        ch.set_permission();
        ch.dismiss_bubble();
        assert!(ch.bubble.is_none());
    }

    #[test]
    fn current_tile_calculation() {
        let ch = Character::new(0, (3, 5), 0, None);
        assert_eq!(ch.current_tile(), (3, 5));
    }
}
