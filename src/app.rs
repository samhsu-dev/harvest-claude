use std::collections::VecDeque;

use color_eyre::eyre::{Result, WrapErr};
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::{Constraint, Layout};

use crate::action::Action;
use crate::cli::Args;
use crate::constants::{MAX_AGENTS, MAX_DELTA_TIME};
use crate::engine::pathfind;
use crate::engine::state::OfficeState;
use crate::event::EventHandler;
use crate::layout::persistence;
use crate::render::buffer::PixelBuffer;
use crate::render::composer::{
    self, BubbleRender, CharacterRender, CompanionRender, FurnitureRender, SceneInput,
};
use crate::tui::TerminalGuard;
use crate::types::{AgentEvent, AgentStatus, CompanionKind};
use crate::ui::input;
use crate::ui::status_bar::{SelectedInfo, StatusBar};
use crate::watcher::registry::AgentRegistry;
use crate::watcher::scanner::{DirectoryScanner, ScanEvent};
use crate::watcher::timer::{TimerEvent, TimerManager};

/// Application orchestrator. Receives `Action`, updates state, renders.
pub struct App {
    office: OfficeState,
    agents: AgentRegistry,
    scanner: DirectoryScanner,
    timers: TimerManager,
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

        Ok(Self {
            office,
            agents,
            scanner,
            timers: TimerManager::new(),
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
                if self.office.characters.len() < MAX_AGENTS
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
                    if self.office.characters.len() < MAX_AGENTS
                        && let Ok(id) = self.agents.add_agent(session_id, path, project_name)
                    {
                        let (palette, hue_shift) = self.agents.assign_palette();
                        self.office.add_character(id, palette, hue_shift);
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
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    let bg = agent.background_tool_ids.clone();
                    agent.active_tools.retain(|k, _| bg.contains(k));
                    agent.had_tools_in_turn = false;
                    agent.status = AgentStatus::Waiting;
                }
                self.timers.cancel_all(agent_id);
                if let Some(ch) = self.office.character_by_agent_mut(agent_id) {
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
            AgentEvent::SubAgentSpawn { .. } => {}
            AgentEvent::SubAgentToolStart { .. } => {
                // Short-term sub-agent activity: activate nearby crop plots
                self.activate_nearby_crops(agent_id);
            }
            AgentEvent::SubAgentToolDone { .. } => {}
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

        // Select the clicked character
        self.selected = Some(agent_id);
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

                let furniture_render: Vec<FurnitureRender> = self
                    .office
                    .furniture
                    .iter()
                    .map(|f| FurnitureRender {
                        kind: f.furniture_type.clone(),
                        col: f.col,
                        row: f.row,
                        color: None,
                        is_seat: f.is_seat,
                    })
                    .collect();

                let character_render: Vec<CharacterRender> = self
                    .office
                    .characters
                    .iter()
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

                // Status bar
                let palette_dots: Vec<(u8, u8, u8)> = self
                    .office
                    .characters
                    .iter()
                    .map(|ch| palette_color(ch.palette))
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

                let status = StatusBar {
                    agent_count: self.office.characters.len(),
                    palette_dots,
                    selected_info,
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

fn palette_color(palette: u8) -> (u8, u8, u8) {
    match palette {
        0 => (70, 130, 180),
        1 => (178, 102, 76),
        2 => (106, 168, 79),
        3 => (180, 95, 160),
        4 => (200, 160, 60),
        5 => (100, 180, 180),
        _ => (180, 180, 180),
    }
}
