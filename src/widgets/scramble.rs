use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::prelude::Text;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap};

use crate::model::settings::ThemeColors;

pub struct ScrambleWidget<'a> {
    text: &'a str,
    title: String,
}

impl<'a> ScrambleWidget<'a> {
    /// Creates a scramble widget labelled with its event.
    pub fn new(text: &'a str, event_name: &str) -> Self {
        Self {
            text,
            title: format!("WCA Scramble ({event_name})"),
        }
    }

    /// Renders wrapped scramble notation using the active theme.
    pub fn render_with_theme(self, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .padding(Padding::new(5, 5, 0, 0));
        let style = Style::default()
            .fg(theme.scramble())
            .add_modifier(Modifier::BOLD);
        let text: Text = self
            .text
            .split('\n')
            .map(|row| Line::from(Span::styled(row.to_string(), style)))
            .collect();
        // Megaminx scrambles come as pre-split rows: left-align them inside a
        // centered block so every U move lines up in one column (U' sticks out).
        if self.text.contains('\n') {
            let inner = block.inner(area);
            block.render(area, buf);
            let block_width = self
                .text
                .split('\n')
                .map(|row| row.chars().count())
                .max()
                .unwrap_or(0);
            let block_width = u16::try_from(block_width).unwrap_or(u16::MAX);
            let width = block_width.min(inner.width);
            let x = inner.x + (inner.width - width) / 2;
            Paragraph::new(text)
                .alignment(Alignment::Left)
                .render(Rect::new(x, inner.y, width, inner.height), buf);
            return;
        }
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
