use crate::model::Model;
use crate::model::screen::Screen;

impl Model {
    pub const fn show_detailed_stats(&self) -> bool {
        self.screen.show_detailed_stats()
    }

    pub fn open_detailed_stats(&mut self) {
        if self.history().is_empty() {
            return;
        }
        self.screen = Screen::DetailedStats {
            row: self.history().len().saturating_sub(1),
            col: 0,
        };
    }

    pub const fn detailed_stats_row(&self) -> usize {
        match &self.screen {
            Screen::DetailedStats { row, .. } | Screen::MeanDetail { row, .. } => *row,
            _ => 0,
        }
    }

    pub const fn detailed_stats_col(&self) -> usize {
        match &self.screen {
            Screen::DetailedStats { col, .. } | Screen::MeanDetail { col, .. } => *col,
            _ => 0,
        }
    }

    pub const fn detailed_stats_select_up(&mut self) {
        if let Screen::DetailedStats { row, .. } = &mut self.screen {
            *row = row.saturating_sub(1);
        }
    }

    pub fn detailed_stats_select_down(&mut self) {
        let max = self.history().len().saturating_sub(1);
        if let Screen::DetailedStats { row, .. } = &mut self.screen {
            *row = (*row + 1).min(max);
        }
    }

    pub const fn detailed_stats_col_left(&mut self) {
        if let Screen::DetailedStats { col, .. } = &mut self.screen {
            *col = col.saturating_sub(1);
        }
    }

    pub const fn detailed_stats_col_right(&mut self) {
        if let Screen::DetailedStats { col, .. } = &mut self.screen
            && *col < 4
        {
            *col += 1;
        }
    }
}
