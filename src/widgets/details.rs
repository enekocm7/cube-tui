use chrono::{Local, TimeZone};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier as StyleModifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::model::settings::ThemeSettings;
use crate::widgets::history::{Modifier, Time};

pub struct DetailsWidget<'a> {
    time: Option<&'a Time>,
    selected_modifier_index: usize,
}

impl<'a> DetailsWidget<'a> {
    pub const fn new(time: Option<&'a Time>, selected_modifier_index: usize) -> Self {
        Self {
            time,
            selected_modifier_index,
        }
    }

    pub fn render_with_theme(self, area: Rect, buf: &mut Buffer, theme: &ThemeSettings) {
        let block = Block::default()
            .title("Time Details")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()));

        let lines = self.time.map_or_else(
            || {
                vec![
                    Line::from(Span::styled(
                        "No time selected.",
                        Style::default().fg(theme.text()),
                    )),
                    Line::from(Span::styled(
                        "Esc: close",
                        Style::default().fg(theme.text()),
                    )),
                ]
            },
            |time| {
                let plus_two_checked = matches!(time.modifier(), Modifier::PlusTwo);
                let dnf_checked = matches!(time.modifier(), Modifier::DNF);

                vec![
                    Line::from(Span::styled(
                        format!("Time: {time}"),
                        Style::default().fg(theme.text()),
                    )),
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

fn checkbox_line(
    label: &str,
    checked: bool,
    selected: bool,
    theme: &ThemeSettings,
) -> Line<'static> {
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

fn format_datetime(unix_ms: u64) -> String {
    if unix_ms == 0 {
        return "-".to_string();
    }

    Local
        .timestamp_millis_opt(i64::try_from(unix_ms).expect("Failed to parse time"))
        .single()
        .map_or_else(
            || "-".to_string(),
            |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        )
}
