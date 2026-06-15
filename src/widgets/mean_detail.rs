use std::borrow::Cow;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::model::settings::ThemeSettings;
use crate::widgets::history::{History, Time};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeanType {
    Mo3,
    Ao5,
}

pub struct MeanDetailWidget<'a> {
    mean_type: MeanType,
    solve_index: usize,
    mean_value: Cow<'static, str>,
    times: &'a [Time],
    times_start_index: usize,
    selected_index: usize,
}

impl<'a> MeanDetailWidget<'a> {
    pub fn new(
        history: &'a History,
        solve_index: usize,
        col: usize,
        selected_index: usize,
    ) -> Self {
        let (mean_type, mean_value, times, times_start_index) = if col == 0 {
            let val = history.mo3_at(solve_index).unwrap_or(Cow::Borrowed("-"));
            let times = history.mo3_times_at(solve_index).unwrap_or(&[]);
            (MeanType::Mo3, val, times, solve_index.saturating_sub(2))
        } else {
            let val = history.ao5_at(solve_index).unwrap_or(Cow::Borrowed("-"));
            let times = history.ao5_times_at(solve_index).unwrap_or(&[]);
            (MeanType::Ao5, val, times, solve_index.saturating_sub(4))
        };
        Self {
            mean_type,
            solve_index,
            mean_value,
            times,
            times_start_index,
            selected_index,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeSettings) {
        let type_name = match self.mean_type {
            MeanType::Mo3 => "Mean of 3",
            MeanType::Ao5 => "Average of 5",
        };

        let title = format!(
            "{} at solve #{} (Enter: open time, Esc: back)",
            type_name,
            self.solve_index + 1
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()));

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled(
                format!("{type_name}: "),
                Style::default()
                    .fg(theme.text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                self.mean_value.clone(),
                Style::default()
                    .fg(theme.text())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        let trimmed = compute_trimmed_indices(self.times, self.mean_type);

        lines.push(Line::from(Span::styled(
            "Times:",
            Style::default()
                .fg(theme.text())
                .add_modifier(Modifier::BOLD),
        )));

        let best_ms = self.times.iter().filter_map(Time::effective_ms).min();

        for (window_idx, time) in self.times.iter().enumerate() {
            let history_index = self.times_start_index + window_idx + 1;
            let is_trimmed = trimmed.contains(&window_idx);
            let is_selected = window_idx == self.selected_index;
            let time_display = time.to_string();

            let annotation = if is_trimmed {
                if time.effective_ms().is_none() {
                    Cow::Borrowed(" ← worst (trimmed)")
                } else if best_ms.is_some_and(|min| time.effective_ms() == Some(min)) {
                    Cow::Borrowed("  best")
                } else {
                    Cow::Borrowed("  worst")
                }
            } else {
                Cow::Borrowed("")
            };

            let style = if is_selected {
                Style::default()
                    .bg(theme.selection())
                    .fg(theme.selection_text())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text())
            };

            let annotation_style = Style::default().fg(theme.text());

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  #{history_index}: "),
                    Style::default().fg(theme.text()),
                ),
                Span::styled(
                    if is_trimmed {
                        format!("({time_display})")
                    } else {
                        time_display
                    },
                    style,
                ),
                Span::styled(annotation, annotation_style),
            ]));
        }

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}

fn compute_trimmed_indices(times: &[Time], mean_type: MeanType) -> Vec<usize> {
    if mean_type != MeanType::Ao5 || times.len() != 5 {
        return vec![];
    }

    let effective: [Option<u64>; 5] = [
        times[0].effective_ms(),
        times[1].effective_ms(),
        times[2].effective_ms(),
        times[3].effective_ms(),
        times[4].effective_ms(),
    ];
    let dnf_count = effective.iter().filter(|e| e.is_none()).count();

    if dnf_count >= 2 {
        return vec![];
    }

    let mut trimmed = Vec::new();

    if dnf_count == 1 {
        let dnf_idx = effective.iter().position(Option::is_none).unwrap();
        trimmed.push(dnf_idx);

        let mut best_val = u64::MAX;
        let mut best_idx = 0;
        for (i, e) in effective.iter().enumerate() {
            if let Some(v) = e
                && *v < best_val
            {
                best_val = *v;
                best_idx = i;
            }
        }
        trimmed.push(best_idx);
    } else {
        let mut best_val = u64::MAX;
        let mut best_idx = 0;
        let mut worst_val = 0;
        let mut worst_idx = 0;

        for (i, e) in effective.iter().enumerate() {
            if let Some(v) = e {
                if *v < best_val {
                    best_val = *v;
                    best_idx = i;
                }
                if *v > worst_val {
                    worst_val = *v;
                    worst_idx = i;
                }
            }
        }

        if best_idx == worst_idx {
            trimmed.push(0);
            trimmed.push(4);
        } else {
            trimmed.push(best_idx);
            trimmed.push(worst_idx);
        }
    }

    trimmed
}
