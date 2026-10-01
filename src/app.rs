use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use color_eyre::eyre::{Result, WrapErr, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};

use crate::action::Action;
use crate::cli::Args;
use crate::constants::{
    JSONL_POLL_MS, LABEL_MAX_WIDTH, MAX_AGENTS, MAX_DELTA_TIME, MIN_TERMINAL_COLS,
    MIN_TERMINAL_ROWS, TILE_SIZE,
};
use crate::engine::pathfind;
use crate::engine::state::OfficeState;
use crate::engine::warehouse::{self, Warehouse, produce_for_anim};
use crate::event::EventHandler;
use crate::layout::persistence;
use crate::render::buffer::PixelBuffer;
use crate::render::composer::{
    self, BubbleRender, CharacterRender, CompanionRender, FurnitureRender, SceneInput,
};
use crate::tui::TerminalGuard;
use crate::types::{AgentEvent, AgentStatus, AnimType, BubbleKind, CompanionKind, ProduceType};
use crate::ui::input;
use crate::ui::status_bar::{AgentPanel, AgentRow, ProduceCounts};
use crate::ui::text;
use crate::watcher::agent::ApplyOutcome;
use crate::watcher::focus;
use crate::watcher::parser;
use crate::watcher::registry::AgentRegistry;
use crate::watcher::scanner::{DirectoryScanner, ScanEvent, session_info};
use crate::watcher::timer::{PollInterval, TimerEvent, TimerManager};

/// Application orchestrator. Receives `Action`, updates state, renders.
pub struct App {
    office: OfficeState,
    agents: AgentRegistry,
    scanner: DirectoryScanner,
    timers: TimerManager,
    reader_poll: PollInterval,
    warehouse: Warehouse,
    config_dir: Option<std::path::PathBuf>,
    selected: Option<usize>,
    running: bool,
    /// Render area origin in terminal cells (for mouse offset).
    render_origin: (u16, u16),
}

/// Load the warehouse from the config directory. When the file cannot be read, persistence is
/// disabled so the unreadable file is never overwritten.
fn load_persistent_warehouse(
    config_dir: Option<std::path::PathBuf>,
) -> (Option<PathBuf>, Warehouse) {
    let Some(dir) = config_dir else {
        return (None, Warehouse::default());
    };
    match warehouse::load_warehouse(&dir) {
        Ok(wh) => (Some(dir), wh),
        Err(e) => {
            tracing::warn!("warehouse persistence disabled: {e:#}");
            (None, Warehouse::default())
        }
    }
}

impl App {
    /// Create a new App from CLI arguments.
    ///
    /// # Errors
    /// Returns an error when an explicit `--watch-dir` is not a directory, the layout cannot be
    /// loaded, or the initial scan fails.
    pub fn new(args: Args) -> Result<Self> {
        for dir in &args.watch_dir {
            if !dir.is_dir() {
                bail!("--watch-dir {} is not a directory", dir.display());
            }
        }
        let layout = persistence::load_or_default(args.layout.as_deref())
            .wrap_err("failed to load layout")?;
        let office = OfficeState::from_layout(layout);

        let watch_dirs = if args.watch_dir.is_empty() {
            let home = dirs::home_dir().unwrap_or_default();
            vec![home.join(".claude").join("projects")]
        } else {
            args.watch_dir.clone()
        };

        let scanner =
            DirectoryScanner::new(watch_dirs).wrap_err("failed to create directory scanner")?;

        let agents = AgentRegistry::new();

        let (config_dir, wh) = load_persistent_warehouse(persistence::config_dir().ok());

        let mut app = Self {
            office,
            agents,
            scanner,
            timers: TimerManager::new(),
            reader_poll: PollInterval::new(JSONL_POLL_MS),
            warehouse: wh,
            config_dir,
            selected: None,
            running: true,
            render_origin: (0, 0),
        };
        app.adopt_existing_sessions()?;
        Ok(app)
    }

    /// Adopt the transcripts found by the initial scan, newest first, up to `MAX_AGENTS`.
    fn adopt_existing_sessions(&mut self) -> Result<()> {
        let initial = self
            .scanner
            .initial_scan()
            .wrap_err("failed initial scan")?;
        for path in initial {
            if let Some((project_name, session_id)) = session_info(&path) {
                self.adopt_session(path, project_name, session_id);
            }
        }
        Ok(())
    }

    /// Run the main event loop.
    pub fn run(&mut self, terminal: &mut TerminalGuard) -> Result<()> {
        let events = EventHandler::new();

        loop {
            let action = events.next().wrap_err("event receive failed")?;
            let should_render = matches!(action, Action::Render);
            self.update(action);
            if !self.running {
                break;
            }
            if should_render {
                self.render(terminal)?;
            }
        }
        Ok(())
    }

    /// Number of agents that count toward `MAX_AGENTS` (every non-Dormant agent).
    fn live_agent_count(&self) -> usize {
        self.agents
            .agents()
            .iter()
            .filter(|a| a.status != AgentStatus::Dormant)
            .count()
    }

    /// Adopt a transcript as a new agent. Returns false when refused at the limit
    /// or when the transcript cannot be opened.
    fn adopt_session(
        &mut self,
        path: std::path::PathBuf,
        project_name: String,
        session_id: String,
    ) -> bool {
        if self.live_agent_count() >= MAX_AGENTS {
            return false;
        }
        let id = match self.agents.add_agent(session_id, path, project_name) {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!("failed to adopt session: {e:#}");
                return false;
            }
        };
        if let Err(e) = self.agents.replay(id) {
            tracing::warn!(agent_id = id, "failed to replay transcript: {e:#}");
            self.agents.remove_agent(id);
            return false;
        }
        if let Some(agent) = self.agents.get(id) {
            self.office
                .add_character(id, agent.palette, agent.hue_shift);
        }
        self.sync_replayed(id);
        true
    }

    // Shows what the replayed transcript says the agent is doing, without a permission bubble.
    fn sync_replayed(&mut self, agent_id: usize) {
        let Some(agent) = self.agents.get(agent_id) else {
            return;
        };
        let tool_name = agent.tools.last().map(|tool| tool.name.clone());
        let needs_approval = agent.is_running_non_exempt();
        let background: Vec<String> = agent.background_tool_ids.iter().cloned().collect();
        if let Some(tool_name) = tool_name {
            self.send_to_seat(agent_id, &tool_name);
        }
        if needs_approval {
            self.timers.start_permission(agent_id);
        }
        for tool_id in background {
            self.add_companion(agent_id, &tool_id);
        }
    }

    /// Remove the agent tracking `path`, with its character, timers, and selection.
    fn remove_session(&mut self, path: &std::path::Path) {
        let id = self
            .agents
            .agents()
            .iter()
            .find(|a| a.jsonl_path == path)
            .map(|a| a.id);
        if let Some(id) = id {
            self.office.remove_character(id);
            self.agents.remove_agent(id);
            self.timers.cancel_all(id);
            if self.selected == Some(id) {
                self.selected = None;
            }
        }
    }

    /// Match action, mutate state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Tick(dt) => self.on_tick(dt),
            Action::Render => {}
            Action::Resize(_, _) => {}
            Action::Key(key) => self.on_key(key),
            Action::Mouse(mouse) => {
                if let MouseEventKind::Down(_) = mouse.kind {
                    self.on_mouse_click(mouse.column, mouse.row);
                }
            }
            Action::Quit => self.running = false,
        }
    }

    fn on_tick(&mut self, dt: f64) {
        let dt = dt.min(MAX_DELTA_TIME);

        // Poll scanner for new/gone sessions
        for event in self.scanner.poll() {
            match event {
                ScanEvent::NewSession {
                    path,
                    project_name,
                    session_id,
                } => {
                    // Check if this is a dormant agent waking up.
                    let dormant_id = self
                        .agents
                        .agents()
                        .iter()
                        .find(|a| a.jsonl_path == path && a.status == AgentStatus::Dormant)
                        .map(|a| a.id);
                    if let Some(id) = dormant_id {
                        // Wake up: restore agent and character
                        if let Some(agent) = self.agents.get_mut(id) {
                            agent.status = AgentStatus::Idle;
                        }
                        if let Some(ch) = self.office.character_by_agent_mut(id) {
                            ch.wake_from_dormant();
                        }
                        continue;
                    }
                    // Skip if already registered (e.g. sub-agent tracked by parent)
                    if self.agents.has_path(&path) {
                        continue;
                    }
                    if !self.adopt_session(path.clone(), project_name, session_id) {
                        // Refused at the limit: let the next write report it again.
                        self.scanner.forget(&path);
                    }
                }
                ScanEvent::SessionDormant { path } => {
                    let id = self
                        .agents
                        .agents()
                        .iter()
                        .find(|a| a.jsonl_path == path)
                        .map(|a| a.id);
                    if let Some(id) = id {
                        if let Some(agent) = self.agents.get_mut(id) {
                            agent.status = AgentStatus::Dormant;
                        }
                        self.timers.cancel_all(id);
                        // Walk character to HOME then hide
                        self.start_dormant_walk(id);
                        if self.selected == Some(id) {
                            self.selected = None;
                        }
                    }
                }
                ScanEvent::SessionGone { path } => self.remove_session(&path),
            }
        }

        // Poll JSONL readers at their own cadence, after the scanner so a wake is seen first.
        let readers_due = self.reader_poll.due(Instant::now());
        let polled = if readers_due {
            self.agents.poll_all()
        } else {
            Vec::new()
        };
        for (agent_id, events) in polled {
            self.timers.cancel_text_idle(agent_id);
            for (event, outcome) in events {
                self.handle_agent_event(agent_id, event, outcome);
            }
        }

        // Check expired timers
        for timer_event in self.timers.check_expired() {
            match timer_event {
                TimerEvent::PermissionTimeout { agent_id } => self.apply_permission(agent_id),
                TimerEvent::TextIdleTimeout { agent_id } => self.apply_waiting(agent_id),
            }
        }

        // Check for delivery arrivals (characters that reached the barn)
        self.collect_deliveries();

        self.office.update(dt);
    }

    /// Apply the side effects of one transcript event the agent has already absorbed.
    fn handle_agent_event(&mut self, agent_id: usize, event: AgentEvent, outcome: ApplyOutcome) {
        match event {
            AgentEvent::ToolStart { tool_name, .. } => self.on_tool_start(agent_id, &tool_name),
            AgentEvent::ToolDone {
                tool_id,
                background_launch,
                ..
            } => {
                self.timers.cancel_permission(agent_id);
                self.clear_permission_bubble(agent_id);
                if background_launch {
                    self.add_companion(agent_id, &tool_id);
                }
                if outcome.background_finished
                    && let Some(ch) = self.office.character_by_agent_mut(agent_id)
                {
                    ch.remove_companion(&tool_id);
                }
            }
            AgentEvent::TurnEnd => self.on_turn_end(agent_id, outcome.turn_had_tools),
            AgentEvent::Text { .. } => {
                let tool_free = self
                    .agents
                    .get(agent_id)
                    .is_some_and(|a| !a.had_tools_in_turn);
                if tool_free {
                    self.timers.start_text_idle(agent_id);
                }
            }
            AgentEvent::Title { .. } | AgentEvent::Prompt { .. } | AgentEvent::Identity { .. } => {}
        }
    }

    // The bubble goes once the agent has left the Permission status.
    fn clear_permission_bubble(&mut self, agent_id: usize) {
        let still_waiting_for_approval = self
            .agents
            .get(agent_id)
            .is_some_and(|a| a.status == AgentStatus::Permission);
        if still_waiting_for_approval {
            return;
        }
        if let Some(ch) = self.office.character_by_agent_mut(agent_id)
            && ch
                .bubble
                .as_ref()
                .is_some_and(|b| b.kind == BubbleKind::Permission)
        {
            ch.dismiss_bubble();
        }
    }

    fn on_tool_start(&mut self, agent_id: usize, tool_name: &str) {
        if parser::is_non_exempt_tool(tool_name) {
            self.timers.start_permission(agent_id);
        }
        // Wake dormant characters on new tool activity
        if let Some(ch) = self.office.character_by_agent_mut(agent_id)
            && ch.is_dormant
        {
            ch.wake_from_dormant();
        }
        self.send_to_seat(agent_id, tool_name);
    }

    // Marks the character active and walks it to its seat.
    fn send_to_seat(&mut self, agent_id: usize, tool_name: &str) {
        // Compute path before mutable borrow of character
        let seat_and_path = self.office.character_by_agent(agent_id).map(|ch| {
            let seat_pos = ch.seat_id.map(|sid| {
                let seat = &self.office.seats[sid];
                (seat.col, seat.row)
            });
            let from = ch.current_tile();
            let path = seat_pos
                .and_then(|sp| pathfind::bfs(&self.office.walkable, from, sp, Some(sp)))
                .unwrap_or_default();
            (seat_pos, path)
        });
        if let Some((seat_pos, path)) = seat_and_path
            && let Some(ch) = self.office.character_by_agent_mut(agent_id)
        {
            ch.set_active(tool_name, seat_pos, VecDeque::from(path));
        }
    }

    fn on_turn_end(&mut self, agent_id: usize, turn_had_tools: bool) {
        self.timers.cancel_all(agent_id);
        // Determine produce from work animation, then deliver to barn
        let delivered = turn_had_tools && self.start_barn_delivery(agent_id);
        if !delivered && let Some(ch) = self.office.character_by_agent_mut(agent_id) {
            ch.set_idle();
            ch.set_waiting();
        }
    }

    // A companion animal stands for a background agent running next to its parent.
    fn add_companion(&mut self, agent_id: usize, tool_id: &str) {
        let kind = pick_companion_kind(agent_id, tool_id);
        if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
            ch.add_companion(tool_id.to_owned(), kind);
        }
    }

    fn apply_permission(&mut self, agent_id: usize) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = AgentStatus::Permission;
        }
        if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
            ch.set_permission();
        }
    }

    fn apply_waiting(&mut self, agent_id: usize) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = AgentStatus::Waiting;
        }
        if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
            ch.set_waiting();
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('c') if ctrl => self.running = false,
            KeyCode::Char('q') | KeyCode::Char('Q') => self.running = false,
            KeyCode::Esc => {
                if let Some(id) = self.selected
                    && let Some(ch) = self.office.character_by_agent_mut(id)
                    && ch.bubble.is_some()
                {
                    ch.dismiss_bubble();
                    return;
                }
                self.selected = None;
            }
            KeyCode::Enter => {
                if let Some(id) = self.selected {
                    self.focus_agent(id);
                }
            }
            KeyCode::Left => self.cycle_selection(false),
            KeyCode::Right => self.cycle_selection(true),
            _ => {}
        }
    }

    fn focus_agent(&self, agent_id: usize) {
        match self.agents.get(agent_id).and_then(|a| a.cwd.clone()) {
            Some(cwd) => focus::focus_agent_window(cwd),
            None => tracing::info!(agent_id, "no working directory known yet, cannot focus"),
        }
    }

    // Agents the user can select: every agent that is not resting.
    fn selectable_ids(&self) -> Vec<usize> {
        self.office
            .characters
            .iter()
            .map(|ch| ch.agent_id)
            .filter(|&id| self.is_selectable(id))
            .collect()
    }

    fn is_selectable(&self, agent_id: usize) -> bool {
        self.agents
            .get(agent_id)
            .is_some_and(|a| a.status != AgentStatus::Dormant)
    }

    fn cycle_selection(&mut self, forward: bool) {
        let ids = self.selectable_ids();
        let count = ids.len();
        let current = self
            .selected
            .and_then(|selected| ids.iter().position(|&id| id == selected));
        let next = match current {
            _ if count == 0 => None,
            Some(idx) if forward => Some((idx + 1) % count),
            Some(idx) => Some((idx + count - 1) % count),
            None => Some(0),
        };
        self.selected = next.map(|idx| ids[idx]);
    }

    fn on_mouse_click(&mut self, col: u16, row: u16) {
        let (ox, oy) = self.render_origin;
        let hit = input::hit_test_character(&self.office, col, row, ox, oy)
            .filter(|&id| self.is_selectable(id));
        let Some(agent_id) = hit else {
            self.selected = None;
            return;
        };

        // If clicking the selected character's bubble, dismiss it
        if self.selected == Some(agent_id)
            && let Some(ch) = self.office.character_by_agent_mut(agent_id)
            && ch.bubble.is_some()
        {
            ch.dismiss_bubble();
            return;
        }

        // Clicking an already-selected character: focus its terminal window
        if self.selected == Some(agent_id) {
            self.focus_agent(agent_id);
            return;
        }

        // Select the clicked character
        self.selected = Some(agent_id);
    }

    /// Walk a character to the nearest HOME furniture, then set dormant on arrival.
    fn start_dormant_walk(&mut self, agent_id: usize) {
        let from = match self.office.character_by_agent(agent_id) {
            Some(ch) => ch.current_tile(),
            None => return,
        };

        let home_tile = self.office.find_near_furniture(from, "HOME");
        let path = home_tile
            .and_then(|ht| pathfind::bfs(&self.office.walkable, from, ht, None))
            .unwrap_or_default();

        if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
            ch.start_dormant_walk(VecDeque::from(path));
        }
    }

    /// Determine produce type and start a delivery walk to the barn.
    ///
    /// Returns true if the character started walking to the barn.
    fn start_barn_delivery(&mut self, agent_id: usize) -> bool {
        let ch = match self.office.character_by_agent(agent_id) {
            Some(ch) => ch,
            None => return false,
        };

        // Determine produce from the seat's work_anim
        let anim = ch.work_anim.unwrap_or(AnimType::Farm);
        let Some(produce) = produce_for_anim(anim) else {
            return false;
        };

        let from = ch.current_tile();
        let barn_tile = self.office.find_near_furniture(from, "BARN");
        let path = barn_tile
            .and_then(|bt| pathfind::bfs(&self.office.walkable, from, bt, None))
            .unwrap_or_default();

        if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
            ch.set_idle();
            ch.set_waiting();
            ch.start_delivery(produce, VecDeque::from(path))
        } else {
            false
        }
    }

    /// Check all characters for completed deliveries and deposit produce.
    fn collect_deliveries(&mut self) {
        let mut deposited = false;
        for ch in &mut self.office.characters {
            // Character arrived at barn (idle with pending_delivery)
            if ch.state == crate::types::CharState::Idle
                && let Some(produce) = ch.take_delivery()
            {
                self.warehouse.add(produce);
                deposited = true;
            }
        }
        if deposited {
            self.save_warehouse();
        }
    }

    /// Persist warehouse to disk.
    fn save_warehouse(&self) {
        if let Some(ref dir) = self.config_dir
            && let Err(e) = warehouse::save_warehouse(dir, &self.warehouse)
        {
            tracing::warn!("failed to save warehouse: {e}");
        }
    }

    /// Compose scene + status bar.
    // One row per agent that is not resting, in agent id order.
    fn agent_rows(&self) -> Vec<AgentRow> {
        self.agents
            .agents()
            .iter()
            .filter(|a| a.status != AgentStatus::Dormant)
            .map(|a| AgentRow {
                color: palette_color(a.palette),
                identity: a.display_name(),
                status: a.status,
                summary: a.summary(),
                selected: self.selected == Some(a.id),
            })
            .collect()
    }

    // Whether the farm needs more terminal cells than the scene area has.
    fn scene_is_cropped(&self, scene: Rect) -> bool {
        let (cols, rows) = (self.office.layout.cols, self.office.layout.rows);
        scene.width < cols * TILE_SIZE || scene.height * 2 < rows * TILE_SIZE
    }

    fn render(&mut self, terminal: &mut TerminalGuard) -> Result<()> {
        terminal
            .terminal
            .draw(|frame| {
                let area = frame.area();
                if area.width < MIN_TERMINAL_COLS || area.height <= MIN_TERMINAL_ROWS {
                    draw_size_notice(frame.buffer_mut(), area);
                    return;
                }
                let rows = self.agent_rows();
                let panel_height = AgentPanel::height_for(rows.len(), area.height);
                let chunks =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(panel_height)])
                        .split(area);
                let main_area = chunks[0];
                self.render_origin = (main_area.x, main_area.y);
                let panel_area = chunks[1];

                let buf_w = main_area.width;
                let buf_h = main_area.height * 2;
                if buf_w == 0 || buf_h == 0 {
                    return;
                }
                let mut pixel_buf = PixelBuffer::new(buf_w, buf_h);

                let tile_map_flat: Vec<_> =
                    self.office.tile_map.iter().flatten().copied().collect();

                let wh = &self.warehouse;
                let furniture_render: Vec<FurnitureRender> = self
                    .office
                    .furniture
                    .iter()
                    .map(|f| {
                        let tier = produce_tier(&f.furniture_type, wh);
                        FurnitureRender {
                            kind: f.furniture_type.clone(),
                            col: f.col,
                            row: f.row,
                            color: None,
                            is_seat: f.is_seat,
                            tier,
                        }
                    })
                    .collect();

                let character_render: Vec<CharacterRender> = self
                    .office
                    .characters
                    .iter()
                    .filter(|ch| !ch.is_dormant)
                    .map(|ch| {
                        let (direction, anim_type, frame) = ch.sprite_key();
                        CharacterRender {
                            palette: ch.palette,
                            direction,
                            anim_type,
                            frame,
                            pixel_x: ch.pos.0 as i16,
                            pixel_y: ch.pos.1 as i16,
                            bubble: ch.bubble.as_ref().map(|b| BubbleRender {
                                kind: b.kind,
                                timer: b.timer,
                            }),
                            companions: ch
                                .companions
                                .iter()
                                .map(|c| CompanionRender {
                                    kind: c.kind,
                                    offset_x: c.offset.0,
                                    offset_y: c.offset.1,
                                    frame: c.anim_frame,
                                })
                                .collect(),
                        }
                    })
                    .collect();

                let tile_colors: Vec<_> = self
                    .office
                    .layout
                    .tile_colors
                    .as_ref()
                    .map(|tc| {
                        tc.iter()
                            .filter_map(|(key, color)| {
                                parse_tile_key(key).map(|pos| (pos, color.clone()))
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let selected_idx = self
                    .selected
                    .and_then(|id| self.office.characters.iter().position(|c| c.agent_id == id));

                let scene = SceneInput {
                    tile_map: &tile_map_flat,
                    cols: self.office.layout.cols,
                    rows: self.office.layout.rows,
                    furniture: &furniture_render,
                    characters: &character_render,
                    tile_colors: &tile_colors,
                    selected: selected_idx,
                };

                composer::compose_scene(&mut pixel_buf, &scene);
                frame.render_widget(&pixel_buf, main_area);

                // Name labels above each character (rendered as text overlay)
                let term_buf = frame.buffer_mut();
                for ch in self.office.characters.iter().filter(|c| !c.is_dormant) {
                    let agent = self.agents.agents().iter().find(|a| a.id == ch.agent_id);
                    let project_name = agent.map_or("agent", |a| a.identity.directory.as_str());
                    let label = text::truncate_to_width(project_name, LABEL_MAX_WIDTH);

                    // One terminal cell is 1px wide and 2px tall (half-block); the
                    // label sits 4px above the head, centered on the 8px-wide sprite.
                    let center_x = i32::from(main_area.x) + ch.pos.0 as i32 + 4;
                    let label_y = i32::from(main_area.y) + (ch.pos.1 as i32 - 4) / 2;
                    let (pr, pg, pb) = palette_color(ch.palette);
                    let style = Style::default().fg(Color::Rgb(pr, pg, pb));
                    text::draw_centered(term_buf, main_area, center_x, label_y, &label, style);
                }

                let panel = AgentPanel {
                    rows,
                    produce: ProduceCounts {
                        wheat: self.warehouse.count(ProduceType::Wheat),
                        fruit: self.warehouse.count(ProduceType::Fruit),
                        fish: self.warehouse.count(ProduceType::Fish),
                    },
                    too_small: self.scene_is_cropped(main_area),
                };
                frame.render_widget(&panel, panel_area);
            })
            .wrap_err("render failed")?;
        Ok(())
    }
}

/// Pick a companion animal kind based on agent/tool IDs for variety.
/// Draw the one-line notice shown when the terminal cannot hold the farm view.
fn draw_size_notice(buf: &mut Buffer, area: Rect) {
    let notice = format!(
        "terminal too small: need {MIN_TERMINAL_COLS}x{}",
        MIN_TERMINAL_ROWS + 1
    );
    let center = i32::from(area.x) + i32::from(area.width) / 2;
    let row = i32::from(area.y) + i32::from(area.height) / 2;
    text::draw_centered(buf, area, center, row, &notice, Style::default());
}

fn pick_companion_kind(agent_id: usize, tool_id: &str) -> CompanionKind {
    let hash = tool_id
        .bytes()
        .fold(agent_id, |acc, b| acc.wrapping_add(b as usize));
    match hash % 3 {
        0 => CompanionKind::Chicken,
        1 => CompanionKind::Cat,
        _ => CompanionKind::Dog,
    }
}

fn parse_tile_key(key: &str) -> Option<(u16, u16)> {
    let mut parts = key.split(',');
    let col: u16 = parts.next()?.parse().ok()?;
    let row: u16 = parts.next()?.parse().ok()?;
    Some((col, row))
}

/// Map a produce furniture type to its warehouse tier for rendering.
fn produce_tier(furniture_type: &str, wh: &Warehouse) -> u8 {
    match furniture_type {
        "WHEAT_PILE" => wh.tier(ProduceType::Wheat),
        "FRUIT_BASKET" => wh.tier(ProduceType::Fruit),
        "FISH_PILE" => wh.tier(ProduceType::Fish),
        _ => 0,
    }
}

fn palette_color(palette: u8) -> (u8, u8, u8) {
    match palette {
        0 => (60, 100, 220),  // blue
        1 => (200, 50, 50),   // red
        2 => (222, 238, 214), // white/light
        3 => (140, 50, 200),  // purple
        4 => (230, 140, 30),  // orange
        5 => (30, 180, 180),  // teal
        _ => (180, 180, 180),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use crate::layout::persistence::default_layout;

    impl App {
        /// App watching `dir`, with no config directory so tests never touch `$HOME`.
        fn for_test(dir: PathBuf) -> Self {
            Self {
                office: OfficeState::from_layout(default_layout()),
                agents: AgentRegistry::new(),
                scanner: DirectoryScanner::new(vec![dir]).unwrap(),
                timers: TimerManager::new(),
                // Due on every tick so tests do not wait for the production cadence.
                reader_poll: PollInterval::new(0),
                warehouse: Warehouse::default(),
                config_dir: None,
                selected: None,
                running: true,
                render_origin: (0, 0),
            }
        }
    }

    fn write_transcript(project: &std::path::Path, name: &str) -> PathBuf {
        let path = project.join(format!("{name}.jsonl"));
        std::fs::write(&path, "{\"type\":\"user\"}\n").unwrap();
        path
    }

    fn tick_until(app: &mut App, limit: Duration, done: impl Fn(&App) -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < limit {
            app.on_tick(0.016);
            if done(app) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn append_line(path: &std::path::Path, line: &str) {
        let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        writeln!(file, "{line}").unwrap();
    }

    fn companion_count(app: &App, agent_id: usize) -> usize {
        app.office
            .character_by_agent(agent_id)
            .map_or(0, |ch| ch.companions.len())
    }

    #[test]
    fn background_agent_companion_lives_until_its_final_result() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let path = write_transcript(&project, "s1");
        let mut app = App::for_test(dir.path().to_path_buf());
        assert!(app.adopt_session(path.clone(), "proj".into(), "s1".into()));
        let id = app.agents.agents()[0].id;

        append_line(
            &path,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"bg1","name":"Agent","input":{"description":"Search"}}]}}"#,
        );
        append_line(
            &path,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"bg1","content":[{"type":"text","text":"Async agent launched successfully.\nagentId: a1"}]}]}}"#,
        );
        assert!(tick_until(
            &mut app,
            Duration::from_secs(2),
            |a| companion_count(a, id) == 1
        ));

        append_line(
            &path,
            r#"{"type":"system","subtype":"turn_duration","durationMs":5}"#,
        );
        app.on_tick(0.016);
        std::thread::sleep(Duration::from_millis(50));
        app.on_tick(0.016);
        assert_eq!(
            companion_count(&app, id),
            1,
            "turn end keeps the background companion"
        );

        append_line(
            &path,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"bg1","content":"final report"}]}}"#,
        );
        assert!(tick_until(
            &mut app,
            Duration::from_secs(2),
            |a| companion_count(a, id) == 0
        ));
    }

    #[test]
    fn tool_result_clears_the_permission_state_and_bubble() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let path = write_transcript(&project, "s1");
        let mut app = App::for_test(dir.path().to_path_buf());
        assert!(app.adopt_session(path.clone(), "proj".into(), "s1".into()));
        let id = app.agents.agents()[0].id;

        append_line(
            &path,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"rm x"}}]}}"#,
        );
        assert!(tick_until(&mut app, Duration::from_secs(2), |a| {
            a.agents
                .get(id)
                .is_some_and(|ag| ag.status == AgentStatus::Active)
        }));
        app.apply_permission(id);
        assert_eq!(app.agents.get(id).unwrap().status, AgentStatus::Permission);
        assert!(app.office.character_by_agent(id).unwrap().bubble.is_some());

        append_line(
            &path,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"ok"}]}}"#,
        );
        assert!(tick_until(&mut app, Duration::from_secs(2), |a| {
            a.agents
                .get(id)
                .is_some_and(|ag| ag.status != AgentStatus::Permission)
        }));
        assert_eq!(app.agents.get(id).unwrap().status, AgentStatus::Active);
        assert!(app.office.character_by_agent(id).unwrap().bubble.is_none());
    }

    #[test]
    fn adopting_a_transcript_with_a_running_tool_shows_the_agent_working() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let path = write_transcript(&project, "s1");
        append_line(
            &path,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"cargo test"}}]}}"#,
        );
        let mut app = App::for_test(dir.path().to_path_buf());

        assert!(app.adopt_session(path, "proj".into(), "s1".into()));

        let id = app.agents.agents()[0].id;
        let agent = app.agents.get(id).unwrap();
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(agent.summary(), "Bash: cargo test");
        let ch = app.office.character_by_agent(id).unwrap();
        assert!(ch.is_active);
        assert!(ch.bubble.is_none());
        assert!(app.timers.permission_pending(id));
    }

    #[test]
    fn refused_transcript_is_adopted_on_a_later_write_once_a_slot_frees() {
        let root = tempfile::tempdir().unwrap();
        // The watcher reports canonical paths; temp dirs may be symlinked.
        let root_path = root.path().canonicalize().unwrap();
        let project = root_path.join("-Users-u-proj");
        std::fs::create_dir(&project).unwrap();
        let mut app = App::for_test(root_path);
        let mut first = Vec::new();
        for i in 0..MAX_AGENTS {
            let path = write_transcript(&project, &format!("s{i}"));
            assert!(app.adopt_session(path.clone(), "-Users-u-proj".into(), format!("s{i}")));
            first.push(path);
        }
        let ninth = write_transcript(&project, "ninth");
        // The scanner sees the ninth transcript while the limit is reached.
        tick_until(&mut app, Duration::from_secs(2), |_| false);
        assert!(!app.agents.has_path(&ninth));

        app.remove_session(&first[0]);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&ninth)
            .unwrap()
            .write_all(b"{\"type\":\"user\"}\n")
            .unwrap();
        let adopted = tick_until(&mut app, Duration::from_secs(5), |a| {
            a.agents.has_path(&ninth)
        });
        assert!(adopted, "refused transcript was never retried");
    }

    #[test]
    fn ctrl_c_quits() {
        let root = tempfile::tempdir().unwrap();
        let mut app = App::for_test(root.path().to_owned());
        app.update(Action::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(!app.running);
    }

    #[test]
    fn plain_c_does_not_quit() {
        let root = tempfile::tempdir().unwrap();
        let mut app = App::for_test(root.path().to_owned());
        app.update(Action::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
        )));
        assert!(app.running);
    }

    #[test]
    fn missing_explicit_watch_dir_is_an_error_naming_the_path() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("no-such-dir");
        let args = Args {
            watch_dir: vec![missing],
            layout: None,
        };

        let err = App::new(args).err().expect("startup must fail");

        assert!(format!("{err:#}").contains("no-such-dir"));
    }

    #[test]
    fn explicit_watch_dir_that_is_a_file_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("plain-file");
        std::fs::write(&file, "x").unwrap();
        let args = Args {
            watch_dir: vec![file],
            layout: None,
        };

        let err = App::new(args).err().expect("startup must fail");

        assert!(format!("{err:#}").contains("plain-file"));
    }

    #[test]
    fn dormant_agents_do_not_count_toward_the_limit() {
        let root = tempfile::tempdir().unwrap();
        // The watcher reports canonical paths; temp dirs may be symlinked.
        let root_path = root.path().canonicalize().unwrap();
        let project = root_path.join("-Users-u-proj");
        std::fs::create_dir(&project).unwrap();
        let mut app = App::for_test(root_path);
        for i in 0..MAX_AGENTS {
            let path = write_transcript(&project, &format!("s{i}"));
            assert!(app.adopt_session(path, "-Users-u-proj".into(), format!("s{i}")));
        }
        let ids: Vec<usize> = app.agents.agents().iter().map(|a| a.id).collect();
        for id in ids {
            app.agents.get_mut(id).unwrap().status = AgentStatus::Dormant;
        }
        let extra = write_transcript(&project, "extra");
        assert!(app.adopt_session(extra, "-Users-u-proj".into(), "extra".into()));
    }

    fn adopt_many(app: &mut App, dir: &std::path::Path, count: usize) -> Vec<usize> {
        let project = dir.join("proj");
        std::fs::create_dir_all(&project).unwrap();
        (0..count)
            .map(|i| {
                let path = write_transcript(&project, &format!("s{i}"));
                assert!(app.adopt_session(path, "proj".into(), format!("s{i}")));
                app.agents.agents().last().unwrap().id
            })
            .collect()
    }

    fn rest(app: &mut App, id: usize) {
        app.agents.get_mut(id).unwrap().status = AgentStatus::Dormant;
        app.start_dormant_walk(id);
    }

    #[test]
    fn arrow_keys_skip_resting_agents() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::for_test(dir.path().to_path_buf());
        let ids = adopt_many(&mut app, dir.path(), 3);
        rest(&mut app, ids[1]);
        app.selected = Some(ids[0]);

        app.cycle_selection(true);
        assert_eq!(app.selected, Some(ids[2]));

        app.cycle_selection(false);
        assert_eq!(app.selected, Some(ids[0]));
    }

    #[test]
    fn arrow_keys_select_nothing_when_every_agent_is_resting() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::for_test(dir.path().to_path_buf());
        let ids = adopt_many(&mut app, dir.path(), 3);
        for id in ids {
            rest(&mut app, id);
        }

        app.cycle_selection(true);
        assert_eq!(app.selected, None);
    }

    #[test]
    fn clicking_a_resting_agent_does_not_select_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::for_test(dir.path().to_path_buf());
        let ids = adopt_many(&mut app, dir.path(), 1);
        rest(&mut app, ids[0]);
        let (tx, ty) = app
            .office
            .character_by_agent(ids[0])
            .unwrap()
            .current_tile();

        app.on_mouse_click(tx * TILE_SIZE, ty * TILE_SIZE / 2);

        assert_eq!(app.selected, None);
    }

    #[test]
    fn clicking_empty_floor_deselects() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::for_test(dir.path().to_path_buf());
        let ids = adopt_many(&mut app, dir.path(), 3);
        app.selected = Some(ids[0]);

        app.on_mouse_click(u16::MAX, u16::MAX);

        assert_eq!(app.selected, None);
    }
}
