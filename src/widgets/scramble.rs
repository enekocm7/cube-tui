use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

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
            .borders(Borders::ALL);
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
