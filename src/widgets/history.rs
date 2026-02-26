use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::scramble::WcaEvent;
use crate::scramble::WcaEvent::Cube3x3;

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

    pub const fn effective_ms(&self) -> Option<u64> {
        match self.modifier {
            Modifier::None => Some(self.timestamp_in_millis),
            Modifier::PlusTwo => Some(self.timestamp_in_millis + 2000),
            Modifier::DNF => None,
        }
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new(0, Cube3x3, String::new())
    }
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).expect("Failed to parse the SystemTime")
        })
}

pub fn format_millis(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;
    format!("{minutes:02}:{seconds:02}.{millis:03}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AverageValue {
    Time(u64),
    Dnf,
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
    times: Vec<Time>,
    #[serde(skip)]
    pub selected: Option<usize>,
    #[serde(skip, default = "default_highlight_selection")]
    highlight_selection: bool,
}

const fn default_highlight_selection() -> bool {
    true
}

impl History {
    pub const fn new() -> Self {
        Self {
            times: Vec::new(),
            selected: None,
            highlight_selection: true,
        }
    }

    pub const fn without_selection_highlight(mut self) -> Self {
        self.highlight_selection = false;
        self
    }

    pub fn add_ms(&mut self, timestamp_in_millis: u64, event: WcaEvent, scramble: String) {
        self.add(Time::new(timestamp_in_millis, event, scramble));
    }

    pub fn add(&mut self, item: Time) {
        self.times.push(item);
        self.selected = Some(self.times.len() - 1);
    }

    pub const fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    pub fn last(&self) -> Option<&Time> {
        self.times.last()
    }

    pub const fn select_last(&mut self) {
        if !self.is_empty() {
            self.selected = Some(self.times.len() - 1);
        }
    }

    pub fn select_next(&mut self) {
        if self.is_empty() {
            return;
        }
        let selected = self.selected.unwrap_or(0);
        self.selected = Some((selected + 1).min(self.times.len() - 1));
    }

    pub fn select_previous(&mut self) {
        if self.is_empty() {
            return;
        }
        let selected = self.selected.unwrap_or(0);
        self.selected = Some(selected.saturating_sub(1));
    }

    pub fn select_index(&mut self, index: usize) {
        if self.is_empty() {
            return;
        }
        self.selected = Some(index.min(self.times.len() - 1));
    }

    pub fn set_modifier(&mut self, modifier: Modifier) {
        if let Some(selected) = self.selected
            && let Some(time) = self.times.get_mut(selected)
        {
            time.set_modifier(modifier);
        }
    }

    pub fn selected_time(&self) -> Option<&Time> {
        self.selected.and_then(|selected| self.times.get(selected))
    }

    pub fn delete_selected(&mut self) {
        if !self.is_empty() {
            let selected = self.selected.unwrap_or(self.times.len() - 1);
            self.times.remove(selected);
            if self.times.is_empty() {
                self.selected = None;
            } else {
                self.selected = Some(selected.min(self.times.len() - 1));
            }
        }
    }

    pub fn get_latest_time(&self) -> Option<&Time> {
        self.times.last()
    }

    pub fn get_fastest_time(&self) -> Option<&Time> {
        self.times
            .iter()
            .min_by_key(|time| time.timestamp_in_millis)
    }

    pub fn get_latest_mo3(&self) -> Option<String> {
        self.get_mo3(self.times.len())
            .map(Self::format_average_value)
    }

    pub fn get_fastest_mo3(&self) -> Option<String> {
        let mut fastest = u64::MAX;
        let mut has_enough_solves_for_average = false;
        for i in 3..=self.times.len() {
            let Some(mo3) = self.get_mo3(i) else {
                continue;
            };
            has_enough_solves_for_average = true;
            if let AverageValue::Time(value) = mo3 {
                fastest = fastest.min(value);
            }
        }

        if fastest != u64::MAX {
            return Some(format_millis(fastest));
        }
        if has_enough_solves_for_average {
            return Some("DNF".to_string());
        }
        None
    }

    pub fn get_latest_ao5(&self) -> Option<String> {
        self.get_ao5(self.times.len())
            .map(Self::format_average_value)
    }

    pub fn get_fastest_ao5(&self) -> Option<String> {
        let mut fastest = u64::MAX;
        let mut has_enough_solves_for_average = false;
        for i in 5..=self.times.len() {
            let Some(ao5) = self.get_ao5(i) else {
                continue;
            };
            has_enough_solves_for_average = true;
            if let AverageValue::Time(value) = ao5 {
                fastest = fastest.min(value);
            }
        }

        if fastest != u64::MAX {
            return Some(format_millis(fastest));
        }
        if has_enough_solves_for_average {
            return Some("DNF".to_string());
        }
        None
    }

    fn get_mo3(&self, index: usize) -> Option<AverageValue> {
        if index <= 2 {
            return None;
        }

        let times = self.times.get(index.saturating_sub(3)..index)?;

        let mut sum = 0;
        for time in times {
            let Some(value) = Self::effective_millis(time) else {
                return Some(AverageValue::Dnf);
            };
            sum += value;
        }

        Some(AverageValue::Time(sum / 3))
    }

    fn get_ao5(&self, index: usize) -> Option<AverageValue> {
        if index <= 4 {
            return None;
        }

        let attempts: Vec<Option<u64>> = self
            .times
            .get(index.saturating_sub(5)..index)?
            .iter()
            .map(Self::effective_millis)
            .collect();

        let dnf_count = attempts.iter().filter(|attempt| attempt.is_none()).count();
        if dnf_count >= 2 {
            return Some(AverageValue::Dnf);
        }

        let mut valid: Vec<u64> = attempts.iter().flatten().copied().collect();
        valid.sort_unstable();

        if dnf_count == 1 {
            let trimmed_sum: u64 = valid.iter().skip(1).sum();
            return Some(AverageValue::Time(trimmed_sum / 3));
        }

        let trimmed_sum: u64 = valid[1..4].iter().sum();
        Some(AverageValue::Time(trimmed_sum / 3))
    }

    const fn effective_millis(time: &Time) -> Option<u64> {
        match time.modifier {
            Modifier::None => Some(time.timestamp_in_millis),
            Modifier::PlusTwo => Some(time.timestamp_in_millis + 2000),
            Modifier::DNF => None,
        }
    }

    fn format_average_value(value: AverageValue) -> String {
        match value {
            AverageValue::Time(ms) => format_millis(ms),
            AverageValue::Dnf => "DNF".to_string(),
        }
    }

    pub const fn len(&self) -> usize {
        self.times.len()
    }

    pub fn get_time_at(&self, index: usize) -> Option<&Time> {
        self.times.get(index)
    }

    pub fn mo3_at(&self, solve_index: usize) -> Option<String> {
        self.get_mo3(solve_index + 1)
            .map(Self::format_average_value)
    }

    pub fn ao5_at(&self, solve_index: usize) -> Option<String> {
        self.get_ao5(solve_index + 1)
            .map(Self::format_average_value)
    }

    pub fn latest_mo3_index(&self) -> Option<usize> {
        if self.get_mo3(self.times.len()).is_some() {
            Some(self.times.len().saturating_sub(1))
        } else {
            None
        }
    }

    pub fn latest_ao5_index(&self) -> Option<usize> {
        if self.get_ao5(self.times.len()).is_some() {
            Some(self.times.len().saturating_sub(1))
        } else {
            None
        }
    }

    fn fastest_average_index(
        &self,
        start_index: usize,
        average_at: fn(&Self, usize) -> Option<AverageValue>,
    ) -> Option<usize> {
        let mut best: Option<(u64, usize)> = None;
        for index in start_index..=self.times.len() {
            let Some(average) = average_at(self, index) else {
                continue;
            };
            if let AverageValue::Time(value) = average {
                let solve_index = index - 1;
                best = match best {
                    Some((best_value, best_index)) if best_value <= value => {
                        Some((best_value, best_index))
                    }
                    _ => Some((value, solve_index)),
                };
            }
        }
        best.map(|(_, i)| i)
    }

    pub fn fastest_mo3_index(&self) -> Option<usize> {
        self.fastest_average_index(3, Self::get_mo3)
    }

    pub fn fastest_ao5_index(&self) -> Option<usize> {
        self.fastest_average_index(5, Self::get_ao5)
    }

    pub fn mo3_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 2 || solve_index >= self.times.len() {
            return None;
        }
        self.get_mo3(solve_index + 1)?;
        self.times.get(solve_index - 2..=solve_index)
    }

    pub fn ao5_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 4 || solve_index >= self.times.len() {
            return None;
        }
        self.get_ao5(solve_index + 1)?;
        self.times.get(solve_index - 4..=solve_index)
    }
}

impl Widget for History {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let total = self.times.len();
        let height = area.height as usize;
        let selected = self.selected.unwrap_or(0);

        let scroll_full = selected.saturating_sub(height.saturating_sub(1));
        let need_below = scroll_full + height < total;

        let bot_rows = usize::from(need_below);
        let items_height = height.saturating_sub(bot_rows);

        let scroll_offset = selected.saturating_sub(items_height.saturating_sub(1));

        for (i, item) in self.times.iter().enumerate().skip(scroll_offset) {
            let display_row = i - scroll_offset;
            if display_row >= items_height {
                break;
            }
            let Ok(row_offset) = u16::try_from(display_row) else {
                break;
            };
            let style = if self.highlight_selection && self.selected.is_some() && i == selected {
                ratatui::style::Style::default()
                    .bg(ratatui::style::Color::Blue)
                    .fg(ratatui::style::Color::Black)
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
