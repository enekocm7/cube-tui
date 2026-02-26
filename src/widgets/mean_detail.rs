use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::widgets::history::{History, Time};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeanType {
    Mo3,
    Ao5,
}

pub struct MeanDetailWidget {
    mean_type: MeanType,
    solve_index: usize,
    mean_value: String,
    times: Vec<(usize, Time)>,
    selected_index: usize,
}

impl MeanDetailWidget {
    pub fn new(history: &History, solve_index: usize, col: usize, selected_index: usize) -> Self {
        let (mean_type, mean_value, times) = if col == 0 {
            let val = history
                .mo3_at(solve_index)
                .unwrap_or_else(|| "-".to_string());
            let ts = history
                .mo3_times_at(solve_index)
                .map_or_else(Vec::new, |slice| {
                    let start = solve_index.saturating_sub(2);
                    slice
                        .iter()
                        .enumerate()
                        .map(|(i, t)| (start + i + 1, t.clone()))
                        .collect()
                });
            (MeanType::Mo3, val, ts)
        } else {
            let val = history
                .ao5_at(solve_index)
                .unwrap_or_else(|| "-".to_string());
            let ts = history
                .ao5_times_at(solve_index)
                .map_or_else(Vec::new, |slice| {
                    let start = solve_index.saturating_sub(4);
                    slice
                        .iter()
                        .enumerate()
                        .map(|(i, t)| (start + i + 1, t.clone()))
                        .collect()
                });
            (MeanType::Ao5, val, ts)
        };

        Self {
            mean_type,
            solve_index,
            mean_value,
            times,
            selected_index,
        }
    }
}

impl Widget for MeanDetailWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let type_name = match self.mean_type {
            MeanType::Mo3 => "Mean of 3",
            MeanType::Ao5 => "Average of 5",
        };

        let title = format!(
            "{} at solve #{} (Enter: open time, Esc: back)",
            type_name,
            self.solve_index + 1
        );

        let block = Block::default().title(title).borders(Borders::ALL);

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled(
                format!("{type_name}: "),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                self.mean_value.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        let trimmed = compute_trimmed_indices(&self.times, self.mean_type);

        lines.push(Line::from(Span::styled(
            "Times:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        for (i, (num, time)) in self.times.iter().enumerate() {
            let is_trimmed = trimmed.contains(&i);
            let is_selected = i == self.selected_index;
            let time_display = time.to_string();

            let annotation = if is_trimmed {
                if time.effective_ms().is_none() {
                    " ← worst (trimmed)"
                } else {
                    let effective = time.effective_ms().unwrap_or(u64::MAX);
                    let all_effective: Vec<u64> = self
                        .times
                        .iter()
                        .filter_map(|(_, t)| t.effective_ms())
                        .collect();
                    if let Some(&min) = all_effective.iter().min() {
                        if effective == min {
                            "  best"
                        } else {
                            "  worst"
                        }
                    } else {
                        ""
                    }
                }
            } else {
                ""
            };

            let style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else if is_trimmed {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            let annotation_style = Style::default().fg(Color::DarkGray);

            lines.push(Line::from(vec![
                Span::styled(format!("  #{num}: "), Style::default().fg(Color::Blue)),
                Span::styled(
                    if is_trimmed {
                        format!("({time_display})")
                    } else {
                        time_display
                    },
                    style,
                ),
                Span::styled(annotation.to_string(), annotation_style),
            ]));
        }

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}

fn compute_trimmed_indices(times: &[(usize, Time)], mean_type: MeanType) -> Vec<usize> {
    if mean_type != MeanType::Ao5 || times.len() != 5 {
        return vec![];
    }

    let effective: Vec<Option<u64>> = times.iter().map(|(_, t)| t.effective_ms()).collect();
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
