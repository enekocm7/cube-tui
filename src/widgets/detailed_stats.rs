use std::borrow::Cow;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::model::settings::ThemeSettings;
use crate::widgets::history::History;

pub struct DetailedStatsWidget<'a> {
    history: &'a History,
    selected_row: usize,
    selected_col: usize,
}

impl<'a> DetailedStatsWidget<'a> {
    pub const fn new(history: &'a History, selected_row: usize, selected_col: usize) -> Self {
        Self {
            history,
            selected_row,
            selected_col,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeSettings) {
        let block = Block::default()
            .title("Detailed Stats (Enter: view mean, ←/→: navigate, Esc: back)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 2 || inner.width < 10 {
            return;
        }

        let total = self.history.len();
        if total == 0 {
            buf.set_string(inner.x, inner.y, "No solves yet.", Style::default());
            return;
        }

        let header = format!(
            " {:>6}  {:>12}  {:>12}  {:>12}  {:>12}  {:>12}  {:>12}",
            "#", "Time", "mo3", "ao5", "ao12", "ao50", "ao100"
        );
        let header_style = Style::default()
            .fg(theme.text())
            .add_modifier(Modifier::BOLD);
        buf.set_string(inner.x, inner.y, &header, header_style);

        let items_height = (inner.height as usize).saturating_sub(1);
        if items_height == 0 {
            return;
        }

        let scroll_offset = if self.selected_row >= items_height {
            self.selected_row - items_height + 1
        } else {
            0
        };

        for display_idx in 0..items_height {
            let solve_idx = scroll_offset + display_idx;
            if solve_idx >= total {
                break;
            }
            let Ok(row_offset) = u16::try_from(display_idx + 1) else {
                break;
            };

            let time_str = self
                .history
                .get_time_at(solve_idx)
                .map_or_else(|| Cow::Borrowed("-"), |t| Cow::Owned(t.to_string()));
            let mo3_str = self.history.mo3_at(solve_idx).unwrap_or(Cow::Borrowed("-"));
            let ao5_str = self.history.ao5_at(solve_idx).unwrap_or(Cow::Borrowed("-"));
            let ao12_str = self
                .history
                .ao12_at(solve_idx)
                .unwrap_or(Cow::Borrowed("-"));
            let ao50_str = self
                .history
                .ao50_at(solve_idx)
                .unwrap_or(Cow::Borrowed("-"));
            let ao100_str = self
                .history
                .ao100_at(solve_idx)
                .unwrap_or(Cow::Borrowed("-"));

            let is_selected = solve_idx == self.selected_row;

            let num_str = format!(" {:>6}", solve_idx + 1);
            buf.set_string(
                inner.x,
                inner.y + row_offset,
                &num_str,
                row_style(is_selected, false, theme),
            );

            let time_col = format!("  {:>12}", truncate(&time_str, 12));
            buf.set_string(
                inner.x + 8,
                inner.y + row_offset,
                &time_col,
                row_style(is_selected, false, theme),
            );

            let mo3_col = format!("  {:>12}", truncate(&mo3_str, 12));
            buf.set_string(
                inner.x + 22,
                inner.y + row_offset,
                &mo3_col,
                row_style(is_selected, is_selected && self.selected_col == 0, theme),
            );

            let ao5_col = format!("  {:>12}", truncate(&ao5_str, 12));
            buf.set_string(
                inner.x + 36,
                inner.y + row_offset,
                &ao5_col,
                row_style(is_selected, is_selected && self.selected_col == 1, theme),
            );

            let ao12_col = format!("  {:>12}", truncate(&ao12_str, 12));
            buf.set_string(
                inner.x + 50,
                inner.y + row_offset,
                &ao12_col,
                row_style(is_selected, is_selected && self.selected_col == 2, theme),
            );

            let ao50_col = format!("  {:>12}", truncate(&ao50_str, 12));
            buf.set_string(
                inner.x + 64,
                inner.y + row_offset,
                &ao50_col,
                row_style(is_selected, is_selected && self.selected_col == 3, theme),
            );

            let ao100_col = format!("  {:>12}", truncate(&ao100_str, 12));
            buf.set_string(
                inner.x + 78,
                inner.y + row_offset,
                &ao100_col,
                row_style(is_selected, is_selected && self.selected_col == 4, theme),
            );
        }

        if scroll_offset > 0 {
            buf.set_string(
                inner.x,
                inner.y + 1,
                format!("↑ {scroll_offset} more"),
                Style::default().fg(theme.text()),
            );
        }
        let visible_end = scroll_offset + items_height;
        if visible_end < total {
            let below = total - visible_end;
            buf.set_string(
                inner.x,
                inner.y + inner.height - 1,
                format!("↓ {below} more"),
                Style::default().fg(theme.text()),
            );
        }
    }
}

fn row_style(is_row_selected: bool, is_cell_highlighted: bool, theme: &ThemeSettings) -> Style {
    if is_cell_highlighted {
        Style::default()
            .bg(Color::Yellow)
            .fg(theme.selection_text())
            .add_modifier(Modifier::BOLD)
    } else if is_row_selected {
        Style::default()
            .bg(theme.selection())
            .fg(theme.selection_text())
    } else {
        Style::default().fg(theme.text())
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        s[..max].to_string()
    }
}
