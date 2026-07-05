use std::borrow::Cow;

use crate::model::settings::ThemeSettings;
use crate::widgets::history::History;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub struct StatsWidget<'a> {
    history: &'a History,
    selected_row: Option<usize>,
    selected_col: Option<usize>,
}

impl<'a> StatsWidget<'a> {
    pub const fn new(history: &'a History) -> Self {
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

    fn fixed_cell(value: Cow<'static, str>) -> Cow<'static, str> {
        const CELL_WIDTH: usize = 10;

        if value.starts_with("DNF(") {
            return Cow::Borrowed("DNF");
        }
        if value.chars().count() <= CELL_WIDTH {
            return value;
        }

        Cow::Owned(value.chars().take(CELL_WIDTH).collect())
    }

    #[allow(clippy::similar_names)]
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeSettings) {
        let block = Block::default()
            .title("Stats")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()));

        let best_time = self
            .history
            .get_fastest_time()
            .map_or_else(|| Cow::Borrowed("-"), |t| Cow::Owned(t.to_string()));
        let current_time = self
            .history
            .get_latest_time()
            .map_or_else(|| Cow::Borrowed("-"), |t| Cow::Owned(t.to_string()));
        let best_mo3 = self.history.get_fastest_mo3().unwrap_or(Cow::Borrowed("-"));
        let current_mo3 = self.history.get_latest_mo3().unwrap_or(Cow::Borrowed("-"));
        let best_ao5 = self.history.get_fastest_ao5().unwrap_or(Cow::Borrowed("-"));
        let current_ao5 = self.history.get_latest_ao5().unwrap_or(Cow::Borrowed("-"));
        let best_ao12 = self
            .history
            .get_fastest_ao12()
            .unwrap_or(Cow::Borrowed("-"));
        let current_ao12 = self.history.get_latest_ao12().unwrap_or(Cow::Borrowed("-"));
        let best_ao50 = self
            .history
            .get_fastest_ao50()
            .unwrap_or(Cow::Borrowed("-"));
        let current_ao50 = self.history.get_latest_ao50().unwrap_or(Cow::Borrowed("-"));
        let best_ao100 = self
            .history
            .get_fastest_ao100()
            .unwrap_or(Cow::Borrowed("-"));
        let current_ao100 = self
            .history
            .get_latest_ao100()
            .unwrap_or(Cow::Borrowed("-"));

        let best_time = Self::fixed_cell(best_time);
        let current_time = Self::fixed_cell(current_time);
        let best_mo3 = Self::fixed_cell(best_mo3);
        let current_mo3 = Self::fixed_cell(current_mo3);
        let best_ao5 = Self::fixed_cell(best_ao5);
        let current_ao5 = Self::fixed_cell(current_ao5);
        let best_ao12 = Self::fixed_cell(best_ao12);
        let current_ao12 = Self::fixed_cell(current_ao12);
        let best_ao50 = Self::fixed_cell(best_ao50);
        let current_ao50 = Self::fixed_cell(current_ao50);
        let best_ao100 = Self::fixed_cell(best_ao100);
        let current_ao100 = Self::fixed_cell(current_ao100);

        let selected_row = self.selected_row;
        let selected_col = self.selected_col;

        let row_line =
            |label: &str, current: Cow<'static, str>, best: Cow<'static, str>, row: usize| {
                let current_selected = selected_row == Some(row) && selected_col == Some(0);
                let best_selected = selected_row == Some(row) && selected_col == Some(1);

                let mut spans = vec![Span::styled(
                    format!("{label:8}"),
                    Style::default().fg(theme.text()),
                )];
                if current_selected {
                    spans.push(Span::styled(
                        format!("{current:>10}"),
                        Style::default()
                            .bg(theme.selection())
                            .fg(theme.selection_text()),
                    ));
                } else {
                    spans.push(Span::styled(
                        format!("{current:>10}"),
                        Style::default().fg(theme.text()),
                    ));
                }

                if best_selected {
                    spans.push(Span::styled(
                        format!("{best:>10}"),
                        Style::default()
                            .bg(theme.selection())
                            .fg(theme.selection_text()),
                    ));
                } else {
                    spans.push(Span::styled(
                        format!("{best:>10}"),
                        Style::default().fg(theme.text()),
                    ));
                }

                Line::from(spans)
            };

        let text = vec![
            Line::from(Span::styled(
                format!("{:8}{:>10}{:>10}", "", "current", "best"),
                Style::default().fg(theme.text()),
            )),
            row_line("time", current_time, best_time, 0),
            row_line("mo3", current_mo3, best_mo3, 1),
            row_line("ao5", current_ao5, best_ao5, 2),
            row_line("ao12", current_ao12, best_ao12, 3),
            row_line("ao50", current_ao50, best_ao50, 4),
            row_line("ao100", current_ao100, best_ao100, 5),
        ];

        Paragraph::new(text).block(block).render(area, buf);
    }
}
