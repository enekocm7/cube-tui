use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

pub struct HelpWidget;

impl Widget for HelpWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::default()
            .title("Commands Help")
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded);

        let help_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "TIMER CONTROLS",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Space              Hold and release to start/stop timer"),
            Line::from("r                  Reset timer"),
            Line::from("n                  Next scramble"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "EVENT NAVIGATION",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("e / E              Next / Previous event"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "SESSION MANAGEMENT",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("[ / ]              Previous / Next session"),
            Line::from("s                  Create new session"),
            Line::from("S                  Delete current session"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "INSPECTION",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("i                  Toggle disable/enable inspection"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "HISTORY NAVIGATION",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Up / Down          Select previous / next time in history"),
            Line::from("Enter              Open details screen for selected time"),
            Line::from("d                  Delete selected time"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "DETAILS SCREEN",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Left / Right       Navigate to previous / next time"),
            Line::from("Up / Down          Select +2 / DNF modifier"),
            Line::from("Space              Toggle selected modifier"),
            Line::from("d                  Delete selected time"),
            Line::from("Esc                Close details screen"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "INTERFACE",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("?                  Show / Hide this help screen"),
            Line::from("q                  Quit application"),
            Line::from(""),
        ];

        Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
