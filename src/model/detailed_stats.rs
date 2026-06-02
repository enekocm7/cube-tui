use crate::{model::Model, widgets::history::Time};

#[derive(Default)]
pub struct DetailedStatsState {
    pub show: bool,
    pub row: usize,
    pub col: usize,
    pub show_mean_detail: bool,
    pub mean_detail_selected_index: usize,
    pub opened_from_stats_column: bool,
}

impl Model {
    pub const fn show_detailed_stats(&self) -> bool {
        self.detailed_stats_state.show
    }

    pub fn open_detailed_stats(&mut self) {
        if self.history().is_empty() {
            return;
        }
        self.detailed_stats_state.show = true;
        self.detailed_stats_state.row = self.history().len().saturating_sub(1);
        self.detailed_stats_state.col = 0;
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
        self.detailed_stats_state.opened_from_stats_column = false;
    }

    pub const fn close_detailed_stats(&mut self) {
        self.detailed_stats_state.show = false;
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
        self.detailed_stats_state.opened_from_stats_column = false;
    }

    pub const fn detailed_stats_row(&self) -> usize {
        self.detailed_stats_state.row
    }

    pub const fn detailed_stats_col(&self) -> usize {
        self.detailed_stats_state.col
    }

    pub const fn detailed_stats_select_up(&mut self) {
        self.detailed_stats_state.row = self.detailed_stats_state.row.saturating_sub(1);
    }

    pub fn detailed_stats_select_down(&mut self) {
        let max = self.history().len().saturating_sub(1);
        self.detailed_stats_state.row = (self.detailed_stats_state.row + 1).min(max);
    }

    pub const fn detailed_stats_col_left(&mut self) {
        self.detailed_stats_state.col = 0;
    }

    pub const fn detailed_stats_col_right(&mut self) {
        self.detailed_stats_state.col = 1;
    }

    pub const fn show_mean_detail(&self) -> bool {
        self.detailed_stats_state.show_mean_detail
    }

    pub fn open_mean_detail(&mut self) {
        let row = self.detailed_stats_state.row;
        let col = self.detailed_stats_state.col;
        let has_mean = if col == 0 {
            self.history().mo3_at(row).is_some()
        } else {
            self.history().ao5_at(row).is_some()
        };
        if has_mean {
            self.detailed_stats_state.show_mean_detail = true;
            self.detailed_stats_state.mean_detail_selected_index = 0;
            self.detailed_stats_state.opened_from_stats_column = false;
        }
    }

    pub fn open_mean_detail_from_stats(&mut self) -> bool {
        let row = self.main_stats_selection.row;
        let col = self.main_stats_selection.col;

        if row == 0 {
            let solve_index = if col == 0 {
                self.history().len().checked_sub(1)
            } else {
                self.history()
                    .times()
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, t)| t.raw_ms())
                    .map(|(i, _)| i)
            };

            let Some(solve_index) = solve_index else {
                return false;
            };

            self.history_mut().select_index(solve_index);
            self.open_details();
            return true;
        }

        let solve_index = match (row, col) {
            (1, 0) => self.history().latest_mo3_index(),
            (1, 1) => self.history().fastest_mo3_index(),
            (2, 0) => self.history().latest_ao5_index(),
            (2, 1) => self.history().fastest_ao5_index(),
            _ => None,
        };

        let Some(solve_index) = solve_index else {
            return false;
        };

        self.detailed_stats_state.show = true;
        self.detailed_stats_state.row = solve_index;
        self.detailed_stats_state.col = usize::from(row != 1);
        self.detailed_stats_state.show_mean_detail = true;
        self.detailed_stats_state.mean_detail_selected_index = 0;
        self.detailed_stats_state.opened_from_stats_column = true;
        true
    }

    pub const fn close_mean_detail(&mut self) {
        let opened_from_stats_column = self.detailed_stats_state.opened_from_stats_column;
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
        self.detailed_stats_state.opened_from_stats_column = false;
        if opened_from_stats_column {
            self.detailed_stats_state.show = false;
        }
    }

    pub fn mean_detail_times_len(&self) -> usize {
        let row = self.detailed_stats_state.row;
        if self.detailed_stats_state.col == 0 {
            self.history().mo3_times_at(row).map_or(0, <[Time]>::len)
        } else {
            self.history().ao5_times_at(row).map_or(0, <[Time]>::len)
        }
    }

    pub const fn mean_detail_selected_index(&self) -> usize {
        self.detailed_stats_state.mean_detail_selected_index
    }

    pub const fn mean_detail_select_up(&mut self) {
        self.detailed_stats_state.mean_detail_selected_index = self
            .detailed_stats_state
            .mean_detail_selected_index
            .saturating_sub(1);
    }

    pub fn mean_detail_select_down(&mut self) {
        let max = self.mean_detail_times_len().saturating_sub(1);
        self.detailed_stats_state.mean_detail_selected_index = self
            .detailed_stats_state
            .mean_detail_selected_index
            .saturating_add(1)
            .min(max);
    }
}
