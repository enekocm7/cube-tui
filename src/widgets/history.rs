use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

use crate::scramble::WcaEvent;

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modifier {
    #[default]
    None,
    PlusTwo,
    DNF,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Time {
    timestamp_in_millis: u64,
    event: WcaEvent,
    scramble: String,
    #[serde(default)]
    modifier: Modifier,
}

impl Time {
    pub const fn new(timestamp_in_millis: u64, event: WcaEvent, scramble: String) -> Self {
        Self {
            timestamp_in_millis,
            event,
            scramble,
            modifier: Modifier::None,
        }
    }

    pub const fn event(&self) -> WcaEvent {
        self.event
    }

    pub fn set_modifier(&mut self, modifier: Modifier) {
        if self.modifier == modifier {
            self.modifier = Modifier::None;
        } else {
            self.modifier = modifier;
        }
    }
}

fn format_millis(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;
    format!("{minutes:02}:{seconds:02}.{millis:03}")
}

impl Display for Time {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.modifier {
            Modifier::None => f.write_str(&format_millis(self.timestamp_in_millis)),
            Modifier::PlusTwo => {
                let adjusted = self.timestamp_in_millis + 2000;
                write!(f, "{}+", format_millis(adjusted))
            }
            Modifier::DNF => {
                write!(f, "DNF({})", format_millis(self.timestamp_in_millis))
            }
        }
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

    pub fn set_modifier(&mut self, modifier: Modifier) {
        if let Some(time) = self.history.get_mut(self.selected) {
            time.set_modifier(modifier);
        }
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
            buf.set_string(area.x, row, format!("{}: {item}", i + 1), style);
        }
    }
}
