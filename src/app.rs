use std::collections::VecDeque;

use color_eyre::eyre::{Result, WrapErr};
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Style};

use crate::action::Action;
use crate::cli::Args;
use crate::constants::{MAX_AGENTS, MAX_DELTA_TIME};
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
use crate::types::{AgentEvent, AgentStatus, AnimType, CompanionKind, ProduceType};
use crate::ui::input;
use crate::ui::status_bar::{AgentSummary, SelectedInfo, StatusBar};
use crate::watcher::focus;
use crate::watcher::registry::AgentRegistry;
use crate::watcher::scanner::{DirectoryScanner, ScanEvent};
use crate::watcher::timer::{TimerEvent, TimerManager};

/// Application orchestrator. Receives `Action`, updates state, renders.
pub struct App {
    office: OfficeState,
    agents: AgentRegistry,
    scanner: DirectoryScanner,
    timers: TimerManager,
    warehouse: Warehouse,
    config_dir: Option<std::path::PathBuf>,
    selected: Option<usize>,
    running: bool,
    /// Render area origin in terminal cells (for mouse offset).
    render_origin: (u16, u16),
}

impl App {
    /// Create a new App from CLI arguments.
    pub fn new(args: Args) -> Result<Self> {
        let layout = if let Some(path) = &args.layout {
            persistence::load_layout(path).wrap_err("failed to load layout")?
        } else {
            persistence::load_or_default().wrap_err("failed to load default layout")?
        };

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

        let config_dir = persistence::config_dir().ok();
        let wh = config_dir
            .as_ref()
            .map(|d| warehouse::load_warehouse(d))
            .unwrap_or_default();

        Ok(Self {
            office,
            agents,
            scanner,
            timers: TimerManager::new(),
            warehouse: wh,
            config_dir,
            selected: None,
            running: true,
            render_origin: (0, 0),
        })
    }

    /// Run the main event loop.
    pub fn run(&mut self, terminal: &mut TerminalGuard) -> Result<()> {
        let events = EventHandler::new();

        // Discover existing sessions (capped at MAX_AGENTS)
        let initial = self
            .scanner
            .initial_scan()
            .wrap_err("failed initial scan")?;
        for path in initial {
            if self.office.characters.len() >= MAX_AGENTS {
                break;
            }
            if let Some((project_name, session_id)) = extract_session_info(&path)
                && let Ok(id) = self.agents.add_agent(session_id, path, project_name)
            {
                let (palette, hue_shift) = self.agents.assign_palette();
                self.office.add_character(id, palette, hue_shift);
            }
        }

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

    /// Match action, mutate state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Tick(dt) => self.on_tick(dt),
            Action::Render => {}
            Action::Resize(_, _) => {}
            Action::Key(key) => self.on_key(key.code),
            Action::Mouse(mouse) => {
                if let MouseEventKind::Down(_) = mouse.kind {
                    self.on_mouse_click(mouse.column, mouse.row);
                }
            }
            Action::Quit => self.running = false,
            Action::AgentDiscovered {
                path,
                project,
                session_id,
            } => {
                if !self.agents.has_path(&path)
                    && self.office.characters.len() < MAX_AGENTS
                    && let Ok(id) = self.agents.add_agent(session_id, path, project)
                {
                    let (palette, hue_shift) = self.agents.assign_palette();
                    self.office.add_character(id, palette, hue_shift);
                }
            }
            Action::AgentGone { path } => {
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
            Action::AgentEvent { agent_id, event } => {
                self.handle_agent_event(agent_id, event);
            }
            Action::PermissionTimeout { agent_id } => {
                self.apply_permission(agent_id);
            }
            Action::TextIdleTimeout { agent_id } => {
                self.apply_waiting(agent_id);
            }
            Action::ToolDoneReady {
                agent_id,
                ref tool_id,
            } => {
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    agent.active_tools.remove(tool_id);
                    if agent.active_tools.is_empty() && !agent.had_tools_in_turn {
                        agent.status = AgentStatus::Idle;
                    }
                }
            }
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
                    if self.office.characters.len() < MAX_AGENTS
                        && let Ok(id) = self.agents.add_agent(session_id, path, project_name)
                    {
                        let (palette, hue_shift) = self.agents.assign_palette();
                        self.office.add_character(id, palette, hue_shift);
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
                ScanEvent::SessionGone { path } => {
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
            }
        }

        // Poll JSONL readers
        for (agent_id, events) in self.agents.poll_all() {
            self.timers.cancel_text_idle(agent_id);
            for event in events {
                self.handle_agent_event(agent_id, event);
            }
        }

        // Check expired timers
        for timer_event in self.timers.check_expired() {
            match timer_event {
                TimerEvent::PermissionTimeout { agent_id } => self.apply_permission(agent_id),
                TimerEvent::TextIdleTimeout { agent_id } => self.apply_waiting(agent_id),
                TimerEvent::ToolDoneReady { agent_id, tool_id } => {
                    if let Some(agent) = self.agents.get_mut(agent_id) {
                        agent.active_tools.remove(&tool_id);
                    }
                }
            }
        }

        // Check for delivery arrivals (characters that reached the barn)
        self.collect_deliveries();

        self.office.update(dt);
    }

    fn handle_agent_event(&mut self, agent_id: usize, event: AgentEvent) {
        match event {
            AgentEvent::ToolStart { tool_id, tool_name } => {
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    agent.active_tools.insert(tool_id, tool_name.clone());
                    agent.had_tools_in_turn = true;
                    agent.status = AgentStatus::Active;
                }
                if crate::watcher::parser::is_non_exempt_tool(&tool_name) {
                    self.timers.start_permission(agent_id);
                }
                // Wake dormant characters on new tool activity
                if let Some(ch) = self.office.character_by_agent_mut(agent_id)
                    && ch.is_dormant
                {
                    ch.wake_from_dormant();
                }
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
                    ch.set_active(&tool_name, seat_pos, VecDeque::from(path));
                }
            }
            AgentEvent::ToolDone { ref tool_id } => {
                self.timers.cancel_permission(agent_id);
                // Remove companion if this was a background agent
                let is_bg = self
                    .agents
                    .get(agent_id)
                    .is_some_and(|a| a.background_tool_ids.contains(tool_id));
                if is_bg && let Some(ch) = self.office.character_by_agent_mut(agent_id) {
                    ch.remove_companion(tool_id);
                }
                self.timers.delay_tool_done(agent_id, tool_id.clone());
            }
            AgentEvent::TurnEnd => {
                let had_tools = self
                    .agents
                    .get(agent_id)
                    .is_some_and(|a| a.had_tools_in_turn);

                if let Some(agent) = self.agents.get_mut(agent_id) {
                    let bg = agent.background_tool_ids.clone();
                    agent.active_tools.retain(|k, _| bg.contains(k));
                    agent.had_tools_in_turn = false;
                    agent.status = AgentStatus::Waiting;
                }
                self.timers.cancel_all(agent_id);

                // Determine produce from work animation, then deliver to barn
                let delivered = if had_tools {
                    self.start_barn_delivery(agent_id)
                } else {
                    false
                };

                if !delivered && let Some(ch) = self.office.character_by_agent_mut(agent_id) {
                    ch.set_idle();
                    ch.set_waiting();
                }
            }
            AgentEvent::TextOnly => {
                if let Some(agent) = self.agents.get_mut(agent_id)
                    && !agent.had_tools_in_turn
                {
                    self.timers.start_text_idle(agent_id);
                }
            }
            AgentEvent::BashProgress { .. } => {
                self.timers.restart_permission(agent_id);
            }
            AgentEvent::BackgroundAgentDetected { ref tool_id } => {
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    agent.background_tool_ids.insert(tool_id.clone());
                }
                // Spawn a companion animal for this background agent
                let kind = pick_companion_kind(agent_id, tool_id);
                if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
                    ch.add_companion(tool_id.clone(), kind);
                }
            }
            AgentEvent::SubAgentSpawn { ref parent_tool_id } => {
                // Sub-agents appear as companion animals on the parent character
                let kind = pick_companion_kind(agent_id, parent_tool_id);
                if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
                    ch.add_companion(parent_tool_id.clone(), kind);
                }
            }
            AgentEvent::SubAgentToolStart { .. } => {
                // Short-term sub-agent activity: activate nearby crop plots
                self.activate_nearby_crops(agent_id);
            }
            AgentEvent::SubAgentToolDone {
                ref parent_tool_id, ..
            } => {
                // Remove companion when sub-agent tool finishes
                if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
                    ch.remove_companion(parent_tool_id);
                }
            }
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

    fn on_key(&mut self, code: KeyCode) {
        match code {
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
            KeyCode::Left => self.cycle_selection(false),
            KeyCode::Right => self.cycle_selection(true),
            _ => {}
        }
    }

    fn cycle_selection(&mut self, forward: bool) {
        let count = self.office.characters.len();
        if count == 0 {
            self.selected = None;
            return;
        }

        let current_idx = self
            .selected
            .and_then(|id| self.office.characters.iter().position(|c| c.agent_id == id));

        let next_idx = match current_idx {
            Some(idx) if forward => (idx + 1) % count,
            Some(idx) => (idx + count - 1) % count,
            None => 0,
        };

        self.selected = Some(self.office.characters[next_idx].agent_id);
    }

    fn on_mouse_click(&mut self, col: u16, row: u16) {
        let (ox, oy) = self.render_origin;
        let Some(agent_id) = input::hit_test_character(&self.office, col, row, ox, oy) else {
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
            if let Some(agent) = self.agents.get(agent_id) {
                focus::focus_agent_window(&agent.jsonl_path);
            }
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

    /// Activate nearby crop plots when a sub-agent tool runs (growth effect).
    fn activate_nearby_crops(&mut self, agent_id: usize) {
        let tile = match self.office.character_by_agent(agent_id) {
            Some(ch) => ch.current_tile(),
            None => return,
        };
        // Find crop plots within 3 tiles and switch to _ON
        for furn in &mut self.office.furniture {
            if furn.furniture_type != "CROP_PLOT" {
                continue;
            }
            let dist = (furn.col as i32 - tile.0 as i32).unsigned_abs()
                + (furn.row as i32 - tile.1 as i32).unsigned_abs();
            if dist <= 3 {
                furn.furniture_type = "CROP_PLOT_ON".to_owned();
            }
        }
    }

    /// Compose scene + status bar.
    fn render(&mut self, terminal: &mut TerminalGuard) -> Result<()> {
        terminal
            .terminal
            .draw(|frame| {
                let area = frame.area();
                let chunks =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
                let main_area = chunks[0];
                self.render_origin = (main_area.x, main_area.y);
                let status_area = chunks[1];

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
                    let project_name = agent.map(|a| a.project_name.as_str()).unwrap_or("agent");
                    let label = short_label(project_name);

                    // Character pixel position → terminal cell position
                    // Each terminal cell = 1px wide, 2px tall (half-block)
                    let char_px_x = ch.pos.0 as i16;
                    let char_px_y = ch.pos.1 as i16;

                    // Label positioned above the character, centered
                    let label_len = label.len() as i16;
                    let label_x = main_area.x as i16 + char_px_x + 4 - label_len / 2;
                    let label_y = main_area.y as i16 + (char_px_y - 4) / 2; // -4px above head

                    if label_y < main_area.y as i16
                        || label_y >= (main_area.y + main_area.height) as i16
                    {
                        continue;
                    }

                    let (pr, pg, pb) = palette_color(ch.palette);
                    let style = Style::default().fg(Color::Rgb(pr, pg, pb));

                    for (i, ch_byte) in label.bytes().enumerate() {
                        let x = label_x + i as i16;
                        if x < main_area.x as i16 || x >= (main_area.x + main_area.width) as i16 {
                            continue;
                        }
                        if let Some(cell) =
                            term_buf.cell_mut(Position::new(x as u16, label_y as u16))
                        {
                            cell.set_symbol(&String::from(ch_byte as char));
                            cell.set_style(style);
                        }
                    }
                }

                // Status bar
                let agent_summaries: Vec<AgentSummary> = self
                    .office
                    .characters
                    .iter()
                    .filter(|ch| !ch.is_dormant)
                    .map(|ch| {
                        let agent = self.agents.agents().iter().find(|a| a.id == ch.agent_id);
                        AgentSummary {
                            color: palette_color(ch.palette),
                            project_name: agent.map(|a| a.project_name.clone()).unwrap_or_default(),
                            status: agent.map(|a| a.status).unwrap_or(AgentStatus::Idle),
                            tool_name: ch.tool_name.clone(),
                        }
                    })
                    .collect();

                let selected_info = self.selected.and_then(|id| {
                    let agent = self.agents.agents().iter().find(|a| a.id == id)?;
                    let ch = self.office.character_by_agent(id)?;
                    Some(SelectedInfo {
                        project_name: agent.project_name.clone(),
                        session_id: agent.session_id.clone(),
                        status: agent.status,
                        tool_name: ch.tool_name.clone(),
                    })
                });

                let produce = crate::ui::status_bar::ProduceCounts {
                    wheat: self.warehouse.count(ProduceType::Wheat),
                    fruit: self.warehouse.count(ProduceType::Fruit),
                    fish: self.warehouse.count(ProduceType::Fish),
                };

                let status = StatusBar {
                    agents: agent_summaries,
                    selected_info,
                    produce,
                };

                frame.render_widget(&status, status_area);
            })
            .wrap_err("render failed")?;
        Ok(())
    }
}

/// Pick a companion animal kind based on agent/tool IDs for variety.
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

fn extract_session_info(path: &std::path::Path) -> Option<(String, String)> {
    let session_id = path.file_stem()?.to_str()?.to_owned();
    let project_name = path.parent()?.file_name()?.to_str()?.to_owned();
    Some((project_name, session_id))
}

fn parse_tile_key(key: &str) -> Option<(u16, u16)> {
    let mut parts = key.split(',');
    let col: u16 = parts.next()?.parse().ok()?;
    let row: u16 = parts.next()?.parse().ok()?;
    Some((col, row))
}

/// Extract a short display label from a project name/hash.
///
/// Converts `-Users-foo-Projects-bar` → `bar`, or truncates to 8 chars.
fn short_label(project_name: &str) -> String {
    // Project names are often hashes like "-Users-foo-Projects-bar"
    // Extract the last meaningful segment
    let cleaned = project_name.trim_start_matches('-');
    let segments: Vec<&str> = cleaned.split('-').collect();

    // Find the segment after "Projects" if it exists
    let label = segments
        .iter()
        .position(|&s| s.eq_ignore_ascii_case("Projects"))
        .and_then(|idx| segments.get(idx + 1))
        .copied()
        .unwrap_or_else(|| segments.last().copied().unwrap_or("agent"));

    if label.len() > 8 {
        format!("{}…", &label[..7])
    } else {
        label.to_owned()
    }
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
