use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::prelude::Text;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap};

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
}

impl Widget for ScrambleWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .padding(Padding::new(5, 5, 0, 0));
        let style = Style::default()
            .fg(Color::Yellow)
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
