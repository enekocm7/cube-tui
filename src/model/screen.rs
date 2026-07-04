#[derive(Clone, Debug, Default)]
pub enum Screen {
    #[default]
    Main,
    DetailedStats {
        row: usize,
        col: usize,
    },
    MeanDetail {
        row: usize,
        col: usize,
        selected_index: usize,
        from_stats_column: bool,
    },
    Details {
        modifier_index: usize,
        return_to: DetailsReturn,
    },
}

#[derive(Clone, Debug)]
pub enum DetailsReturn {
    Main,
    MeanDetail {
        row: usize,
        col: usize,
        selected_index: usize,
        from_stats_column: bool,
    },
}

impl Screen {
    pub const fn show_details(&self) -> bool {
        matches!(self, Self::Details { .. })
    }

    pub const fn show_detailed_stats(&self) -> bool {
        matches!(self, Self::DetailedStats { .. })
    }

    pub const fn show_mean_detail(&self) -> bool {
        matches!(self, Self::MeanDetail { .. })
    }
}
