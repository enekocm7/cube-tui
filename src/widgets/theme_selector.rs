use std::fs;

use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Widget};

use crate::model::keybinds::{Action, Keybinds};
use crate::model::settings::ThemeColors;
use crate::model::toast::{ToastBuffer, ToastDuration, ToastType};
use crate::persistence::themes_dir;
use crate::{model::settings::Theme, persistence::load_theme};

pub struct ThemeSelector {
    pub themes: Vec<Theme>,
    pub selection: usize,
    scroll_offset: usize,
}

impl ThemeSelector {
    /// Loads theme files and creates a selector at the first entry.
    pub fn new(toasts: &mut ToastBuffer) -> Self {
        let mut selector = Self {
            themes: Vec::new(),
            selection: 0,
            scroll_offset: 0,
        };
        let theme_path = match themes_dir() {
            Ok(path) => path,
            Err(error) => {
                toasts.push(format!("{error:#}"), ToastType::Error, ToastDuration::Long);
                return selector;
            }
        };
        let entries = match fs::read_dir(&theme_path) {
            Ok(entries) => entries,
            Err(error) => {
                toasts.push(
                    format!("Could not list themes: {error}"),
                    ToastType::Error,
                    ToastDuration::Long,
                );
                return selector;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    toasts.push(
                        format!("Could not read a theme entry: {error}"),
                        ToastType::Warning,
                        ToastDuration::Long,
                    );
                    continue;
                }
            };
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
            {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                toasts.push(
                    "Skipped a theme with an invalid filename",
                    ToastType::Warning,
                    ToastDuration::Long,
                );
                continue;
            };
            match load_theme(name) {
                Ok(colors) => selector.themes.push(Theme::new(name, colors)),
                Err(error) => toasts.push(
                    format!("{error:#}. Theme skipped."),
                    ToastType::Warning,
                    ToastDuration::Long,
                ),
            }
        }
        selector.themes.sort_by(|a, b| a.name().cmp(b.name()));
        if selector.themes.is_empty() {
            toasts.push(
                "No usable themes found. Add a TOML theme in the themes folder.",
                ToastType::Info,
                ToastDuration::Short,
            );
        }
        selector
    }

    /// Moves selection to the next available theme.
    pub fn next(&mut self) {
        if self.selection < self.themes.len().saturating_sub(1) {
            self.selection += 1;
        }
    }

    /// Moves selection to the previous available theme.
    pub fn previous(&mut self) {
        if self.selection > 0 {
            self.selection -= 1;
        }
    }

    /// Returns the currently selected theme.
    pub fn selected(&self) -> Option<&Theme> {
        self.themes.get(self.selection)
    }

    /// Renders the theme list and highlights its current selection.
    pub fn render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeColors,
        keybinds: &Keybinds,
    ) {
        let popup_width = u16::max(area.width / 2, 70);
        let popup_height = u16::max(area.height / 3, 20);

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
            .title("Themes")
            .borders(Borders::ALL)
            .bg(theme.background())
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(theme.border()));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Length(3), Fill(1), Length(1), Length(1)])
            .split(inner);

        let message = Paragraph::new("To create one go to the themes folder (cube --themes)")
            .fg(theme.text());
        message.render(chunks[0], buf);

        let list_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Length(1), Fill(1), Length(1)])
            .split(chunks[1]);

        let list_area = list_chunks[1];
        let visible_height = list_area.height as usize;

        if visible_height == 0 {
            self.scroll_offset = 0;
        } else {
            if self.selection < self.scroll_offset {
                self.scroll_offset = self.selection;
            } else if self.selection >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selection - visible_height + 1;
            }
            let max_offset = self.themes.len().saturating_sub(visible_height);
            self.scroll_offset = self.scroll_offset.min(max_offset);
        }

        let scroll_offset = self.scroll_offset;
        let end = (scroll_offset + visible_height).min(self.themes.len());

        if scroll_offset > 0 {
            let indicator = Paragraph::new(format!("↑ {scroll_offset} more")).fg(theme.text());
            indicator.render(list_chunks[0], buf);
        }

        for (i, theme_item) in self.themes[scroll_offset..end].iter().enumerate() {
            let absolute_index = scroll_offset + i;
            let is_selected = absolute_index == self.selection;
            let item_area = Rect::new(list_area.x, list_area.y + i as u16, list_area.width, 1);

            if is_selected {
                let styled = Paragraph::new(theme_item.name())
                    .bg(theme.selection())
                    .fg(theme.selection_text());
                styled.render(item_area, buf);
            } else {
                let styled = Paragraph::new(theme_item.name()).fg(theme.text());
                styled.render(item_area, buf);
            }
        }

        let below = self.themes.len() - end;
        if below > 0 {
            let indicator = Paragraph::new(format!("↓ {below} more")).fg(theme.text());
            indicator.render(list_chunks[2], buf);
        }

        let help_text = Paragraph::new(format!(
            "{}: Close this window | {}: Open in default editor",
            keybinds.label(Action::Back),
            keybinds.label(Action::NextEvent)
        ))
        .fg(theme.text());
        help_text.render(chunks[3], buf);
    }
}
