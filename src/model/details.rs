use crate::model::Model;
use crate::model::main_focus::MainFocus;
use crate::widgets::history::{Modifier, Time};

#[derive(Default)]
pub struct DetailsState {
    pub show: bool,
    pub modifier_index: usize,
    pub opened_from_stats_col: bool,
}

impl Model {
    pub const fn show_details(&self) -> bool {
        self.details_state.show
    }

    pub fn open_details(&mut self) {
        self.details_state.show = true;
        self.details_state.opened_from_stats_col =
            self.detailed_stats_state.opened_from_stats_column;
        self.details_state.modifier_index = match self.history().selected_time().map(Time::modifier)
        {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
    }

    pub const fn close_details(&mut self) {
        self.details_state.show = false;
    }

    pub const fn return_to_stats_column(&mut self) -> bool {
        self.details_state.show = false;
        self.main_focus = MainFocus::Stats;
        true
    }

    pub const fn return_to_mean_detail(&mut self) -> bool {
        self.details_state.show = false;
        self.detailed_stats_state.show = true;
        self.detailed_stats_state.show_mean_detail = true;
        self.detailed_stats_state.opened_from_stats_column =
            self.details_state.opened_from_stats_col;
        true
    }

    pub fn next_details_modifier(&mut self) {
        self.details_state.modifier_index = (self.details_state.modifier_index + 1).min(1);
    }

    pub const fn prev_details_modifier(&mut self) {
        self.details_state.modifier_index = self.details_state.modifier_index.saturating_sub(1);
    }

    pub const fn selected_details_modifier_index(&self) -> usize {
        self.details_state.modifier_index
    }

    pub const fn selected_details_modifier(&self) -> Modifier {
        if self.details_state.modifier_index == 0 {
            Modifier::PlusTwo
        } else {
            Modifier::DNF
        }
    }

    pub fn details_nav_prev(&mut self) {
        self.history_mut().select_previous();
        self.details_state.modifier_index = match self.history().selected_time().map(Time::modifier)
        {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
    }

    pub fn details_nav_next(&mut self) {
        self.history_mut().select_next();
        self.details_state.modifier_index = match self.history().selected_time().map(Time::modifier)
        {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
    }
    pub fn open_details_for_selected_mean_time(&mut self) -> bool {
        let row = self.detailed_stats_state.row;
        let col = self.detailed_stats_state.col;
        let selected_index = self.detailed_stats_state.mean_detail_selected_index;
        let solve_index = if col == 0 {
            if row < 2 {
                return false;
            }
            row.saturating_sub(2).saturating_add(selected_index)
        } else {
            if row < 4 {
                return false;
            }
            row.saturating_sub(4).saturating_add(selected_index)
        };

        if solve_index >= self.history().len() {
            return false;
        }

        self.history_mut().select_index(solve_index);
        self.detailed_stats_state.show = false;
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
        self.open_details();
        true
    }
}
