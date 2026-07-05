use std::borrow::Cow;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};

use crate::model::settings::ThemeSettings;
use crate::widgets::history::{History, Time};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeanType {
    Mo3,
    Ao5,
    Ao12,
    Ao50,
    Ao100,
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
        let (mean_type, mean_value, times, times_start_index) = match col {
            0 => {
                let val = history.mo3_at(solve_index).unwrap_or(Cow::Borrowed("-"));
                let times = history.mo3_times_at(solve_index).unwrap_or(&[]);
                (MeanType::Mo3, val, times, solve_index.saturating_sub(2))
            }
            1 => {
                let val = history.ao5_at(solve_index).unwrap_or(Cow::Borrowed("-"));
                let times = history.ao5_times_at(solve_index).unwrap_or(&[]);
                (MeanType::Ao5, val, times, solve_index.saturating_sub(4))
            }
            2 => {
                let val = history.ao12_at(solve_index).unwrap_or(Cow::Borrowed("-"));
                let times = history.ao12_times_at(solve_index).unwrap_or(&[]);
                (MeanType::Ao12, val, times, solve_index.saturating_sub(11))
            }
            3 => {
                let val = history.ao50_at(solve_index).unwrap_or(Cow::Borrowed("-"));
                let times = history.ao50_times_at(solve_index).unwrap_or(&[]);
                (MeanType::Ao50, val, times, solve_index.saturating_sub(49))
            }
            4 => {
                let val = history.ao100_at(solve_index).unwrap_or(Cow::Borrowed("-"));
                let times = history.ao100_times_at(solve_index).unwrap_or(&[]);
                (MeanType::Ao100, val, times, solve_index.saturating_sub(99))
            }
            _ => (
                MeanType::Mo3,
                Cow::Borrowed("-"),
                &[] as &[Time],
                solve_index.saturating_sub(2),
            ),
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
            MeanType::Ao12 => "Average of 12",
            MeanType::Ao50 => "Average of 50",
            MeanType::Ao100 => "Average of 100",
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

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 4 || inner.width < 10 {
            return;
        }

        let header_lines: u16 = 3;
        let avail = (inner.height as usize).saturating_sub(header_lines as usize);
        if avail == 0 {
            return;
        }

        let total = self.times.len();
        let (items_height, scroll_offset, has_above, has_below) = if total <= avail {
            (total, 0, false, false)
        } else if self.selected_index < avail - 1 {
            (avail - 1, 0, false, true)
        } else {
            let h = avail - 2;
            let off = self.selected_index.saturating_sub(h - 1).min(total - h);
            if off + h >= total {
                let h2 = avail - 1;
                let off2 = total - h2;
                (h2, off2, true, false)
            } else {
                (h, off, off > 0, off + h < total)
            }
        };

        let mut y = inner.y;
        buf.set_string(
            inner.x,
            y,
            format!("{type_name}: {}", self.mean_value),
            Style::default()
                .fg(theme.text())
                .add_modifier(Modifier::BOLD),
        );
        y += 1;
        y += 1;
        buf.set_string(
            inner.x,
            y,
            "Times:",
            Style::default()
                .fg(theme.text())
                .add_modifier(Modifier::BOLD),
        );
        y += 1;

        if has_above {
            buf.set_string(
                inner.x,
                y,
                format!("↑ {scroll_offset} more"),
                Style::default().fg(theme.text()),
            );
            y += 1;
        }

        let trimmed = compute_trimmed_indices(self.times, self.mean_type);
        let best_ms = self.times.iter().filter_map(Time::effective_ms).min();

        for display_idx in 0..items_height {
            let window_idx = scroll_offset + display_idx;
            let time = &self.times[window_idx];
            let history_index = self.times_start_index + window_idx + 1;
            let is_trimmed = trimmed.contains(&window_idx);
            let is_selected = window_idx == self.selected_index;
            let time_display = time.to_string();

            let annotation: Cow<'static, str> = if is_trimmed {
                if time.effective_ms().is_none() {
                    Cow::Borrowed(" <- worst (trimmed)")
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

            Line::from(vec![
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
                Span::styled(annotation, Style::default().fg(theme.text())),
            ])
            .render(Rect::new(inner.x, y, inner.width, 1), buf);
            y += 1;
        }

        if has_below {
            let below = total - (scroll_offset + items_height);
            buf.set_string(
                inner.x,
                y,
                format!("↓ {below} more"),
                Style::default().fg(theme.text()),
            );
        }
    }
}

fn compute_trimmed_indices(times: &[Time], mean_type: MeanType) -> Vec<usize> {
    let n = match mean_type {
        MeanType::Mo3 => return vec![],
        MeanType::Ao5 => 5,
        MeanType::Ao12 => 12,
        MeanType::Ao50 => 50,
        MeanType::Ao100 => 100,
    };

    if times.len() != n {
        return vec![];
    }

    let effective: Vec<Option<u64>> = times.iter().map(Time::effective_ms).collect();
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
            trimmed.push(n - 1);
        } else {
            trimmed.push(best_idx);
            trimmed.push(worst_idx);
        }
    }

    trimmed
}
