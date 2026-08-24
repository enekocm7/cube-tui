use crate::{
    model::{Model, settings::Theme},
    persistence,
    widgets::theme_selector::ThemeSelector,
};

impl Model {
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

    pub fn clone_theme_selector(&mut self) {
        self.theme_selector = None;
    }

    pub fn theme_selector_up(&mut self) {
        if let Some(theme_selector) = &mut self.theme_selector {
            theme_selector.previous();
            let theme = theme_selector.selected().cloned().unwrap_or_default();
            self.apply_theme(&theme);
        }
    }

    pub fn theme_selector_down(&mut self) {
        if let Some(theme_selector) = &mut self.theme_selector {
            theme_selector.next();
            let theme = theme_selector.selected().cloned().unwrap_or_default();
            self.apply_theme(&theme);
        }
    }

    fn apply_theme(&mut self, theme: &Theme) {
        self.settings.set_theme(theme);
        persistence::save_config(self.settings());
    }
}
