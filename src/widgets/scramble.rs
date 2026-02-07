use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

pub struct Scramble<'a> {
    text: &'a str,
}

impl<'a> Scramble<'a> {
    pub const fn new(text: &'a str) -> Self {
        Self { text }
    }
}

impl Widget for Scramble<'_> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::default().title("Scramble").borders(Borders::ALL);
        let line = Line::from(Span::styled(
            self.text,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        Paragraph::new(line)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
