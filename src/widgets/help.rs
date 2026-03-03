use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

pub struct HelpWidget {
    scroll: u16,
}

impl HelpWidget {
    pub const fn new(scroll: u16) -> Self {
        Self { scroll }
    }

    #[allow(clippy::too_many_lines)]
    fn help_text() -> Vec<Line<'static>> {
        vec![
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
            Line::from("Tab                Toggle focus between history and stats"),
            Line::from("t                  Open detailed stats screen"),
            Line::from("d                  Delete selected time"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "MAIN STATS FOCUS",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Up / Down          Select time/mo3/ao5 row"),
            Line::from("Left / Right       Select current/best column"),
            Line::from("Enter              Open mean detail for selected mean cell"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "DETAILED STATS",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Up / Down          Select solve"),
            Line::from("Left / Right       Switch mo3 / ao5 column"),
            Line::from("Enter              Open mean detail"),
            Line::from("Esc                Close detailed stats"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "MEAN DETAIL",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Up / Down          Select time within mean"),
            Line::from("Enter              Open details for selected time"),
            Line::from("Esc                Back to detailed stats"),
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
            Line::from(vec![Span::styled(
                "BLUETOOTH",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("b                  Open bluetooth device list"),
            Line::from("Up / Down          Select bluetooth device"),
            Line::from("Enter              Connect to selected device"),
            Line::from("Esc                Close bluetooth device list"),
            Line::from(""),
        ]
    }

    pub fn max_scroll_for_height(height: u16) -> u16 {
        let total_lines = u16::try_from(Self::help_text().len()).unwrap_or(u16::MAX);
        let visible_lines = height.saturating_sub(2);
        total_lines.saturating_sub(visible_lines)
    }
}

impl Widget for HelpWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let help_text = Self::help_text();

        let max_scroll = Self::max_scroll_for_height(area.height);
        let scroll = self.scroll.min(max_scroll);

        let title = if scroll > 0 && scroll < max_scroll {
            "Commands Help (↑ more, ↓ more)"
        } else if scroll > 0 {
            "Commands Help (↑ more)"
        } else if scroll < max_scroll {
            "Commands Help (↓ more)"
        } else {
            "Commands Help"
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded);

        Paragraph::new(help_text)
            .block(block)
            .scroll((scroll, 0))
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
