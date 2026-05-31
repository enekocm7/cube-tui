use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::prelude::Text;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap};

use crate::model::settings::ThemeSettings;

pub struct ScrambleWidget<'a> {
    text: &'a str,
    title: String,
}

impl<'a> ScrambleWidget<'a> {
    pub fn new(text: &'a str, event_name: &str) -> Self {
        Self {
            text,
            title: format!("Scramble ({event_name})"),
        }
    }

    pub fn render_with_theme(self, area: Rect, buf: &mut Buffer, theme: &ThemeSettings) {
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
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
