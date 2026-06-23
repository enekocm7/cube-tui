use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget},
};

use crate::model::settings::ThemeSettings;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    #[default]
    No,
    Yes,
}

pub struct ConfirmationWidget<'a> {
    pub message: &'a str,
    selection: Selection,
}

impl<'a> ConfirmationWidget<'a> {
    pub const fn new(message: &'a str, selection: Selection) -> Self {
        Self { message, selection }
    }

    pub fn render_with_theme(self, area: Rect, buf: &mut Buffer, theme: &ThemeSettings) {
        let popup_width = u16::max(area.width / 4, 30);
        let popup_height = u16::max(area.height / 5, 7);

        let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(
            x,
            y,
            popup_width.min(area.width),
            popup_height.min(area.height),
        );

        Widget::render(Clear, popup_area, buf);

        let block = Block::default()
            .title("Are you sure?")
            .borders(Borders::ALL)
            .bg(theme.background())
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(theme.border()));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(1),
            ])
            .split(inner);

        let message = Paragraph::new(self.message).style(Style::default().fg(theme.text()));
        message.render(chunks[0], buf);

        let button_area = chunks[2];
        let buttons = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(button_area);

        let selected_style = Style::default()
            .bg(theme.selection())
            .fg(theme.selection_text());

        let unselected_style = Style::default().fg(theme.text());

        let no_style = if self.selection == Selection::No {
            selected_style
        } else {
            unselected_style
        };

        let yes_style = if self.selection == Selection::Yes {
            selected_style
        } else {
            unselected_style
        };

        let no_paragraph = Paragraph::new("No")
            .alignment(Alignment::Center)
            .style(no_style);
        no_paragraph.render(buttons[0], buf);

        let yes_paragraph = Paragraph::new("Yes")
            .alignment(Alignment::Center)
            .style(yes_style);
        yes_paragraph.render(buttons[1], buf);
    }
}
