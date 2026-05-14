#[derive(Default)]
pub struct DetailedStatsState {
    pub show: bool,
    pub row: usize,
    pub col: usize,
    pub show_mean_detail: bool,
    pub mean_detail_selected_index: usize,
    pub opened_from_stats_column: bool,
}
