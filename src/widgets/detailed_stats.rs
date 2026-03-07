use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::widgets::history::History;

pub struct DetailedStatsWidget {
    history: History,
    selected_row: usize,
    selected_col: usize,
}

impl DetailedStatsWidget {
    pub const fn new(history: History, selected_row: usize, selected_col: usize) -> Self {
        Self {
            history,
            selected_row,
            selected_col,
        }
    }
}

impl Widget for DetailedStatsWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::default()
            .title("Detailed Stats (Enter: view mean, ←/→: mo3/ao5, Esc: back)")
            .borders(Borders::ALL);

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

        let header = format!(" {:>6}  {:>12}  {:>12}  {:>12}", "#", "Time", "mo3", "ao5");
        let header_style = Style::default()
            .fg(Color::Cyan)
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
                .map_or_else(|| "-".to_string(), ToString::to_string);
            let mo3_str = self
                .history
                .mo3_at(solve_idx)
                .unwrap_or_else(|| "-".to_string());
            let ao5_str = self
                .history
                .ao5_at(solve_idx)
                .unwrap_or_else(|| "-".to_string());

            let is_selected = solve_idx == self.selected_row;

            let num_str = format!(" {:>6}", solve_idx + 1);
            buf.set_string(
                inner.x,
                inner.y + row_offset,
                &num_str,
                row_style(is_selected, false),
            );

            let time_col = format!("  {:>12}", truncate(&time_str, 12));
            buf.set_string(
                inner.x + 8,
                inner.y + row_offset,
                &time_col,
                row_style(is_selected, false),
            );

            let mo3_col = format!("  {:>12}", truncate(&mo3_str, 12));
            buf.set_string(
                inner.x + 22,
                inner.y + row_offset,
                &mo3_col,
                row_style(is_selected, is_selected && self.selected_col == 0),
            );

            let ao5_col = format!("  {:>12}", truncate(&ao5_str, 12));
            buf.set_string(
                inner.x + 36,
                inner.y + row_offset,
                &ao5_col,
                row_style(is_selected, is_selected && self.selected_col == 1),
            );
        }

        if scroll_offset > 0 {
            buf.set_string(
                inner.x,
                inner.y + 1,
                format!("↑ {scroll_offset} more"),
                Style::default().fg(Color::DarkGray),
            );
        }
        let visible_end = scroll_offset + items_height;
        if visible_end < total {
            let below = total - visible_end;
            buf.set_string(
                inner.x,
                inner.y + inner.height - 1,
                format!("↓ {below} more"),
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}

fn row_style(is_row_selected: bool, is_cell_highlighted: bool) -> Style {
    if is_cell_highlighted {
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else if is_row_selected {
        Style::default().bg(Color::Blue).fg(Color::Black)
    } else {
        Style::default()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        s[..max].to_string()
    }
}
