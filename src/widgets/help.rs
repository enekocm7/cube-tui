use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::model::settings::ThemeSettings;

enum HelpLine {
    Header(&'static str),
    Body(&'static str),
    Empty,
}

const HELP_TEXT: &[HelpLine] = &[
    HelpLine::Header("TIMER CONTROLS"),
    HelpLine::Body("Space              Hold and release to start/stop timer"),
    HelpLine::Body("r                  Reset timer"),
    HelpLine::Body("n                  Next scramble"),
    HelpLine::Empty,
    HelpLine::Header("EVENT NAVIGATION"),
    HelpLine::Body("e / E              Next / Previous event"),
    HelpLine::Empty,
    HelpLine::Header("SESSION MANAGEMENT"),
    HelpLine::Body("[ / ]              Previous / Next session"),
    HelpLine::Body("s                  Create new session"),
    HelpLine::Body("S                  Delete current session"),
    HelpLine::Empty,
    HelpLine::Header("INSPECTION"),
    HelpLine::Body("i                  Toggle disable/enable inspection"),
    HelpLine::Empty,
    HelpLine::Header("HISTORY NAVIGATION"),
    HelpLine::Body("Up / Down          Select previous / next time in history"),
    HelpLine::Body("Enter              Open details screen for selected time"),
    HelpLine::Body("Tab                Toggle focus between history and stats"),
    HelpLine::Body("t                  Open detailed stats screen"),
    HelpLine::Body("d                  Delete selected time"),
    HelpLine::Empty,
    HelpLine::Header("MAIN STATS FOCUS"),
    HelpLine::Body("Up / Down          Select time/mo3/ao5 row"),
    HelpLine::Body("Left / Right       Select current/best column"),
    HelpLine::Body("Enter              Open mean detail for selected mean cell"),
    HelpLine::Empty,
    HelpLine::Header("DETAILED STATS"),
    HelpLine::Body("Up / Down          Select solve"),
    HelpLine::Body("Left / Right       Switch mo3 / ao5 column"),
    HelpLine::Body("Enter              Open mean detail"),
    HelpLine::Body("Esc                Close detailed stats"),
    HelpLine::Empty,
    HelpLine::Header("MEAN DETAIL"),
    HelpLine::Body("Up / Down          Select time within mean"),
    HelpLine::Body("Enter              Open details for selected time"),
    HelpLine::Body("Esc                Back to detailed stats"),
    HelpLine::Empty,
    HelpLine::Header("DETAILS SCREEN"),
    HelpLine::Body("Left / Right       Navigate to previous / next time"),
    HelpLine::Body("Up / Down          Select +2 / DNF modifier"),
    HelpLine::Body("Space              Toggle selected modifier"),
    HelpLine::Body("d                  Delete selected time"),
    HelpLine::Body("Esc                Close details screen"),
    HelpLine::Empty,
    HelpLine::Header("INTERFACE"),
    HelpLine::Body("?                  Show / Hide this help screen"),
    HelpLine::Body("q                  Quit application"),
    HelpLine::Empty,
    HelpLine::Header("ZEN MODE"),
    HelpLine::Body("z                  Toggle zen mode (hides UI while timer runs)"),
    HelpLine::Empty,
    HelpLine::Header("BLUETOOTH"),
    HelpLine::Body("b                  Open bluetooth device list"),
    HelpLine::Body("Up / Down          Select bluetooth device"),
    HelpLine::Body("Enter              Connect to selected device"),
    HelpLine::Body("Esc                Close bluetooth device list"),
    HelpLine::Empty,
];

pub struct HelpWidget {
    scroll: u16,
}

impl HelpWidget {
    pub const fn new(scroll: u16) -> Self {
        Self { scroll }
    }

    pub fn max_scroll_for_height(height: u16) -> u16 {
        let total_lines = u16::try_from(HELP_TEXT.len()).unwrap_or(u16::MAX);
        let visible_lines = height.saturating_sub(2);
        total_lines.saturating_sub(visible_lines)
    }

    pub fn render_with_theme(self, area: Rect, buf: &mut Buffer, theme: &ThemeSettings) {
        let text_color = theme.text();
        let help_text: Vec<Line> = HELP_TEXT
            .iter()
            .map(|entry| match entry {
                HelpLine::Header(text) => Line::from(vec![Span::styled(
                    *text,
                    Style::default().fg(text_color).add_modifier(Modifier::BOLD),
                )]),
                HelpLine::Body(text) => {
                    Line::from(Span::styled(*text, Style::default().fg(text_color)))
                }
                HelpLine::Empty => Line::from(""),
            })
            .collect();

        let max_scroll = u16::try_from(help_text.len())
            .unwrap_or(u16::MAX)
            .saturating_sub(area.height.saturating_sub(2));
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
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border()));

        Paragraph::new(help_text)
            .block(block)
            .scroll((scroll, 0))
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
