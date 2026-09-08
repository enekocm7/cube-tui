use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::model::keybinds::{Action, Keybinds};
use crate::model::settings::ThemeColors;

enum HelpLine {
    Header(&'static str),
    Body(Action, &'static str),
    Pair(Action, Action, &'static str),
    Empty,
}

const HELP_TEXT: &[HelpLine] = &[
    HelpLine::Header("TIMER CONTROLS"),
    HelpLine::Body(Action::Timer, "Hold and release to start/stop timer"),
    HelpLine::Body(Action::ResetTimer, "Reset timer"),
    HelpLine::Body(Action::NextScramble, "Next scramble"),
    HelpLine::Empty,
    HelpLine::Header("EVENT NAVIGATION"),
    HelpLine::Pair(
        Action::NextEvent,
        Action::PreviousEvent,
        "Next / Previous event",
    ),
    HelpLine::Empty,
    HelpLine::Header("SESSION MANAGEMENT"),
    HelpLine::Pair(
        Action::PreviousSession,
        Action::NextSession,
        "Previous / Next session",
    ),
    HelpLine::Body(Action::NewSession, "Create new session"),
    HelpLine::Body(Action::DeleteSession, "Delete current session"),
    HelpLine::Empty,
    HelpLine::Header("INSPECTION"),
    HelpLine::Body(Action::ToggleInspection, "Toggle disable/enable inspection"),
    HelpLine::Empty,
    HelpLine::Header("HISTORY NAVIGATION"),
    HelpLine::Pair(
        Action::SelectUp,
        Action::SelectDown,
        "Select previous / next time in history",
    ),
    HelpLine::Body(Action::Enter, "Open details screen for selected time"),
    HelpLine::Body(
        Action::ToggleFocus,
        "Toggle focus between history and stats",
    ),
    HelpLine::Body(Action::DetailedStats, "Open detailed stats screen"),
    HelpLine::Body(Action::DeleteTime, "Delete selected time"),
    HelpLine::Empty,
    HelpLine::Header("MAIN STATS FOCUS"),
    HelpLine::Pair(
        Action::SelectUp,
        Action::SelectDown,
        "Select time/mo3/ao5 row",
    ),
    HelpLine::Pair(
        Action::NavigateLeft,
        Action::NavigateRight,
        "Select current/best column",
    ),
    HelpLine::Body(Action::Enter, "Open mean detail for selected mean cell"),
    HelpLine::Empty,
    HelpLine::Header("DETAILED STATS"),
    HelpLine::Pair(Action::SelectUp, Action::SelectDown, "Select solve"),
    HelpLine::Pair(
        Action::NavigateLeft,
        Action::NavigateRight,
        "Switch mo3 / ao5 column",
    ),
    HelpLine::Body(Action::Enter, "Open mean detail"),
    HelpLine::Body(Action::Back, "Close detailed stats"),
    HelpLine::Empty,
    HelpLine::Header("MEAN DETAIL"),
    HelpLine::Pair(
        Action::SelectUp,
        Action::SelectDown,
        "Select time within mean",
    ),
    HelpLine::Body(Action::Enter, "Open details for selected time"),
    HelpLine::Body(Action::Back, "Back to detailed stats"),
    HelpLine::Empty,
    HelpLine::Header("DETAILS SCREEN"),
    HelpLine::Pair(
        Action::NavigateLeft,
        Action::NavigateRight,
        "Navigate to previous / next time",
    ),
    HelpLine::Pair(
        Action::SelectUp,
        Action::SelectDown,
        "Select +2 / DNF modifier",
    ),
    HelpLine::Body(Action::Timer, "Toggle selected modifier"),
    HelpLine::Body(Action::DeleteTime, "Delete selected time"),
    HelpLine::Body(Action::Back, "Close details screen"),
    HelpLine::Empty,
    HelpLine::Header("INTERFACE"),
    HelpLine::Body(Action::Help, "Show / Hide this help screen"),
    HelpLine::Body(Action::Quit, "Quit application"),
    HelpLine::Empty,
    HelpLine::Header("ZEN MODE"),
    HelpLine::Body(
        Action::ToggleZen,
        "Toggle zen mode (hides UI while timer runs)",
    ),
    HelpLine::Empty,
    HelpLine::Header("THEMES"),
    HelpLine::Body(Action::ThemeSelector, "Open theme selector"),
    HelpLine::Body(Action::Back, "Close theme selector"),
    HelpLine::Empty,
];

#[cfg(feature = "bluetooth")]
const BLUETOOTH_HELP_TEXT: &[HelpLine] = &[
    HelpLine::Header("BLUETOOTH"),
    HelpLine::Body(Action::Bluetooth, "Open bluetooth device list"),
    HelpLine::Pair(
        Action::SelectUp,
        Action::SelectDown,
        "Select bluetooth device",
    ),
    HelpLine::Body(Action::Enter, "Connect to selected device"),
    HelpLine::Body(Action::Back, "Close bluetooth device list"),
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
        let total_lines = u16::try_from(total_help_lines()).unwrap_or(u16::MAX);
        let visible_lines = height.saturating_sub(2);
        total_lines.saturating_sub(visible_lines)
    }

    pub fn render_with_theme(
        self,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeColors,
        keybinds: &Keybinds,
    ) {
        let text_color = theme.text();
        let help_text: Vec<Line> = HELP_TEXT
            .iter()
            .map(|entry| help_line_to_line(entry, text_color, keybinds))
            .collect();
        #[cfg(feature = "bluetooth")]
        let help_text = {
            let mut help_text = help_text;
            help_text.extend(
                BLUETOOTH_HELP_TEXT
                    .iter()
                    .map(|entry| help_line_to_line(entry, text_color, keybinds)),
            );
            help_text
        };

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

fn total_help_lines() -> usize {
    #[cfg(feature = "bluetooth")]
    let count = HELP_TEXT.len() + BLUETOOTH_HELP_TEXT.len();
    #[cfg(not(feature = "bluetooth"))]
    let count = HELP_TEXT.len();
    count
}

fn help_line_to_line<'a>(entry: &HelpLine, text_color: Color, keybinds: &Keybinds) -> Line<'a> {
    match entry {
        HelpLine::Header(text) => Line::from(vec![Span::styled(
            *text,
            Style::default().fg(text_color).add_modifier(Modifier::BOLD),
        )]),
        HelpLine::Body(action, text) => Line::from(Span::styled(
            format!("{:<18} {text}", keybinds.label(*action)),
            Style::default().fg(text_color),
        )),
        HelpLine::Pair(first, second, text) => Line::from(Span::styled(
            format!(
                "{:<18} {text}",
                format!("{} / {}", keybinds.label(*first), keybinds.label(*second))
            ),
            Style::default().fg(text_color),
        )),
        HelpLine::Empty => Line::from(""),
    }
}
