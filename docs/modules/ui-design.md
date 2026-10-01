# UI Module Design

Terminal widgets below the scene and mouse hit-testing. Event sources: `design.md` (EventHandler, App).

## Design Overview

```
ui/
├── mod.rs
├── text.rs         # truncate_to_width (display-width aware)
├── status_bar.rs   # AgentPanel widget (bottom area)
└── input.rs        # cell → tile, character hit test
```

- **Classes**: `AgentRow`, `ProduceCounts`, `AgentPanel`.
- **Relationships**: `AgentPanel` contains `AgentRow` and `ProduceCounts`. `ui` imports `types`, `constants`, `engine` (hit test only).
- **Roles**. Data holders: `AgentRow`, `ProduceCounts`. Widget: `AgentPanel`. Helpers: `truncate_to_width`, `tile_from_cell`, `hit_test_character`.
- **Removed**: `SelectedInfo`, the single selected-agent line; the panel shows every Agent at once.

## Class / Type Specifications

### AgentRow (`status_bar.rs`, data holder)

`AgentRow { color: (u8,u8,u8), identity: String, status: AgentStatus, summary: String, selected: bool }`. Derives `Debug, Clone`. `identity` is `Agent::display_name`; `summary` is `Agent::summary`. The App builds one row per non-Dormant Agent, in Agent id order.

### ProduceCounts (`status_bar.rs`, data holder)

`ProduceCounts { wheat: u32, fruit: u32, fish: u32 }`. Derives `Debug, Clone, Copy, Default`.

### AgentPanel (`status_bar.rs`, widget)

`AgentPanel { rows: Vec<AgentRow>, produce: ProduceCounts, too_small: bool }`. Implements `Widget` for `&AgentPanel`.

Layout, top to bottom, one terminal row each:

| Row | Content |
|-----|---------|
| One per Agent | `● <identity> │ <status> <summary>` with the dot in the Agent colour; the selected row highlighted |
| Last | Produce counts, key hints `q quit  ←→ select  esc deselect  enter focus`, and when `too_small` the hint `enlarge terminal to see the full farm` |

- Each row is padded or cut to the area width. The identity is cut to `NAME_MAX_WIDTH` columns, the summary takes the remaining columns.
- Status text: `Active`, `Idle`, `Waiting`, `Permission`. Permission is drawn in the amber of the permission bubble.
- Rows beyond the available height are dropped from the bottom; the final row is never dropped.
- Panel height: `rows.len() + 1`, capped to leave at least `MIN_TERMINAL_ROWS` rows for the scene.

## Function Specifications

**truncate_to_width(text: &str, max_width: usize) -> String** (`text.rs`) — Longest prefix whose display width is at most `max_width`, measured with `unicode-width`. Appends `…` (width 1) when text was cut, within the limit. Never splits a character; a wide character that does not fit is dropped. Control characters are replaced by a space.

**tile_from_cell(col, row, offset_x, offset_y) -> TilePos** (`input.rs`) — Terminal cell to tile. Half-block aware: `pixel_y = (row - offset_y) * 2`. Saturating subtraction.

**hit_test_character(state, col, row, offset_x, offset_y) -> Option<usize>** (`input.rs`) — Agent id of the character at the clicked tile. The caller discards ids whose Agent is Dormant.

## Input Mapping

Keys are mapped in `App::update`, not here.

| Input | Effect |
|-------|--------|
| `q`, `Esc` with nothing selected, `Ctrl+C` | Quit |
| `Esc` with a selection | Deselect |
| `←` `→` | Previous or next non-Dormant Agent |
| `Enter` | Focus the selected Agent's terminal window |
| Click on a character | Select; also dismiss its bubble |
| Click elsewhere | Deselect |

`Ctrl+C` is detected from `KeyModifiers::CONTROL` with `KeyCode::Char('c')` because raw mode delivers it as a key event.

## Error Handling

Rendering never fails on content: oversized text is truncated, an area narrower than a row is clipped. The App draws one centred notice instead of the panel and scene when the terminal is narrower than `MIN_TERMINAL_COLS` or not taller than `MIN_TERMINAL_ROWS` (the scene rows, the panel needs one more).
