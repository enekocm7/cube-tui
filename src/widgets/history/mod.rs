mod render;
mod statistics;
mod time;

use std::borrow::Cow;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::scramble::WcaEvent;
use statistics::{AVERAGE_SIZES, BestAverage};

pub use time::{Modifier, SolveChange, SolveSnapshot, Time, format_millis};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    /// Session metadata is persisted alongside solves; older files omit it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    session_name: String,
    times: Vec<Time>,
    #[serde(skip)]
    fastest_time: OnceLock<Option<usize>>,
    #[serde(skip)]
    fastest_averages: [OnceLock<Option<BestAverage>>; AVERAGE_SIZES.len()],
    #[serde(skip)]
    pub selected: Option<usize>,
}

impl History {
    /// Creates an empty history with uninitialized statistics caches.
    pub const fn new() -> Self {
        Self {
            session_name: String::new(),
            times: Vec::new(),
            fastest_time: OnceLock::new(),
            fastest_averages: [const { OnceLock::new() }; AVERAGE_SIZES.len()],
            selected: None,
        }
    }

    /// Returns the custom session name, or an empty string for an unnamed session.
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    /// Sets the session label without affecting solves or statistics caches.
    pub fn set_session_name(&mut self, name: String) {
        self.session_name = name;
    }

    /// Creates and appends a solve from its duration, event, and scramble.
    pub fn add_ms(
        &mut self,
        timestamp_in_millis: u64,
        event: WcaEvent,
        scramble: impl Into<Cow<'static, str>>,
    ) {
        self.add(Time::new(timestamp_in_millis, event, scramble));
    }

    /// Appends a solve, selects it, and incrementally updates initialized caches.
    pub fn add(&mut self, mut item: Time) {
        let previous_best = self.get_fastest_time().and_then(Time::effective_ms);
        item.was_single_record = item
            .effective_ms()
            .is_some_and(|millis| previous_best.is_none_or(|best| millis < best));
        self.times.push(item);
        self.selected = Some(self.times.len() - 1);
        self.update_fastest_after_append();
    }

    /// Returns all solves in chronological order.
    pub fn times(&self) -> &[Time] {
        &self.times
    }

    /// Returns whether the history contains no solves.
    pub const fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// Returns the most recently appended solve.
    pub fn last(&self) -> Option<&Time> {
        self.times.last()
    }

    /// Selects the most recent solve when the history is non-empty.
    pub const fn select_last(&mut self) {
        if !self.is_empty() {
            self.selected = Some(self.times.len() - 1);
        }
    }

    /// Moves selection one solve toward the end of history.
    pub fn select_next(&mut self) {
        if self.is_empty() {
            return;
        }
        let selected = self.selected.unwrap_or(0);
        self.selected = Some((selected + 1).min(self.times.len() - 1));
    }

    /// Moves selection one solve toward the beginning of history.
    pub fn select_previous(&mut self) {
        if self.is_empty() {
            return;
        }
        let selected = self.selected.unwrap_or(0);
        self.selected = Some(selected.saturating_sub(1));
    }

    /// Selects an index, clamping it to the available history.
    pub fn select_index(&mut self, index: usize) {
        if self.is_empty() {
            return;
        }
        self.selected = Some(index.min(self.times.len() - 1));
    }

    /// Toggles a modifier on the selected solve and invalidates derived statistics.
    pub fn set_modifier(&mut self, modifier: Modifier) {
        if let Some(selected) = self.selected
            && let Some(time) = self.times.get_mut(selected)
        {
            time.set_modifier(modifier);
            self.invalidate_fastest();
        }
    }

    /// Returns the currently selected solve.
    pub fn selected_time(&self) -> Option<&Time> {
        self.selected.and_then(|selected| self.times.get(selected))
    }

    /// Edits a fixed solve index and invalidates all derived statistics.
    pub fn edit_solve(&mut self, index: usize, after: SolveSnapshot) -> bool {
        if self
            .times
            .get_mut(index)
            .is_some_and(|time| time.edit(after))
        {
            self.invalidate_fastest();
            true
        } else {
            false
        }
    }

    /// Deletes the selected solve and invalidates derived statistics.
    pub fn delete_selected(&mut self) {
        if !self.is_empty() {
            let selected = self.selected.unwrap_or(self.times.len() - 1);
            self.times.remove(selected);
            self.invalidate_fastest();
            if self.times.is_empty() {
                self.selected = None;
            } else {
                self.selected = Some(selected.min(self.times.len() - 1));
            }
        }
    }

    /// Returns the most recent solve.
    pub fn get_latest_time(&self) -> Option<&Time> {
        self.times.last()
    }

    /// Returns the number of solves in the history.
    pub const fn len(&self) -> usize {
        self.times.len()
    }

    /// Returns the solve at `index`.
    pub fn get_time_at(&self, index: usize) -> Option<&Time> {
        self.times.get(index)
    }
}
