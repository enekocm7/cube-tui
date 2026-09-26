use crate::{
    model::{Model, settings::Theme},
    persistence,
    widgets::theme_selector::ThemeSelector,
};

impl Model {
    /// Loads available themes and opens the selector at the active theme.
    pub fn open_theme_selector(&mut self) {
        let mut theme_selector = ThemeSelector::new(&mut self.toasts);
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
        self.save_settings();
    }

    /// Opens the selected theme file in the system editor.
    pub fn open_theme_in_editor(&mut self) {
        let Some(selector) = &self.theme_selector else {
            return;
        };
        let Some(theme) = selector.selected() else {
            self.toast_info("Select a theme before opening it in the editor.");
            return;
        };
        let dir = match persistence::themes_dir() {
            Ok(dir) => dir,
            Err(error) => {
                self.toast_error(format!("{error:#}"));
                return;
            }
        };
        let path = dir.join(theme.name());
        if let Err(error) = open::that(&path) {
            self.toast_error(format!("Could not open theme {}: {error}", path.display()));
        }
    }
}
