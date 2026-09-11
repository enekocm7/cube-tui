use crate::model::Model;
use crate::model::screen::{DetailsReturn, Screen};
use crate::widgets::history::{Modifier, Time};

impl Model {
    /// Returns whether the solve-details dialog is visible.
    pub const fn show_details(&self) -> bool {
        self.screen.show_details()
    }

    /// Opens details for the selected solve and initializes its modifier row.
    pub fn open_details(&mut self) {
        let modifier_index = match self.history().selected_time().map(Time::modifier) {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
        self.screen = Screen::Details {
            modifier_index,
            return_to: DetailsReturn::Main,
        };
    }

    /// Closes the current screen and restores its recorded parent screen.
    pub const fn close_current_screen(&mut self) {
        let new_screen = match &self.screen {
            Screen::Details { return_to, .. } => match return_to {
                DetailsReturn::Main => Screen::Main,
                DetailsReturn::MeanDetail {
                    row,
                    col,
                    selected_index,
                    from_stats_column,
                } => Screen::MeanDetail {
                    row: *row,
                    col: *col,
                    selected_index: *selected_index,
                    from_stats_column: *from_stats_column,
                },
            },
            Screen::Main | Screen::DetailedStats { .. } => Screen::Main,
            Screen::MeanDetail {
                row,
                col,
                from_stats_column,
                ..
            } => {
                if *from_stats_column {
                    Screen::Main
                } else {
                    Screen::DetailedStats {
                        row: *row,
                        col: *col,
                    }
                }
            }
        };
        self.screen = new_screen;
    }

    /// Selects the next modifier option in the details dialog.
    pub fn next_details_modifier(&mut self) {
        if let Screen::Details { modifier_index, .. } = &mut self.screen {
            *modifier_index = (*modifier_index + 1).min(1);
        }
    }

    /// Selects the previous modifier option in the details dialog.
    pub const fn prev_details_modifier(&mut self) {
        if let Screen::Details { modifier_index, .. } = &mut self.screen {
            *modifier_index = modifier_index.saturating_sub(1);
        }
    }

    /// Returns the selected modifier option's zero-based index.
    pub const fn selected_details_modifier_index(&self) -> usize {
        if let Screen::Details { modifier_index, .. } = &self.screen {
            *modifier_index
        } else {
            0
        }
    }

    /// Returns the modifier represented by the current dialog selection.
    pub const fn selected_details_modifier(&self) -> Modifier {
        if let Screen::Details { modifier_index, .. } = &self.screen {
            if *modifier_index == 0 {
                Modifier::PlusTwo
            } else {
                Modifier::DNF
            }
        } else {
            Modifier::PlusTwo
        }
    }

    /// Selects the previous solve and synchronizes the modifier choice.
    pub fn details_nav_prev(&mut self) {
        self.history_mut().select_previous();
        self.sync_details_modifier();
    }

    /// Selects the next solve and synchronizes the modifier choice.
    pub fn details_nav_next(&mut self) {
        self.history_mut().select_next();
        self.sync_details_modifier();
    }

    /// Updates the dialog selection to match the underlying solve modifier.
    fn sync_details_modifier(&mut self) {
        let new_index = match self.history().selected_time().map(Time::modifier) {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
        if let Screen::Details { modifier_index, .. } = &mut self.screen {
            *modifier_index = new_index;
        }
    }
}
