use crate::model::Model;
use crate::model::screen::Screen;

impl Model {
    /// Returns whether the detailed-statistics screen is visible.
    pub const fn show_detailed_stats(&self) -> bool {
        self.screen.show_detailed_stats()
    }

    /// Opens detailed statistics at the most recent solve.
    pub fn open_detailed_stats(&mut self) {
        if self.history().is_empty() {
            return;
        }
        self.screen = Screen::DetailedStats {
            row: self.history().len().saturating_sub(1),
            col: 0,
        };
    }

    /// Returns the selected solve row for a detailed statistics view.
    pub const fn detailed_stats_row(&self) -> usize {
        match &self.screen {
            Screen::DetailedStats { row, .. } | Screen::MeanDetail { row, .. } => *row,
            _ => 0,
        }
    }

    /// Returns the selected average column for a detailed statistics view.
    pub const fn detailed_stats_col(&self) -> usize {
        match &self.screen {
            Screen::DetailedStats { col, .. } | Screen::MeanDetail { col, .. } => *col,
            _ => 0,
        }
    }

    /// Moves the detailed-statistics selection up one row.
    pub const fn detailed_stats_select_up(&mut self) {
        if let Screen::DetailedStats { row, .. } = &mut self.screen {
            *row = row.saturating_sub(1);
        }
    }

    /// Moves the detailed-statistics selection down within the history.
    pub fn detailed_stats_select_down(&mut self) {
        let max = self.history().len().saturating_sub(1);
        if let Screen::DetailedStats { row, .. } = &mut self.screen {
            *row = (*row + 1).min(max);
        }
    }

    /// Moves the detailed-statistics selection left one column.
    pub const fn detailed_stats_col_left(&mut self) {
        if let Screen::DetailedStats { col, .. } = &mut self.screen {
            *col = col.saturating_sub(1);
        }
    }

    /// Moves the detailed-statistics selection right within supported averages.
    pub const fn detailed_stats_col_right(&mut self) {
        if let Screen::DetailedStats { col, .. } = &mut self.screen
            && *col < 4
        {
            *col += 1;
        }
    }
}
