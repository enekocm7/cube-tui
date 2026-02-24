use chrono::{Local, TimeZone};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier as StyleModifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

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
}

impl Widget for DetailsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::default().title("Time Details").borders(Borders::ALL);

        let lines = self.time.map_or_else(
            || vec![Line::from("No time selected."), Line::from("Esc: close")],
            |time| {
                let plus_two_checked = matches!(time.modifier(), Modifier::PlusTwo);
                let dnf_checked = matches!(time.modifier(), Modifier::DNF);

                vec![
                    Line::from(format!("Time: {time}")),
                    Line::from(format!(
                        "Datetime: {}",
                        format_datetime(time.solved_at_unix_ms())
                    )),
                    Line::from(""),
                    Line::from(format!("Event: {}", time.event().name())),
                    Line::from(""),
                    Line::from(format!("Scramble: {}", time.scramble())),
                    Line::from(""),
                    Line::from("Modifiers:"),
                    checkbox_line("+2", plus_two_checked, self.selected_modifier_index == 0),
                    checkbox_line("DNF", dnf_checked, self.selected_modifier_index == 1),
                ]
            },
        );

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}

fn checkbox_line(label: &str, checked: bool, selected: bool) -> Line<'_> {
    let check = if checked { "x" } else { " " };
    let style = if selected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(StyleModifier::BOLD)
    } else {
        Style::default()
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
