use crate::model::Model;

#[derive(Default)]
pub struct HelpState {
    pub show: bool,
    pub scroll: u16,
    pub max_scroll: u16,
}

impl Model {
    pub const fn show_help(&self) -> bool {
        self.help_state.show
    }

    pub const fn toggle_help(&mut self) {
        self.help_state.show = !self.help_state.show;
        if self.help_state.show {
            self.help_state.scroll = 0;
        }
    }

    pub const fn help_scroll(&self) -> u16 {
        self.help_state.scroll
    }

    pub fn set_help_max_scroll(&mut self, max_scroll: u16) {
        self.help_state.max_scroll = max_scroll;
        self.help_state.scroll = self.help_state.scroll.min(self.help_state.max_scroll);
    }

    pub const fn scroll_help_up(&mut self) {
        self.help_state.scroll = self.help_state.scroll.saturating_sub(1);
    }

    pub fn scroll_help_down(&mut self) {
        self.help_state.scroll = self
            .help_state
            .scroll
            .saturating_add(1)
            .min(self.help_state.max_scroll);
    }
}
