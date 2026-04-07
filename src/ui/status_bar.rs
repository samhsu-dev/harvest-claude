use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::types::AgentStatus;

/// Information about the currently selected agent.
#[derive(Debug, Clone)]
pub struct SelectedInfo {
    /// Project directory name.
    pub project_name: String,
    /// Session identifier.
    pub session_id: String,
    /// Current agent status.
    pub status: AgentStatus,
    /// Name of the active tool, if any.
    pub tool_name: Option<String>,
}

/// Bottom status bar showing agent count, selection details, and keybindings.
#[derive(Debug, Clone)]
pub struct StatusBar {
    /// Number of active agents.
    pub agent_count: usize,
    /// RGB palette dot colors, one per agent.
    pub palette_dots: Vec<(u8, u8, u8)>,
    /// Info for the selected agent, if any.
    pub selected_info: Option<SelectedInfo>,
}

impl StatusBar {
    fn left_spans(&self) -> Vec<Span<'_>> {
        let mut spans = vec![Span::styled(
            format!(" {} agents ", self.agent_count),
            Style::default().fg(Color::White),
        )];
        for &(r, g, b) in &self.palette_dots {
            spans.push(Span::styled("●", Style::default().fg(Color::Rgb(r, g, b))));
        }
        if !self.palette_dots.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans
    }

    fn center_spans(&self) -> Vec<Span<'_>> {
        let Some(info) = &self.selected_info else {
            return Vec::new();
        };

        let mut spans = vec![Span::styled(
            &info.project_name,
            Style::default().fg(Color::Cyan),
        )];

        if let Some(tool) = &info.tool_name {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                tool.as_str(),
                Style::default().fg(Color::White),
            ));
        }

        spans.push(Span::raw(" "));
        spans.push(status_span(info.status));
        spans
    }

    fn right_spans(&self) -> Vec<Span<'static>> {
        vec![Span::styled(
            "q:quit  ←→:select  esc:deselect ",
            Style::default().fg(Color::DarkGray),
        )]
    }
}

impl Widget for &StatusBar {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // Fill background
        for x in area.left()..area.right() {
            buf[(x, area.top())]
                .set_style(Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White));
            buf[(x, area.top())].set_char(' ');
        }

        let left = self.left_spans();
        let center = self.center_spans();
        let right = self.right_spans();

        let left_width: usize = left.iter().map(|s| s.width()).sum();
        let right_width: usize = right.iter().map(|s| s.width()).sum();
        let center_width: usize = center.iter().map(|s| s.width()).sum();
        let total_width = area.width as usize;

        // Render left-aligned spans
        let left_line = Line::from(left);
        let left_area = Rect::new(area.x, area.y, total_width.min(left_width) as u16, 1);
        left_line.render(left_area, buf);

        // Render center spans (centered in remaining space)
        if center_width > 0 && total_width > left_width + right_width {
            let center_start = total_width.saturating_sub(center_width) / 2;
            let center_start = center_start.max(left_width);
            let available = total_width.saturating_sub(center_start);
            if available > 0 {
                let center_line = Line::from(center);
                let center_area = Rect::new(
                    area.x + center_start as u16,
                    area.y,
                    available.min(center_width) as u16,
                    1,
                );
                center_line.render(center_area, buf);
            }
        }

        // Render right-aligned spans
        if total_width >= right_width {
            let right_start = total_width - right_width;
            let right_line = Line::from(right);
            let right_area = Rect::new(area.x + right_start as u16, area.y, right_width as u16, 1);
            right_line.render(right_area, buf);
        }
    }
}

/// Map agent status to a colored span.
fn status_span(status: AgentStatus) -> Span<'static> {
    match status {
        AgentStatus::Active => Span::styled("Active", Style::default().fg(Color::Green)),
        AgentStatus::Idle => Span::styled("Idle", Style::default().fg(Color::Gray)),
        AgentStatus::Waiting => Span::styled("Waiting", Style::default().fg(Color::Yellow)),
        AgentStatus::Permission => Span::styled("Permission", Style::default().fg(Color::Red)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn status_bar_renders_without_panic() {
        let bar = StatusBar {
            agent_count: 2,
            palette_dots: vec![(255, 0, 0), (0, 255, 0)],
            selected_info: Some(SelectedInfo {
                project_name: "my-project".into(),
                session_id: "abc123".into(),
                status: AgentStatus::Active,
                tool_name: Some("Read".into()),
            }),
        };
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        (&bar).render(area, &mut buf);
    }

    #[test]
    fn status_bar_renders_empty_selection() {
        let bar = StatusBar {
            agent_count: 0,
            palette_dots: vec![],
            selected_info: None,
        };
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        (&bar).render(area, &mut buf);
    }

    #[test]
    fn status_bar_handles_zero_area() {
        let bar = StatusBar {
            agent_count: 1,
            palette_dots: vec![(128, 128, 128)],
            selected_info: None,
        };
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(area);
        (&bar).render(area, &mut buf);
    }
}
