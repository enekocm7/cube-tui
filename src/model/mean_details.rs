use crate::model::Model;
use crate::model::screen::{DetailsReturn, Screen};
use crate::widgets::history::Time;

impl Model {
    pub const fn show_mean_detail(&self) -> bool {
        self.screen.show_mean_detail()
    }

    pub fn open_mean_detail(&mut self) {
        let (row, col) = self.detailed_stats_row_col();
        let has_mean = match col {
            0 => self.history().mo3_at(row).is_some(),
            1 => self.history().ao5_at(row).is_some(),
            2 => self.history().ao12_at(row).is_some(),
            3 => self.history().ao50_at(row).is_some(),
            4 => self.history().ao100_at(row).is_some(),
            _ => false,
        };
        if has_mean {
            self.screen = Screen::MeanDetail {
                row,
                col,
                selected_index: 0,
                from_stats_column: false,
            };
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
            (3, 0) => self.history().latest_ao12_index(),
            (3, 1) => self.history().fastest_ao12_index(),
            (4, 0) => self.history().latest_ao50_index(),
            (4, 1) => self.history().fastest_ao50_index(),
            (5, 0) => self.history().latest_ao100_index(),
            (5, 1) => self.history().fastest_ao100_index(),
            _ => None,
        };

        let Some(solve_index) = solve_index else {
            return false;
        };

        let mean_col = match row {
            2 => 1,
            3 => 2,
            4 => 3,
            5 => 4,
            _ => 0,
        };

        self.screen = Screen::MeanDetail {
            row: solve_index,
            col: mean_col,
            selected_index: 0,
            from_stats_column: true,
        };
        true
    }

    pub fn mean_detail_times_len(&self) -> usize {
        let row = self.detailed_stats_row();
        let times = match self.detailed_stats_col() {
            0 => self.history().mo3_times_at(row),
            1 => self.history().ao5_times_at(row),
            2 => self.history().ao12_times_at(row),
            3 => self.history().ao50_times_at(row),
            4 => self.history().ao100_times_at(row),
            _ => None,
        };
        times.map_or(0, <[Time]>::len)
    }

    pub const fn mean_detail_selected_index(&self) -> usize {
        if let Screen::MeanDetail { selected_index, .. } = &self.screen {
            *selected_index
        } else {
            0
        }
    }

    pub const fn mean_detail_select_up(&mut self) {
        if let Screen::MeanDetail { selected_index, .. } = &mut self.screen {
            *selected_index = selected_index.saturating_sub(1);
        }
    }

    pub fn mean_detail_select_down(&mut self) {
        let max = self.mean_detail_times_len().saturating_sub(1);
        if let Screen::MeanDetail { selected_index, .. } = &mut self.screen {
            *selected_index = selected_index.saturating_add(1).min(max);
        }
    }

    pub fn open_details_for_selected_mean_time(&mut self) -> bool {
        let row = self.detailed_stats_row();
        let col = self.detailed_stats_col();
        let selected_index = self.mean_detail_selected_index();
        let window_size = match col {
            0 => 3,
            1 => 5,
            2 => 12,
            3 => 50,
            4 => 100,
            _ => return false,
        };
        let min_row = window_size - 1;
        if row < min_row {
            return false;
        }
        let solve_index = row.saturating_sub(min_row).saturating_add(selected_index);

        if solve_index >= self.history().len() {
            return false;
        }

        self.history_mut().select_index(solve_index);
        let modifier_index = match self.history().selected_time().map(Time::modifier) {
            Some(crate::widgets::history::Modifier::DNF) => 1,
            _ => 0,
        };
        self.screen = Screen::Details {
            modifier_index,
            return_to: DetailsReturn::MeanDetail {
                row,
                col,
                selected_index,
                from_stats_column: self.mean_detail_from_stats_column(),
            },
        };
        true
    }

    const fn mean_detail_from_stats_column(&self) -> bool {
        if let Screen::MeanDetail {
            from_stats_column, ..
        } = &self.screen
        {
            *from_stats_column
        } else {
            false
        }
    }

    const fn detailed_stats_row_col(&self) -> (usize, usize) {
        if let Screen::DetailedStats { row, col } = &self.screen {
            (*row, *col)
        } else {
            (0, 0)
        }
    }
}
