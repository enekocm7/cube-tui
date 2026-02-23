use std::fmt::{Display, Formatter};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use serde::{Deserialize, Serialize};

use crate::scramble::WcaEvent;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Time {
    timestamp_in_millis: u64,
    event: WcaEvent,
    scramble: String,
}

impl Time {
    pub const fn new(timestamp_in_millis: u64, event: WcaEvent, scramble: String) -> Self {
        Self {
            timestamp_in_millis,
            event,
            scramble,
        }
    }

    pub const fn event(&self) -> WcaEvent {
        self.event
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let total_seconds = self.timestamp_in_millis / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        let millis = self.timestamp_in_millis % 1000;
        f.write_str(&format!("{minutes:02}:{seconds:02}.{millis:03}"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    history: Vec<Time>,
    #[serde(skip)]
    pub selected: usize,
}

impl History {
    pub const fn new() -> Self {
        Self {
            history: Vec::new(),
            selected: 0,
        }
    }

    pub fn add_ms(&mut self, timestamp_in_millis: u64, event: WcaEvent, scramble: String) {
        self.add(Time::new(timestamp_in_millis, event, scramble));
    }

    pub fn add(&mut self, item: Time) {
        self.history.push(item);
        self.selected = self.history.len() - 1;
    }

    pub const fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    pub fn last(&self) -> Option<&Time> {
        self.history.last()
    }

    pub fn select_next(&mut self) {
        if self.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.history.len() - 1);
    }

    pub const fn select_previous(&mut self) {
        if self.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
    }
}

impl Widget for History {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        for (i, item) in self.history.iter().enumerate() {
            let Ok(offset) = u16::try_from(i) else {
                break;
            };
            let row = area.y + offset;
            if row >= area.y + area.height {
                break;
            }
            let style = if i == self.selected {
                ratatui::style::Style::default().bg(ratatui::style::Color::Blue)
            } else {
                ratatui::style::Style::default()
            };
            buf.set_string(
                area.x,
                row,
                format!("{}: {item}", i + 1),
                style,
            );
        }
    }
}
