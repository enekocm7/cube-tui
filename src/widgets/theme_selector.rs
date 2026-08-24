use std::fs;

use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::Percentage;
use ratatui::layout::{Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Widget};

use crate::model::settings::ThemeColors;
use crate::persistence::themes_dir;
use crate::{model::settings::Theme, persistence::load_theme};

pub struct ThemeSelector {
    themes: Vec<Theme>,
    selection: usize,
}

impl ThemeSelector {
    pub fn new() -> Self {
        if let Some(theme_path) = themes_dir() {
            let mut themes: Vec<Theme> = Vec::new();
            for entry in fs::read_dir(theme_path)
                .expect("Should not fail if `themes_dir` creates the dir already")
            {
                let entry = entry.expect("Should not fail to read the entries");
                let entry_name = entry.file_name();
                let entry_name = entry_name.to_str().unwrap_or("default.toml");
                let theme = load_theme(entry_name);
                if let Some(theme) = theme {
                    themes.push(Theme::new(entry_name, theme));
                }
            }
            return Self {
                themes,
                selection: 0,
            };
        }
        Self {
            themes: Vec::new(),
            selection: 0,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
        let popup_width = u16::max(area.width / 2, 50);
        let popup_height = u16::max(area.height / 2, 50);

        let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(
            x,
            y,
            popup_width.min(area.width - 2),
            popup_height.min(area.height - 2),
        );

        Widget::render(Clear, popup_area, buf);

        let block = Block::default()
            .title("Themes")
            .borders(Borders::ALL)
            .bg(theme.background())
            .padding(Padding::uniform(5))
            .border_style(Style::default().fg(theme.border()));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Percentage(20), Percentage(80)])
            .split(inner);

        let message = Paragraph::new(
            "Select the themes, to create one go to the themes folder (cube --themes)",
        )
        .fg(theme.text());
        message.render(chunks[0], buf);
        
    }
}
