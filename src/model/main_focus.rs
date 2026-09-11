#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MainFocus {
    History,
    Stats,
}

pub struct MainStatsSelection {
    pub row: usize,
    pub col: usize,
}

impl Default for MainStatsSelection {
    /// Selects the first cell in the statistics pane.
    fn default() -> Self {
        Self { row: 1, col: 0 }
    }
}
