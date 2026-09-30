use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

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
    scramble: Cow<'static, str>,
    #[serde(default)]
    solved_at_unix_ms: u64,
    #[serde(default)]
    modifier: Modifier,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    changes: Vec<SolveChange>,
    /// Persisted display marker for a single that improved the session best.
    #[serde(default)]
    pub(super) was_single_record: bool,
}

/// Editable metadata, without recursively copying the audit trail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveSnapshot {
    pub time_ms: u64,
    pub event: WcaEvent,
    pub scramble: String,
    pub solved_at_unix_ms: u64,
    pub modifier: Modifier,
    pub comment: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolveChange {
    pub changed_at_unix_ms: u64,
    pub before: SolveSnapshot,
    pub after: SolveSnapshot,
}

impl Time {
    /// Creates an unpenalized solve timestamped at the current wall-clock time.
    pub fn new(
        timestamp_in_millis: u64,
        event: WcaEvent,
        scramble: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new_with_modifier(timestamp_in_millis, event, scramble, Modifier::None)
    }

    /// Creates a solve with an explicit modifier and the current wall-clock time.
    pub fn new_with_modifier(
        timestamp_in_millis: u64,
        event: WcaEvent,
        scramble: impl Into<Cow<'static, str>>,
        modifier: Modifier,
    ) -> Self {
        Self {
            timestamp_in_millis,
            event,
            scramble: scramble.into(),
            solved_at_unix_ms: current_unix_ms(),
            modifier,
            comment: String::new(),
            changes: Vec::new(),
            was_single_record: false,
        }
    }

    /// Reconstructs a solve with explicit persisted metadata.
    pub const fn new_with_meta(
        timestamp_in_millis: u64,
        event: WcaEvent,
        scramble: Cow<'static, str>,
        solved_at_unix_ms: u64,
        modifier: Modifier,
    ) -> Self {
        Self {
            timestamp_in_millis,
            event,
            scramble,
            solved_at_unix_ms,
            modifier,
            comment: String::new(),
            changes: Vec::new(),
            was_single_record: false,
        }
    }

    /// Returns the scramble used for this solve.
    pub fn scramble(&self) -> &str {
        &self.scramble
    }

    /// Returns when the solve finished, as Unix epoch milliseconds.
    pub const fn solved_at_unix_ms(&self) -> u64 {
        self.solved_at_unix_ms
    }

    /// Returns the measured time before applying a penalty.
    pub const fn raw_ms(&self) -> u64 {
        self.timestamp_in_millis
    }

    /// Returns the solve's current penalty modifier.
    pub const fn modifier(&self) -> Modifier {
        self.modifier
    }

    /// Returns the puzzle event solved by this attempt.
    pub const fn event(&self) -> WcaEvent {
        self.event
    }

    pub const fn was_single_record(&self) -> bool {
        self.was_single_record
    }

    pub fn comment(&self) -> &str {
        &self.comment
    }

    /// Supplies an imported comment without recording an edit.
    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = comment;
        self
    }

    pub fn changes(&self) -> &[super::SolveChange] {
        &self.changes
    }

    pub fn snapshot(&self) -> SolveSnapshot {
        SolveSnapshot {
            time_ms: self.timestamp_in_millis,
            event: self.event,
            scramble: self.scramble().to_owned(),
            solved_at_unix_ms: self.solved_at_unix_ms,
            modifier: self.modifier,
            comment: self.comment.clone(),
        }
    }

    /// Applies an atomic edit and retains both versions; no-op saves are ignored.
    pub(super) fn edit(&mut self, after: SolveSnapshot) -> bool {
        let before = self.snapshot();
        if before == after {
            return false;
        }
        self.timestamp_in_millis = after.time_ms;
        self.event = after.event;
        self.scramble = after.scramble.clone().into();
        self.solved_at_unix_ms = after.solved_at_unix_ms;
        self.modifier = after.modifier;
        self.comment.clone_from(&after.comment);
        self.changes.push(SolveChange {
            changed_at_unix_ms: current_unix_ms(),
            before,
            after,
        });
        true
    }

    /// Toggles `modifier`, clearing it when it is already selected.
    pub fn set_modifier(&mut self, modifier: Modifier) {
        let mut after = self.snapshot();
        after.modifier = if self.modifier == modifier {
            Modifier::None
        } else {
            modifier
        };
        self.edit(after);
    }

    /// Returns the penalty-adjusted duration, or `None` for a DNF.
    pub const fn effective_ms(&self) -> Option<u64> {
        match self.modifier {
            Modifier::None => Some(self.timestamp_in_millis),
            Modifier::PlusTwo => Some(self.timestamp_in_millis + 2000),
            Modifier::DNF => None,
        }
    }
}

impl Default for Time {
    /// Creates a zero-duration 3×3 solve for deserialization defaults.
    fn default() -> Self {
        Self::new(0, Cube3x3, String::new())
    }
}

/// Returns the current Unix epoch timestamp in milliseconds.
fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).expect("Failed to parse the SystemTime")
        })
}

/// Formats milliseconds as `MM:SS.mmm`.
pub fn format_millis(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;
    format!("{minutes:02}:{seconds:02}.{millis:03}")
}

pub(super) fn mark_single_records(times: &mut [Time]) {
    let mut best = None;
    for time in times {
        time.was_single_record = time
            .effective_ms()
            .is_some_and(|millis| best.is_none_or(|previous| millis < previous));
        if time.was_single_record {
            best = time.effective_ms();
        }
    }
}

impl Display for Time {
    /// Formats the effective solve time and its penalty marker.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.modifier {
            Modifier::None => f.write_str(&format_millis(self.timestamp_in_millis)),
            Modifier::PlusTwo => {
                let adjusted = self.timestamp_in_millis + 2000;
                write!(f, "{}+", format_millis(adjusted))
            }
            Modifier::DNF => {
                write!(f, "DNF")
            }
        }
    }
}
