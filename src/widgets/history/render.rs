use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::settings::ThemeColors;

use super::History;

impl History {
    /// Renders the scrollable history list with an optional selection highlight.
    pub fn render_with_theme(
        &self,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeColors,
        highlight: Option<bool>,
    ) {
        let highlight = highlight.unwrap_or(true);
        let total = self.times.len();
        let height = area.height as usize;
        let selected = self.selected.unwrap_or(0);

        let scroll_full = selected.saturating_sub(height.saturating_sub(1));
        let need_below = scroll_full + height < total;

        let bot_rows = usize::from(need_below);
        let items_height = height.saturating_sub(bot_rows);

        let scroll_offset = selected.saturating_sub(items_height.saturating_sub(1));

        for (i, item) in self.times.iter().enumerate().skip(scroll_offset) {
            let display_row = i - scroll_offset;
            if display_row >= items_height {
                break;
            }
            let Ok(row_offset) = u16::try_from(display_row) else {
                break;
            };
            let style = if highlight && self.selected.is_some() && i == selected {
                ratatui::style::Style::default()
                    .bg(theme.selection())
                    .fg(theme.selection_text())
            } else {
                ratatui::style::Style::default().fg(theme.text())
            };
            let prefix = format!("{}: ", i + 1);
            let (time_x, _) = buf.set_stringn(
                area.x,
                area.y + row_offset,
                &prefix,
                usize::from(area.width),
                style,
            );
            let is_record = item.was_single_record();
            let time_style = if is_record {
                style
                    .fg(theme.accent())
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                style
            };
            buf.set_stringn(
                time_x,
                area.y + row_offset,
                item.to_string(),
                usize::from(area.right().saturating_sub(time_x)),
                time_style,
            );
        }

        if need_below {
            let below_count = total.saturating_sub(scroll_offset + items_height);
            buf.set_string(
                area.x,
                area.y + area.height - 1,
                format!("↓ {below_count} more"),
                ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
            );
        }
    }
}
