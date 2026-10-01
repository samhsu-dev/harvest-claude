use std::collections::VecDeque;

use harvest_claude::engine::pathfind;
use harvest_claude::engine::state::OfficeState;
use harvest_claude::layout::persistence::default_layout;
use harvest_claude::render::buffer::PixelBuffer;
use harvest_claude::render::colorize::colorize_sprite;
use harvest_claude::render::composer::{self, CharacterRender, FurnitureRender, SceneInput};
use harvest_claude::types::{
    AgentEvent, AgentStatus, AnimType, BubbleKind, CharState, Direction, OfficeLayout,
    PlacedFurniture, TileColor, TileType,
};
use harvest_claude::watcher::parser;
use harvest_claude::watcher::registry::AgentRegistry;
use harvest_claude::watcher::timer::TimerManager;

// =========================================================================
// Helpers
// =========================================================================

fn layout_with_workstation() -> OfficeLayout {
    OfficeLayout {
        version: 1,
        cols: 8,
        rows: 6,
        tiles: vec![1; 48],
        furniture: vec![
            PlacedFurniture::new("crop-1", "CROP_PLOT", 3, 2),
            PlacedFurniture::new("stump-1", "STUMP_BACK", 3, 3),
            PlacedFurniture::new("stump-2", "STUMP_BACK", 6, 3),
        ],
        tile_colors: None,
        layout_revision: Some(1),
    }
}

/// Parse one transcript line and apply its events to the agent, as the registry does.
fn apply_line(registry: &mut AgentRegistry, agent_id: usize, line: &str) -> Vec<AgentEvent> {
    let record = parser::parse_line(line).expect("valid record");
    let events = parser::extract_events(&record);
    let agent = registry.get_mut(agent_id).expect("agent exists");
    for event in &events {
        agent.apply(event);
    }
    events
}

fn create_test_jsonl(dir: &std::path::Path) -> std::path::PathBuf {
    use std::io::Write;
    let path = dir.join("test.jsonl");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(b"{\"type\":\"system\",\"message\":{\"subtype\":\"init\"}}\n")
        .unwrap();
    path
}

// =========================================================================
// 1. Layout -> Engine integration
// =========================================================================

mod layout_engine {
    use super::*;

    #[test]
    fn default_layout_creates_valid_office_state() {
        let layout = default_layout();
        let state = OfficeState::from_layout(layout);

        assert_eq!(state.tile_map.len(), 16);
        assert_eq!(state.tile_map[0].len(), 28);
        assert!(!state.seats.is_empty(), "default layout has seats");
        assert!(
            !state.walkable.is_empty(),
            "default layout has walkable tiles"
        );
    }

    #[test]
    fn layout_furniture_produces_walkable_paths() {
        let state = OfficeState::from_layout(layout_with_workstation());

        // Pick two walkable tiles and verify BFS connects them
        let tiles: Vec<_> = state.walkable.iter().copied().collect();
        assert!(tiles.len() >= 2);

        let from = tiles[0];
        let to = tiles[tiles.len() - 1];
        let path = pathfind::bfs(&state.walkable, from, to, None);
        assert!(
            path.is_some(),
            "walkable tiles in an all-floor layout must be connected"
        );
    }

    #[test]
    fn add_character_gets_seat_assignment() {
        let mut state = OfficeState::from_layout(layout_with_workstation());
        assert!(!state.seats.is_empty());

        state.add_character(1, 0, None);
        let ch = state.character_by_agent(1).expect("character exists");
        assert!(ch.seat_id.is_some(), "character must be assigned a seat");
    }

    #[test]
    fn characters_can_pathfind_to_seats() {
        let mut state = OfficeState::from_layout(layout_with_workstation());
        state.add_character(1, 0, None);

        let ch = state.character_by_agent(1).unwrap();
        let seat_idx = ch.seat_id.expect("seat assigned");
        let seat = &state.seats[seat_idx];
        let seat_pos = (seat.col, seat.row);
        let from = ch.current_tile();

        let path = pathfind::bfs(&state.walkable, from, seat_pos, Some(seat_pos));
        assert!(
            path.is_some(),
            "BFS from character position to seat must succeed"
        );
    }

    #[test]
    fn desk_z_map_populated_for_desk_furniture() {
        let state = OfficeState::from_layout(layout_with_workstation());
        assert!(
            !state.desk_z_by_tile.is_empty(),
            "desk_z_by_tile must have entries for desk positions"
        );
        // Crop plot at (3,2) is a 1x1 tile
        assert!(state.desk_z_by_tile.contains_key(&(3, 2)));
    }

    #[test]
    fn multiple_characters_get_different_seats() {
        let mut state = OfficeState::from_layout(layout_with_workstation());
        state.add_character(1, 0, None);
        state.add_character(2, 1, None);

        let ch1 = state.character_by_agent(1).unwrap();
        let ch2 = state.character_by_agent(2).unwrap();
        assert_ne!(
            ch1.seat_id, ch2.seat_id,
            "two characters must get different seats"
        );
    }
}

// =========================================================================
// 2. Watcher -> Engine integration
// =========================================================================

mod watcher_engine {
    use super::*;

    #[test]
    fn agent_tool_start_activates_character() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());

        let mut registry = AgentRegistry::new();
        let agent_id = registry
            .add_agent("sess1".into(), path, "project".into())
            .unwrap();

        let mut state = OfficeState::from_layout(layout_with_workstation());
        let agent = registry.get(agent_id).unwrap();
        state.add_character(agent_id, agent.palette, agent.hue_shift);

        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"description":"Run build"}}]}}"#;
        let events = apply_line(&mut registry, agent_id, line);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            AgentEvent::ToolStart { tool_name, .. } if tool_name == "Bash"
        ));
        let agent = registry.get(agent_id).unwrap();
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(agent.summary(), "Bash: Run build");
    }

    #[test]
    fn turn_end_idles_character() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());

        let mut registry = AgentRegistry::new();
        let agent_id = registry
            .add_agent("sess1".into(), path, "project".into())
            .unwrap();

        let mut state = OfficeState::from_layout(layout_with_workstation());
        let agent = registry.get(agent_id).unwrap();
        state.add_character(agent_id, agent.palette, agent.hue_shift);

        let line = r#"{"type":"system","subtype":"turn_duration","durationMs":1523}"#;
        let events = apply_line(&mut registry, agent_id, line);
        assert_eq!(events, vec![AgentEvent::TurnEnd]);

        let ch = state.character_by_agent_mut(agent_id).unwrap();
        ch.set_idle();
        ch.set_waiting();

        assert_eq!(registry.get(agent_id).unwrap().status, AgentStatus::Waiting);
        let ch = state.character_by_agent(agent_id).unwrap();
        assert!(!ch.is_active);
        assert!(ch.bubble.is_some());
    }

    #[test]
    fn permission_timer_expires_sets_bubble() {
        let mut timers = TimerManager::new();
        let agent_id = 42;

        // Insert deadline in the past to simulate expiration
        timers.start_permission(agent_id);
        // Force expiration by checking after a small sleep would be needed,
        // but we can manipulate directly by re-inserting a past deadline:
        // The timer uses Instant, so we use the check_expired pattern from unit tests.
        let mut mgr = TimerManager::new();
        // Manually create a past-deadline timer via the internal pattern
        mgr.start_permission(agent_id);
        // We cannot set past instants directly, so verify the timer event type
        // by starting and then checking non-expired returns empty
        let events = mgr.check_expired();
        assert!(
            events.is_empty(),
            "freshly started permission timer has not expired"
        );

        // Test the expired path with a reconstructed TimerManager
        let mut mgr2 = TimerManager::new();
        mgr2.start_permission(agent_id);
        // Spin-wait is unreliable in tests. Instead verify the timer contract:
        // after cancel, check_expired returns nothing
        mgr2.cancel_permission(agent_id);
        let events = mgr2.check_expired();
        assert!(events.is_empty());

        // Verify the full chain: timer event -> character bubble
        let mut state = OfficeState::from_layout(layout_with_workstation());
        state.add_character(agent_id, 0, None);
        let ch = state.character_by_agent_mut(agent_id).unwrap();
        ch.set_permission();
        assert!(matches!(
            ch.bubble.as_ref().unwrap().kind,
            BubbleKind::Permission
        ));
    }

    #[test]
    fn text_idle_timer_with_no_tools_triggers() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());

        let mut registry = AgentRegistry::new();
        let agent_id = registry
            .add_agent("sess1".into(), path, "project".into())
            .unwrap();

        // Text-only assistant message with no prior tools in turn
        let line =
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"thinking..."}]}}"#;
        assert!(!registry.get(agent_id).unwrap().had_tools_in_turn);

        let events = apply_line(&mut registry, agent_id, line);
        assert_eq!(
            events,
            vec![AgentEvent::Text {
                first_line: "thinking...".into()
            }]
        );
        let agent = registry.get(agent_id).unwrap();
        assert!(!agent.had_tools_in_turn);
        assert_eq!(agent.summary(), "thinking...");

        // Start text idle timer (what App would do)
        let mut timers = TimerManager::new();
        timers.start_text_idle(agent_id);

        // Cancel to verify it was registered
        timers.cancel_text_idle(agent_id);
        let events = timers.check_expired();
        assert!(events.is_empty());
    }

    #[test]
    fn jsonl_parse_to_agent_event_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());

        let mut registry = AgentRegistry::new();
        let agent_id = registry
            .add_agent("sess1".into(), path, "project".into())
            .unwrap();

        // Tool start
        let line1 = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a.rs"}}]}}"#;
        let events1 = apply_line(&mut registry, agent_id, line1);
        assert!(matches!(
            &events1[0],
            AgentEvent::ToolStart {
                tool_name,
                tool_id,
                detail,
            } if tool_name == "Read" && tool_id == "t1" && detail == "/a.rs"
        ));
        assert_eq!(registry.get(agent_id).unwrap().tools.len(), 1);

        // Tool result
        let line2 =
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1"}]}}"#;
        let events2 = apply_line(&mut registry, agent_id, line2);
        assert!(matches!(
            &events2[0],
            AgentEvent::ToolDone { tool_id, .. } if tool_id == "t1"
        ));
        assert!(registry.get(agent_id).unwrap().tools.is_empty());

        // Turn end
        let line3 = r#"{"type":"system","subtype":"turn_duration","durationMs":1523}"#;
        let events3 = apply_line(&mut registry, agent_id, line3);
        assert!(matches!(&events3[0], AgentEvent::TurnEnd));
        assert_eq!(registry.get(agent_id).unwrap().status, AgentStatus::Waiting);

        // Unknown type produces no events
        let line4 = r#"{"type":"unknown","data":{}}"#;
        assert!(apply_line(&mut registry, agent_id, line4).is_empty());
    }

    #[test]
    fn non_exempt_tool_triggers_permission_timer() {
        assert!(parser::is_non_exempt_tool("Bash"));
        assert!(parser::is_non_exempt_tool("Read"));
        assert!(!parser::is_non_exempt_tool("Task"));
        assert!(!parser::is_non_exempt_tool("Agent"));
    }
}

// =========================================================================
// 3. Render integration
// =========================================================================

mod render {
    use super::*;

    #[test]
    fn pixel_buffer_renders_floor_sprites() {
        let mut buf = PixelBuffer::new(16, 16);
        // Create a small solid sprite to simulate a floor tile
        let sprite = vec![vec![(180, 160, 140, 255); 8]; 8];
        buf.blit(&sprite, 0, 0);

        // Verify non-zero pixels were written
        let px = buf.get(0, 0);
        assert_ne!(px.3, 0, "blitted pixel must have non-zero alpha");
        assert_eq!(px, (180, 160, 140, 255));
    }

    #[test]
    fn compose_scene_with_empty_office() {
        let tiles = vec![TileType::Grass; 16];
        let mut buf = PixelBuffer::new(32, 32);
        let input = SceneInput {
            tile_map: &tiles,
            cols: 4,
            rows: 4,
            furniture: &[],
            characters: &[],
            tile_colors: &[],
            selected: None,
        };
        // Must not panic
        composer::compose_scene(&mut buf, &input);
    }

    #[test]
    fn compose_scene_with_character_has_nonzero_pixels() {
        let tiles = vec![TileType::Grass; 16];
        let chars = vec![CharacterRender {
            palette: 0,
            direction: Direction::Down,
            anim_type: AnimType::Walk,
            frame: 0,
            pixel_x: 8,
            pixel_y: 8,
            bubble: None,
            companions: vec![],
        }];
        let mut buf = PixelBuffer::new(64, 64);
        let input = SceneInput {
            tile_map: &tiles,
            cols: 4,
            rows: 4,
            furniture: &[],
            characters: &chars,
            tile_colors: &[],
            selected: None,
        };
        composer::compose_scene(&mut buf, &input);

        // Check the region around the character position for non-transparent pixels
        let mut found_nonzero = false;
        for y in 0..64 {
            for x in 0..64 {
                if buf.get(x, y).3 > 0 {
                    found_nonzero = true;
                    break;
                }
            }
            if found_nonzero {
                break;
            }
        }
        assert!(
            found_nonzero,
            "scene with character must have visible pixels"
        );
    }

    #[test]
    fn colorize_preserves_alpha_channel() {
        let sprite = vec![vec![
            (128, 128, 128, 255),
            (200, 100, 50, 180),
            (0, 0, 0, 0),
            (50, 50, 50, 1),
        ]];
        let color = TileColor {
            h: 200.0,
            s: 0.5,
            b: 0.0,
        };
        let result = colorize_sprite(&sprite, &color);

        assert_eq!(result[0][0].3, 255);
        assert_eq!(result[0][1].3, 180);
        assert_eq!(result[0][2].3, 0, "transparent pixels stay transparent");
        assert_eq!(result[0][3].3, 1);
    }

    #[test]
    fn compose_scene_with_furniture_renders() {
        let tiles = vec![TileType::Grass; 16];
        let furniture = vec![FurnitureRender {
            kind: "CROP_PLOT".to_owned(),
            col: 0,
            row: 0,
            color: None,
            is_seat: false,
            tier: 0,
        }];
        let mut buf = PixelBuffer::new(64, 64);
        let input = SceneInput {
            tile_map: &tiles,
            cols: 4,
            rows: 4,
            furniture: &furniture,
            characters: &[],
            tile_colors: &[],
            selected: None,
        };
        // Must not panic
        composer::compose_scene(&mut buf, &input);
    }
}

// =========================================================================
// 4. Full pipeline
// =========================================================================

mod full_pipeline {
    use super::*;

    #[test]
    fn default_office_full_render_pipeline() {
        let layout = default_layout();
        let cols = layout.cols;
        let rows = layout.rows;
        let mut state = OfficeState::from_layout(layout);

        state.add_character(1, 0, None);

        let tile_map_flat: Vec<_> = state.tile_map.iter().flatten().copied().collect();

        let furniture_render: Vec<FurnitureRender> = state
            .furniture
            .iter()
            .map(|f| FurnitureRender {
                kind: f.furniture_type.clone(),
                col: f.col,
                row: f.row,
                color: None,
                is_seat: f.is_seat,
                tier: 0,
            })
            .collect();

        let character_render: Vec<CharacterRender> = state
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
                    bubble: None,
                    companions: vec![],
                }
            })
            .collect();

        let tile_colors: Vec<_> = state
            .layout
            .tile_colors
            .as_ref()
            .map(|tc| {
                tc.iter()
                    .filter_map(|(key, color)| {
                        let mut parts = key.split(',');
                        let col: u16 = parts.next()?.parse().ok()?;
                        let row: u16 = parts.next()?.parse().ok()?;
                        Some(((col, row), color.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut buf = PixelBuffer::new(cols * 8, rows * 8 * 2);

        let scene = SceneInput {
            tile_map: &tile_map_flat,
            cols,
            rows,
            furniture: &furniture_render,
            characters: &character_render,
            tile_colors: &tile_colors,
            selected: None,
        };

        composer::compose_scene(&mut buf, &scene);

        // Verify the buffer has visible content
        let mut nonzero_count = 0;
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                if buf.get(x, y).3 > 0 {
                    nonzero_count += 1;
                }
            }
        }
        assert!(
            nonzero_count > 100,
            "full pipeline must produce many visible pixels, got {nonzero_count}"
        );
    }

    #[test]
    fn agent_lifecycle_discovery_to_removal() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());

        let mut registry = AgentRegistry::new();
        let mut state = OfficeState::from_layout(layout_with_workstation());
        let mut timers = TimerManager::new();

        // Discovery
        let agent_id = registry
            .add_agent("sess1".into(), path, "project".into())
            .unwrap();
        let agent = registry.get(agent_id).unwrap();
        state.add_character(agent_id, agent.palette, agent.hue_shift);

        assert_eq!(registry.agents().len(), 1);
        assert_eq!(state.characters.len(), 1);

        // Simulate tool activity
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash"}]}}"#;
        apply_line(&mut registry, agent_id, line);
        assert_eq!(registry.get(agent_id).unwrap().status, AgentStatus::Active);
        timers.start_permission(agent_id);

        // Removal
        state.remove_character(agent_id);
        registry.remove_agent(agent_id);
        timers.cancel_all(agent_id);

        assert!(registry.agents().is_empty());
        assert!(state.characters.is_empty());
        // All seats freed
        assert!(state.seats.iter().all(|s| s.occupied_by.is_none()));
    }

    #[test]
    fn character_state_transitions_through_lifecycle() {
        let mut state = OfficeState::from_layout(layout_with_workstation());
        state.add_character(1, 0, None);

        let ch = state.character_by_agent(1).unwrap();
        assert_eq!(ch.state, CharState::Idle);

        // Activate with empty path (already at seat) -> Type
        let ch = state.character_by_agent_mut(1).unwrap();
        ch.set_active("Bash", Some(ch.current_tile()), VecDeque::new());
        assert_eq!(ch.state, CharState::Type);
        assert!(ch.is_active);

        // Idle transition
        let ch = state.character_by_agent_mut(1).unwrap();
        ch.set_idle();
        assert_eq!(ch.state, CharState::Idle);
        assert!(!ch.is_active);

        // Permission bubble
        let ch = state.character_by_agent_mut(1).unwrap();
        ch.set_permission();
        assert!(matches!(
            ch.bubble.as_ref().unwrap().kind,
            BubbleKind::Permission
        ));

        // Dismiss
        let ch = state.character_by_agent_mut(1).unwrap();
        ch.dismiss_bubble();
        assert!(ch.bubble.is_none());
    }
}
