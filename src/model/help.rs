use crate::model::Model;

#[derive(Default)]
pub struct HelpState {
    pub show: bool,
    pub scroll: u16,
    pub max_scroll: u16,
}

impl Model {
    /// Returns whether the help overlay is visible.
    pub const fn show_help(&self) -> bool {
        self.help_state.show
    }

    /// Shows or hides help, resetting scroll when it opens.
    pub const fn toggle_help(&mut self) {
        self.help_state.show = !self.help_state.show;
        if self.help_state.show {
            self.help_state.scroll = 0;
        }
    }

    /// Returns the help overlay's current line offset.
    pub const fn help_scroll(&self) -> u16 {
        self.help_state.scroll
    }

    /// Updates the maximum offset and clamps the current scroll position.
    pub fn set_help_max_scroll(&mut self, max_scroll: u16) {
        self.help_state.max_scroll = max_scroll;
        self.help_state.scroll = self.help_state.scroll.min(self.help_state.max_scroll);
    }

    /// Scrolls help up by one line.
    pub const fn scroll_help_up(&mut self) {
        self.help_state.scroll = self.help_state.scroll.saturating_sub(1);
    }

    /// Scrolls help down by one line without passing the content end.
    pub fn scroll_help_down(&mut self) {
        self.help_state.scroll = self
            .help_state
            .scroll
            .saturating_add(1)
            .min(self.help_state.max_scroll);
    }
}
