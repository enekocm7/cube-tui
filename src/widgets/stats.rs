use crate::widgets::history::History;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatsWidget {
    history: History,
    selected_row: Option<usize>,
    selected_col: Option<usize>,
}

impl StatsWidget {
    pub const fn new(history: History) -> Self {
        Self {
            history,
            selected_row: None,
            selected_col: None,
        }
    }

    pub const fn with_selection(mut self, row: usize, col: usize) -> Self {
        self.selected_row = Some(row);
        self.selected_col = Some(col);
        self
    }

    fn fixed_cell(value: String) -> String {
        const CELL_WIDTH: usize = 10;

        let normalized = if value.starts_with("DNF(") {
            "DNF".to_string()
        } else {
            value
        };

        normalized.chars().take(CELL_WIDTH).collect()
    }

    fn selected_style() -> Style {
        Style::default().bg(Color::Blue).fg(Color::Black)
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

        let best_time = Self::fixed_cell(best_time);
        let current_time = Self::fixed_cell(current_time);
        let best_mo3 = Self::fixed_cell(best_mo3);
        let current_mo3 = Self::fixed_cell(current_mo3);
        let best_ao5 = Self::fixed_cell(best_ao5);
        let current_ao5 = Self::fixed_cell(current_ao5);

        let row_line = |label: &str, current: String, best: String, row: usize| {
            let current_selected = self.selected_row == Some(row) && self.selected_col == Some(0);
            let best_selected = self.selected_row == Some(row) && self.selected_col == Some(1);

            let mut spans = vec![Span::raw(format!("{label:8}"))];
            if current_selected {
                spans.push(Span::styled(
                    format!("{current:>10}"),
                    Self::selected_style(),
                ));
            } else {
                spans.push(Span::raw(format!("{current:>10}")));
            }

            if best_selected {
                spans.push(Span::styled(format!("{best:>10}"), Self::selected_style()));
            } else {
                spans.push(Span::raw(format!("{best:>10}")));
            }

            Line::from(spans)
        };

        let text = vec![
            Line::from(format!("{:8}{:>10}{:>10}", "", "current", "best")),
            row_line("time", current_time, best_time, 0),
            row_line("mo3", current_mo3, best_mo3, 1),
            row_line("ao5", current_ao5, best_ao5, 2),
        ];

        Paragraph::new(text).block(block).render(area, buf);
    }
}
