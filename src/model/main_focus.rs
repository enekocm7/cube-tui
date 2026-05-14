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
    fn default() -> Self {
        Self { row: 1, col: 0 }
    }
}
