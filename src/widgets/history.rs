use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

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
    solved_at_unix_ms: u64,
    #[serde(default)]
    modifier: Modifier,
}

impl Time {
    pub fn new(timestamp_in_millis: u64, event: WcaEvent, scramble: String) -> Self {
        Self {
            timestamp_in_millis,
            event,
            scramble,
            solved_at_unix_ms: current_unix_ms(),
            modifier: Modifier::None,
        }
    }

    pub fn scramble(&self) -> &str {
        &self.scramble
    }

    pub const fn solved_at_unix_ms(&self) -> u64 {
        self.solved_at_unix_ms
    }

    pub const fn modifier(&self) -> Modifier {
        self.modifier
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

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).expect("Failed to parse the SystemTime")
        })
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

    pub const fn select_last(&mut self) {
        if !self.is_empty() {
            self.selected = self.history.len() - 1;
        }
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

    pub fn selected_time(&self) -> Option<&Time> {
        self.history.get(self.selected)
    }

    pub fn delete_selected(&mut self) {
        if !self.is_empty() {
            self.history.remove(self.selected);
            if self.selected >= self.history.len() && !self.is_empty() {
                self.selected = self.history.len() - 1;
            }
        }
    }
}

impl Widget for History {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let total = self.history.len();
        let height = area.height as usize;

        let scroll_full = self.selected.saturating_sub(height.saturating_sub(1));
        let need_below = scroll_full + height < total;

        let bot_rows = usize::from(need_below);
        let items_height = height.saturating_sub(bot_rows);

        let scroll_offset = self.selected.saturating_sub(items_height.saturating_sub(1));

        for (i, item) in self.history.iter().enumerate().skip(scroll_offset) {
            let display_row = i - scroll_offset;
            if display_row >= items_height {
                break;
            }
            let Ok(row_offset) = u16::try_from(display_row) else {
                break;
            };
            let style = if i == self.selected {
                ratatui::style::Style::default().bg(ratatui::style::Color::Blue)
            } else {
                ratatui::style::Style::default()
            };
            buf.set_string(
                area.x,
                area.y + row_offset,
                format!("{}: {item}", i + 1),
                style,
            );
        }

        if need_below {
            let below_count = total.saturating_sub(scroll_offset + items_height);
            buf.set_string(
                area.x,
                area.y + area.height - 1,
                format!("↓ {below_count} more"),
                ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
            );
        }
    }
}
