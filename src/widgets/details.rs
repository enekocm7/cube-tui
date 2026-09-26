use chrono::{Local, TimeZone};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier as StyleModifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::model::settings::ThemeColors;
use crate::widgets::history::{Modifier, Time, format_millis};

pub struct DetailsWidget<'a> {
    time: Option<&'a Time>,
    selected_modifier_index: usize,
}

impl<'a> DetailsWidget<'a> {
    /// Creates a solve-details widget for an optional selected solve.
    pub const fn new(time: Option<&'a Time>, selected_modifier_index: usize) -> Self {
        Self {
            time,
            selected_modifier_index,
        }
    }

    /// Renders solve metadata and editable penalty choices.
    pub fn render_with_theme(self, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
        let block = Block::default()
            .title("Time Details")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()));

        let lines = self.time.map_or_else(
            || {
                vec![Line::from(Span::styled(
                    "No time selected.",
                    Style::default().fg(theme.text()),
                ))]
            },
            |time| {
                let plus_two_checked = matches!(time.modifier(), Modifier::PlusTwo);
                let dnf_checked = matches!(time.modifier(), Modifier::DNF);
                let time_text = if dnf_checked {
                    format!("Time: {time} ({})", format_millis(time.raw_ms()))
                } else {
                    format!("Time: {time}")
                };
                vec![
                    Line::from(Span::styled(time_text, Style::default().fg(theme.text()))),
                    Line::from(Span::styled(
                        format!("Datetime: {}", format_datetime(time.solved_at_unix_ms())),
                        Style::default().fg(theme.text()),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Event: {}", time.event().name()),
                        Style::default().fg(theme.text()),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Scramble: {}", time.scramble()),
                        Style::default().fg(theme.text()),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Modifiers:",
                        Style::default().fg(theme.text()),
                    )),
                    checkbox_line(
                        "+2",
                        plus_two_checked,
                        self.selected_modifier_index == 0,
                        theme,
                    ),
                    checkbox_line("DNF", dnf_checked, self.selected_modifier_index == 1, theme),
                ]
            },
        );

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}

/// Builds one selectable checkbox row for a solve modifier.
fn checkbox_line(label: &str, checked: bool, selected: bool, theme: &ThemeColors) -> Line<'static> {
    let check = if checked { "x" } else { " " };
    let style = if selected {
        Style::default()
            .fg(theme.text())
            .add_modifier(StyleModifier::BOLD)
    } else {
        Style::default().fg(theme.text())
    };
    Line::from(Span::styled(format!("[{check}] {label}"), style))
}

/// Formats a Unix millisecond timestamp in the user's local time zone.
fn format_datetime(unix_ms: u64) -> String {
    if unix_ms == 0 {
        return "-".to_string();
    }

    let Ok(unix_ms) = i64::try_from(unix_ms) else {
        return "-".to_string();
    };
    Local.timestamp_millis_opt(unix_ms).single().map_or_else(
        || "-".to_string(),
        |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
    )
}
