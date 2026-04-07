use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::types::AgentStatus;

/// Per-agent summary shown in the status bar.
#[derive(Debug, Clone)]
pub struct AgentSummary {
    /// RGB palette dot color.
    pub color: (u8, u8, u8),
    /// Short project name.
    pub project_name: String,
    /// Current agent status.
    pub status: AgentStatus,
    /// Currently active tool name, if any.
    pub tool_name: Option<String>,
}

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

/// Bottom status bar showing per-agent summaries, selection details, and keybindings.
#[derive(Debug, Clone)]
pub struct StatusBar {
    /// Per-agent summaries.
    pub agents: Vec<AgentSummary>,
    /// Info for the selected agent, if any.
    pub selected_info: Option<SelectedInfo>,
}

impl StatusBar {
    fn left_spans(&self) -> Vec<Span<'_>> {
        let mut spans = vec![Span::styled(
            format!(" {} ", self.agents.len()),
            Style::default().fg(Color::DarkGray),
        )];

        for (i, agent) in self.agents.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }

            let (r, g, b) = agent.color;
            spans.push(Span::styled("●", Style::default().fg(Color::Rgb(r, g, b))));

            // Truncate project name to 12 chars
            let name = truncate_name(&agent.project_name, 12);
            spans.push(Span::styled(
                format!("{name} "),
                Style::default().fg(Color::Cyan),
            ));

            spans.push(status_span(agent.status));

            if let Some(tool) = &agent.tool_name {
                spans.push(Span::styled(
                    format!(":{tool}"),
                    Style::default().fg(Color::White),
                ));
            }
        }

        if !self.agents.is_empty() {
            spans.push(Span::raw(" "));
        }

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
        let right = self.right_spans();

        let left_width: usize = left.iter().map(|s| s.width()).sum();
        let right_width: usize = right.iter().map(|s| s.width()).sum();
        let total_width = area.width as usize;

        // Render left-aligned spans
        let left_line = Line::from(left);
        let left_area = Rect::new(area.x, area.y, total_width.min(left_width) as u16, 1);
        left_line.render(left_area, buf);

        // Render right-aligned spans
        if total_width >= right_width {
            let right_start = total_width - right_width;
            let right_line = Line::from(right);
            let right_area = Rect::new(area.x + right_start as u16, area.y, right_width as u16, 1);
            right_line.render(right_area, buf);
        }
    }
}

/// Truncate a string to `max` characters, appending "…" if truncated.
fn truncate_name(name: &str, max: usize) -> String {
    if name.len() <= max {
        name.to_owned()
    } else {
        format!("{}…", &name[..max - 1])
    }
}

/// Map agent status to a colored span.
fn status_span(status: AgentStatus) -> Span<'static> {
    match status {
        AgentStatus::Active => Span::styled("Active", Style::default().fg(Color::Green)),
        AgentStatus::Idle => Span::styled("Idle", Style::default().fg(Color::Gray)),
        AgentStatus::Waiting => Span::styled("Waiting", Style::default().fg(Color::Yellow)),
        AgentStatus::Permission => Span::styled("Permission", Style::default().fg(Color::Red)),
        AgentStatus::Dormant => Span::styled("Resting", Style::default().fg(Color::DarkGray)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn status_bar_renders_without_panic() {
        let bar = StatusBar {
            agents: vec![
                AgentSummary {
                    color: (255, 0, 0),
                    project_name: "my-project".into(),
                    status: AgentStatus::Active,
                    tool_name: Some("Read".into()),
                },
                AgentSummary {
                    color: (0, 255, 0),
                    project_name: "other".into(),
                    status: AgentStatus::Idle,
                    tool_name: None,
                },
            ],
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
    fn status_bar_renders_empty() {
        let bar = StatusBar {
            agents: vec![],
            selected_info: None,
        };
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        (&bar).render(area, &mut buf);
    }

    #[test]
    fn status_bar_handles_zero_area() {
        let bar = StatusBar {
            agents: vec![AgentSummary {
                color: (128, 128, 128),
                project_name: "test".into(),
                status: AgentStatus::Active,
                tool_name: None,
            }],
            selected_info: None,
        };
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(area);
        (&bar).render(area, &mut buf);
    }

    #[test]
    fn truncate_name_short() {
        assert_eq!(truncate_name("hello", 12), "hello");
    }

    #[test]
    fn truncate_name_long() {
        let result = truncate_name("very-long-project-name", 12);
        assert!(result.len() <= 14); // 11 + "…" (multi-byte)
        assert!(result.ends_with('…'));
    }
}
