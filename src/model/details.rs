#[derive(Default)]
pub struct DetailsState {
    pub show: bool,
    pub modifier_index: usize,
    pub return_to_mean_detail: Option<MeanDetailReturnState>,
    pub return_to_stats: Option<StatsReturnState>,
}

#[derive(Copy, Clone)]
pub struct MeanDetailReturnState {
    pub row: usize,
    pub col: usize,
    pub selected_index: usize,
}

#[derive(Copy, Clone)]
pub struct StatsReturnState {
    pub row: usize,
    pub col: usize,
}
