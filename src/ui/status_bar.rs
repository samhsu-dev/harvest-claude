use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::constants::{MIN_TERMINAL_ROWS, NAME_MAX_WIDTH};
use crate::types::AgentStatus;
use crate::ui::text::{fit_to_width, truncate_to_width};

/// Columns reserved for the status word, wide enough for `Permission`.
const STATUS_WIDTH: usize = 10;

/// Background of the panel.
const PANEL_BG: Color = Color::Rgb(30, 30, 30);

/// Background of the selected agent row.
const SELECTED_BG: Color = Color::Rgb(60, 60, 60);

const KEY_HINTS: &str = "q quit  ←→ select  esc deselect";

const TOO_SMALL_HINT: &str = "enlarge terminal to see the full farm";

/// One agent line of the panel.
#[derive(Debug, Clone)]
pub struct AgentRow {
    /// RGB palette dot color.
    pub color: (u8, u8, u8),
    /// Project label, `Agent::display_name`.
    pub identity: String,
    /// Current agent status.
    pub status: AgentStatus,
    /// One-line description of the current work, `Agent::summary`.
    pub summary: String,
    /// Whether the agent is the selected one.
    pub selected: bool,
}

/// Warehouse produce counts.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProduceCounts {
    pub wheat: u32,
    pub fruit: u32,
    pub fish: u32,
}

/// Panel below the scene: one row per agent, then produce counts and key hints.
#[derive(Debug, Clone)]
pub struct AgentPanel {
    /// Agent rows in display order.
    pub rows: Vec<AgentRow>,
    /// Warehouse produce counts.
    pub produce: ProduceCounts,
    /// Whether the terminal is smaller than the farm, so the scene is cropped.
    pub too_small: bool,
}

impl AgentPanel {
    /// Panel height for `row_count` agents in a terminal `total_height` rows tall.
    ///
    /// Leaves at least `MIN_TERMINAL_ROWS` rows for the scene and never drops
    /// the last row.
    pub fn height_for(row_count: usize, total_height: u16) -> u16 {
        let wanted = u16::try_from(row_count + 1).unwrap_or(u16::MAX);
        wanted
            .min(total_height.saturating_sub(MIN_TERMINAL_ROWS))
            .max(1)
    }

    fn agent_line(row: &AgentRow, width: usize) -> Line<'static> {
        let (r, g, b) = row.color;
        let background = if row.selected { SELECTED_BG } else { PANEL_BG };
        let base = Style::default().fg(Color::White).bg(background);
        let (status_text, status_color) = status_label(row.status);
        let prefix = format!(" {} │ ", fit_to_width(&row.identity, NAME_MAX_WIDTH));
        let used = 1 + prefix.width() + STATUS_WIDTH + 1;
        let summary = fit_to_width(&row.summary, width.saturating_sub(used));
        Line::from(vec![
            Span::styled("●", base.fg(Color::Rgb(r, g, b))),
            Span::styled(prefix, base.fg(Color::Cyan)),
            Span::styled(
                fit_to_width(status_text, STATUS_WIDTH),
                base.fg(status_color),
            ),
            Span::styled(format!(" {summary}"), base),
        ])
        .style(if row.selected {
            base.add_modifier(Modifier::BOLD)
        } else {
            base
        })
    }

    fn footer_line(&self, width: usize) -> Line<'static> {
        let produce = format!(
            " W:{} F:{} f:{} ",
            self.produce.wheat, self.produce.fruit, self.produce.fish
        );
        let hint = if self.too_small {
            format!("{TOO_SMALL_HINT}  {KEY_HINTS} ")
        } else {
            format!("{KEY_HINTS} ")
        };
        let room = width.saturating_sub(produce.width());
        let hint = truncate_to_width(&hint, room);
        let gap = " ".repeat(room.saturating_sub(hint.width()));
        let base = Style::default().bg(PANEL_BG);
        Line::from(vec![
            Span::styled(produce, base.fg(Color::Yellow)),
            Span::styled(gap, base),
            Span::styled(hint, base.fg(Color::DarkGray)),
        ])
    }
}

impl Widget for &AgentPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let width = usize::from(area.width);
        buf.set_style(area, Style::default().bg(PANEL_BG));
        let shown = usize::from(area.height) - 1;
        for (offset, row) in self.rows.iter().take(shown).enumerate() {
            let y = area.y + offset as u16;
            AgentPanel::agent_line(row, width).render(Rect::new(area.x, y, area.width, 1), buf);
        }
        let footer = Rect::new(area.x, area.bottom() - 1, area.width, 1);
        self.footer_line(width).render(footer, buf);
    }
}

fn status_label(status: AgentStatus) -> (&'static str, Color) {
    match status {
        AgentStatus::Active => ("Active", Color::Green),
        AgentStatus::Idle => ("Idle", Color::Gray),
        AgentStatus::Waiting => ("Waiting", Color::Yellow),
        AgentStatus::Permission => ("Permission", Color::Rgb(255, 170, 0)),
        AgentStatus::Dormant => ("Resting", Color::DarkGray),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn row(identity: &str, summary: &str, selected: bool) -> AgentRow {
        AgentRow {
            color: (255, 0, 0),
            identity: identity.into(),
            status: AgentStatus::Active,
            summary: summary.into(),
            selected,
        }
    }

    fn panel(rows: Vec<AgentRow>, too_small: bool) -> AgentPanel {
        AgentPanel {
            rows,
            produce: ProduceCounts::default(),
            too_small,
        }
    }

    // Row text; the blank cell that follows a wide glyph is skipped.
    fn text_of(buf: &Buffer, y: u16) -> String {
        let mut text = String::new();
        let mut skip = 0;
        for x in 0..buf.area.width {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let symbol = buf[(x, y)].symbol();
            skip = symbol.width().saturating_sub(1);
            text.push_str(symbol);
        }
        text
    }

    fn render(panel: &AgentPanel, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        buf
    }

    #[rstest]
    #[case(0, 40, 1)] // no agents: footer only
    #[case(3, 40, 4)] // fits
    #[case(15, 20, 12)] // capped to leave MIN_TERMINAL_ROWS for the scene
    #[case(10, 8, 1)] // never below the footer
    fn height_leaves_room_for_the_scene(
        #[case] agents: usize,
        #[case] total: u16,
        #[case] want: u16,
    ) {
        assert_eq!(AgentPanel::height_for(agents, total), want);
    }

    #[test]
    fn row_shows_identity_status_and_summary() {
        let buf = render(
            &panel(vec![row("farm (main)", "Bash: cargo test", false)], false),
            80,
            2,
        );
        let line = text_of(&buf, 0);
        assert!(line.contains("farm (main)"), "{line}");
        assert!(line.contains("Active"), "{line}");
        assert!(line.contains("Bash: cargo test"), "{line}");
    }

    #[test]
    fn long_summary_is_cut_to_the_row_width() {
        let long = "x".repeat(200);
        let buf = render(&panel(vec![row("farm", &long, false)], false), 60, 2);
        let line = text_of(&buf, 0);
        assert!(line.trim_end().ends_with('…'), "{line}");
    }

    #[test]
    fn wide_characters_in_a_summary_are_not_split() {
        let buf = render(
            &panel(
                vec![row("农场", "正在运行测试，请稍等片刻再继续操作", false)],
                false,
            ),
            40,
            2,
        );
        let line = text_of(&buf, 0);
        assert!(line.contains("农场"), "{line}");
        assert!(line.trim_end().ends_with('…'), "{line}");
    }

    #[test]
    fn selected_row_is_highlighted() {
        let buf = render(
            &panel(vec![row("a", "x", false), row("b", "y", true)], false),
            40,
            3,
        );
        assert_eq!(buf[(5, 0)].bg, PANEL_BG);
        assert_eq!(buf[(5, 1)].bg, SELECTED_BG);
    }

    #[test]
    fn rows_beyond_the_height_are_dropped_but_the_footer_stays() {
        let rows = (0..5)
            .map(|i| row(&format!("p{i}"), "work", false))
            .collect();
        let buf = render(&panel(rows, false), 60, 3);
        assert!(text_of(&buf, 0).contains("p0"));
        assert!(text_of(&buf, 1).contains("p1"));
        assert!(text_of(&buf, 2).contains("q quit"));
    }

    #[rstest]
    #[case(false, false)]
    #[case(true, true)]
    fn too_small_hint_follows_the_flag(#[case] too_small: bool, #[case] shown: bool) {
        let buf = render(&panel(vec![], too_small), 120, 1);
        assert_eq!(text_of(&buf, 0).contains("enlarge terminal"), shown);
    }

    #[test]
    fn zero_area_renders_nothing() {
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(area);
        panel(vec![row("a", "b", false)], false).render(area, &mut buf);
    }
}
