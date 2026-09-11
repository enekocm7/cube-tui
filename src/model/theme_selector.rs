use crate::{
    model::{Model, settings::Theme},
    persistence,
    widgets::theme_selector::ThemeSelector,
};

impl Model {
    /// Loads available themes and opens the selector at the active theme.
    pub fn open_theme_selector(&mut self) {
        let mut theme_selector = ThemeSelector::new();
        let actual_theme_name = self.settings().theme_name();
        theme_selector
            .themes
            .iter()
            .enumerate()
            .for_each(|(i, theme)| {
                if theme.name() == actual_theme_name {
                    theme_selector.selection = i;
                }
            });
        self.theme_selector = Some(theme_selector);
    }

    /// Closes the theme selector.
    pub fn close_theme_selector(&mut self) {
        self.theme_selector = None;
    }

    /// Moves to and previews the previous theme.
    pub fn theme_selector_up(&mut self) {
        if let Some(theme_selector) = &mut self.theme_selector {
            let before = theme_selector.selection;
            theme_selector.previous();
            if theme_selector.selection != before
                && let Some(theme) = theme_selector.selected().cloned()
            {
                self.apply_theme(&theme);
            }
        }
    }

    /// Moves to and previews the next theme.
    pub fn theme_selector_down(&mut self) {
        if let Some(theme_selector) = &mut self.theme_selector {
            let before = theme_selector.selection;
            theme_selector.next();
            if theme_selector.selection != before
                && let Some(theme) = theme_selector.selected().cloned()
            {
                self.apply_theme(&theme);
            }
        }
    }

    /// Applies a theme and persists the updated setting.
    fn apply_theme(&mut self, theme: &Theme) {
        self.settings.set_theme(theme);
        persistence::save_config(self.settings());
    }

    /// Opens the selected theme file in the system editor.
    pub fn open_theme_in_editor(&self) {
        if let Some(theme_selector) = &self.theme_selector
            && let Some(theme) = theme_selector.selected()
        {
            let name = theme.name();
            let path = persistence::themes_dir()
                .expect("Shouldn't fail to get the themes dir")
                .join(name);
            open::that(path).unwrap_or_else(|_| eprintln!("Failed to open theme path"));
        }
    }
}
