use crate::model::Model;

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
}
