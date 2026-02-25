use crate::widgets::history::History;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatsWidget {
    history: History,
}

impl StatsWidget {
    pub const fn new(history: History) -> Self {
        Self { history }
    }
}

impl Widget for StatsWidget {
    #[allow(clippy::similar_names)]
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::default().title("Stats").borders(Borders::ALL);

        let best_time = self
            .history
            .get_fastest_time()
            .map_or_else(|| "-".to_string(), ToString::to_string);
        let current_time = self
            .history
            .get_latest_time()
            .map_or_else(|| "-".to_string(), ToString::to_string);
        let best_mo3 = self
            .history
            .get_fastest_mo3()
            .unwrap_or_else(|| "-".to_string());
        let current_mo3 = self
            .history
            .get_latest_mo3()
            .unwrap_or_else(|| "-".to_string());
        let best_ao5 = self
            .history
            .get_fastest_ao5()
            .unwrap_or_else(|| "-".to_string());
        let current_ao5 = self
            .history
            .get_latest_ao5()
            .unwrap_or_else(|| "-".to_string());

        let text = vec![
            Line::from(format!("{:8}{:>10}{:>10}", "", "current", "best")),
            Line::from(format!("{:8}{:>10}{:>10}", "time", current_time, best_time)),
            Line::from(format!("{:8}{:>10}{:>10}", "mo3", current_mo3, best_mo3)),
            Line::from(format!("{:8}{:>10}{:>10}", "ao5", current_ao5, best_ao5)),
        ];
        Paragraph::new(text).block(block).render(area, buf);
    }
}
