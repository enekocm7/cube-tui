use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};

use crate::model::settings::ThemeColors;
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
}

impl Time {
    /// Creates an unpenalized solve timestamped at the current wall-clock time.
    pub fn new(
        timestamp_in_millis: u64,
        event: WcaEvent,
        scramble: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            timestamp_in_millis,
            event,
            scramble: scramble.into(),
            solved_at_unix_ms: current_unix_ms(),
            modifier: Modifier::None,
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

    /// Toggles `modifier`, clearing it when it is already selected.
    pub fn set_modifier(&mut self, modifier: Modifier) {
        if self.modifier == modifier {
            self.modifier = Modifier::None;
        } else {
            self.modifier = modifier;
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AverageValue {
    Time(u64),
    Dnf,
}

const AVERAGE_SIZES: [usize; 5] = [3, 5, 12, 50, 100];

#[derive(Clone, Copy, Debug)]
struct BestAverage {
    millis: u64,
    solve_index: usize,
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
                write!(f, "DNF({})", format_millis(self.timestamp_in_millis))
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
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
            times: Vec::new(),
            fastest_time: OnceLock::new(),
            fastest_averages: [const { OnceLock::new() }; AVERAGE_SIZES.len()],
            selected: None,
        }
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
    pub fn add(&mut self, item: Time) {
        self.times.push(item);
        self.selected = Some(self.times.len() - 1);
        self.update_fastest_after_append();
    }

    /// Updates previously requested fastest statistics for the newest solve.
    ///
    /// Uninitialized caches are left untouched so bulk imports do not calculate
    /// statistics that the UI may never request.
    fn update_fastest_after_append(&mut self) {
        let solve_index = self.times.len() - 1;
        if let Some(best_index) = self.fastest_time.get_mut()
            && let Some(solve_ms) = self.times[solve_index].effective_ms()
            && best_index.is_none_or(|index| {
                self.times[index]
                    .effective_ms()
                    .is_none_or(|best_ms| solve_ms < best_ms)
            })
        {
            *best_index = Some(solve_index);
        }

        // Only maintain statistics that have been requested, so imports can append
        // solves without calculating every average along the way.
        for (slot, n) in AVERAGE_SIZES.into_iter().enumerate() {
            let Some(mut best) = self.fastest_averages[slot].take() else {
                continue;
            };
            if let Some(AverageValue::Time(millis)) = self.get_avg(self.times.len(), n)
                && best.is_none_or(|average| millis < average.millis)
            {
                best = Some(BestAverage {
                    millis,
                    solve_index,
                });
            }
            self.fastest_averages[slot] = OnceLock::from(best);
        }
    }

    /// Clears derived statistics after an edit that can reorder results.
    fn invalidate_fastest(&mut self) {
        self.fastest_time.take();
        for cache in &mut self.fastest_averages {
            cache.take();
        }
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

    /// Returns the fastest non-DNF solve, calculating and caching its index on demand.
    pub fn get_fastest_time(&self) -> Option<&Time> {
        self.fastest_time
            .get_or_init(|| {
                self.times
                    .iter()
                    .enumerate()
                    .filter_map(|(index, time)| time.effective_ms().map(|millis| (millis, index)))
                    .min_by_key(|&(millis, _)| millis)
                    .map(|(_, index)| index)
            })
            .and_then(|index| self.times.get(index))
    }

    /// Returns the latest mean of three as formatted text.
    pub fn get_latest_mo3(&self) -> Option<Cow<'static, str>> {
        self.get_mo3(self.times.len())
            .map(Self::format_average_value)
    }

    /// Returns the fastest mean of three as formatted text.
    pub fn get_fastest_mo3(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(3)
    }

    /// Returns the latest average of five as formatted text.
    pub fn get_latest_ao5(&self) -> Option<Cow<'static, str>> {
        self.get_ao5(self.times.len())
            .map(Self::format_average_value)
    }

    /// Returns the fastest average of five as formatted text.
    pub fn get_fastest_ao5(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(5)
    }

    /// Returns the latest average of twelve as formatted text.
    pub fn get_latest_ao12(&self) -> Option<Cow<'static, str>> {
        self.get_ao12(self.times.len())
            .map(Self::format_average_value)
    }

    /// Returns the fastest average of twelve as formatted text.
    pub fn get_fastest_ao12(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(12)
    }

    /// Returns the latest average of fifty as formatted text.
    pub fn get_latest_ao50(&self) -> Option<Cow<'static, str>> {
        self.get_ao50(self.times.len())
            .map(Self::format_average_value)
    }

    /// Returns the fastest average of fifty as formatted text.
    pub fn get_fastest_ao50(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(50)
    }

    /// Returns the latest average of one hundred as formatted text.
    pub fn get_latest_ao100(&self) -> Option<Cow<'static, str>> {
        self.get_ao100(self.times.len())
            .map(Self::format_average_value)
    }

    /// Returns the fastest average of one hundred as formatted text.
    pub fn get_fastest_ao100(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(100)
    }

    /// Formats the cached fastest average for a supported window size.
    ///
    /// Returns `None` when too few solves exist and `DNF` when every complete
    /// window is invalid.
    fn fastest_average_value(&self, n: usize) -> Option<Cow<'static, str>> {
        if self.times.len() < n {
            return None;
        }
        Some(
            self.fastest_average(n)
                .map_or(Cow::Borrowed("DNF"), |best| {
                    Cow::Owned(format_millis(best.millis))
                }),
        )
    }

    /// Returns the fastest valid average for a supported window size.
    ///
    /// The first request scans the history and caches the value and ending solve
    /// index. Later reads are constant-time until an edit invalidates the cache.
    fn fastest_average(&self, n: usize) -> Option<BestAverage> {
        let slot = AVERAGE_SIZES.iter().position(|&size| size == n)?;
        *self.fastest_averages[slot].get_or_init(|| {
            (n..=self.times.len())
                .filter_map(|index| match self.get_avg(index, n) {
                    Some(AverageValue::Time(millis)) => Some(BestAverage {
                        millis,
                        solve_index: index - 1,
                    }),
                    _ => None,
                })
                .min_by_key(|average| average.millis)
        })
    }

    /// Returns the mean of three ending immediately before `index`.
    fn get_mo3(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 3)
    }

    /// Returns the average of five ending immediately before `index`.
    fn get_ao5(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 5)
    }

    /// Returns the average of twelve ending immediately before `index`.
    fn get_ao12(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 12)
    }

    /// Returns the average of fifty ending immediately before `index`.
    fn get_ao50(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 50)
    }

    /// Returns the average of one hundred ending immediately before `index`.
    fn get_ao100(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 100)
    }

    /// Calculates the average ending at `index` using constant temporary memory.
    ///
    /// Mean-of-three windows reject any DNF. Larger WCA averages discard the
    /// best and worst result, treating one DNF as the discarded worst result.
    fn get_avg(&self, index: usize, n: usize) -> Option<AverageValue> {
        if index < n {
            return None;
        }

        let attempts = self.times.get(index.saturating_sub(n)..index)?;
        // Trimming only the best and worst needs their extrema, not a sorted
        // allocation. A wider sum also accommodates large discarded values.
        let mut sum = 0_u128;
        let mut best = u64::MAX;
        let mut worst = 0;
        let mut dnf_count = 0;
        for time in attempts {
            match time.effective_ms() {
                Some(millis) => {
                    sum += u128::from(millis);
                    best = best.min(millis);
                    worst = worst.max(millis);
                }
                None => dnf_count += 1,
            }
        }

        if n == 3 {
            if dnf_count > 0 {
                return Some(AverageValue::Dnf);
            }
            return Some(AverageValue::Time(
                u64::try_from(sum / 3).expect("the mean fits in u64"),
            ));
        }

        if dnf_count >= 2 {
            return Some(AverageValue::Dnf);
        }

        sum -= u128::from(best);
        if dnf_count == 0 {
            sum -= u128::from(worst);
        }
        Some(AverageValue::Time(
            u64::try_from(sum / (n - 2) as u128).expect("the mean fits in u64"),
        ))
    }

    /// Converts an average result to the text shown in statistics views.
    fn format_average_value(value: AverageValue) -> Cow<'static, str> {
        match value {
            AverageValue::Time(ms) => Cow::Owned(format_millis(ms)),
            AverageValue::Dnf => Cow::Borrowed("DNF"),
        }
    }

    /// Returns the number of solves in the history.
    pub const fn len(&self) -> usize {
        self.times.len()
    }

    /// Returns the solve at `index`.
    pub fn get_time_at(&self, index: usize) -> Option<&Time> {
        self.times.get(index)
    }

    /// Returns the mean of three ending at `solve_index`.
    pub fn mo3_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_mo3(solve_index + 1)
            .map(Self::format_average_value)
    }

    /// Returns the average of five ending at `solve_index`.
    pub fn ao5_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_ao5(solve_index + 1)
            .map(Self::format_average_value)
    }

    /// Returns the average of twelve ending at `solve_index`.
    pub fn ao12_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_ao12(solve_index + 1)
            .map(Self::format_average_value)
    }

    /// Returns the average of fifty ending at `solve_index`.
    pub fn ao50_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_ao50(solve_index + 1)
            .map(Self::format_average_value)
    }

    /// Returns the average of one hundred ending at `solve_index`.
    pub fn ao100_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_ao100(solve_index + 1)
            .map(Self::format_average_value)
    }

    /// Returns the ending index of the latest complete mean of three.
    pub const fn latest_mo3_index(&self) -> Option<usize> {
        if self.times.len() >= 3 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    /// Returns the ending index of the latest complete average of five.
    pub const fn latest_ao5_index(&self) -> Option<usize> {
        if self.times.len() >= 5 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    /// Returns the ending index of the latest complete average of twelve.
    pub const fn latest_ao12_index(&self) -> Option<usize> {
        if self.times.len() >= 12 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    /// Returns the ending index of the latest complete average of fifty.
    pub const fn latest_ao50_index(&self) -> Option<usize> {
        if self.times.len() >= 50 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    /// Returns the ending index of the latest complete average of one hundred.
    pub const fn latest_ao100_index(&self) -> Option<usize> {
        if self.times.len() >= 100 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    /// Returns the ending solve index of the cached fastest average.
    fn fastest_average_index(&self, n: usize) -> Option<usize> {
        self.fastest_average(n).map(|best| best.solve_index)
    }

    /// Returns the ending solve index of the fastest mean of three.
    pub fn fastest_mo3_index(&self) -> Option<usize> {
        self.fastest_average_index(3)
    }

    /// Returns the ending solve index of the fastest average of five.
    pub fn fastest_ao5_index(&self) -> Option<usize> {
        self.fastest_average_index(5)
    }

    /// Returns the ending solve index of the fastest average of twelve.
    pub fn fastest_ao12_index(&self) -> Option<usize> {
        self.fastest_average_index(12)
    }

    /// Returns the ending solve index of the fastest average of fifty.
    pub fn fastest_ao50_index(&self) -> Option<usize> {
        self.fastest_average_index(50)
    }

    /// Returns the ending solve index of the fastest average of one hundred.
    pub fn fastest_ao100_index(&self) -> Option<usize> {
        self.fastest_average_index(100)
    }

    /// Returns the three solves ending at `solve_index`.
    pub fn mo3_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 2 || solve_index >= self.times.len() {
            return None;
        }
        self.times.get(solve_index - 2..=solve_index)
    }

    /// Returns the five solves ending at `solve_index`.
    pub fn ao5_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 4 || solve_index >= self.times.len() {
            return None;
        }
        self.times.get(solve_index - 4..=solve_index)
    }

    /// Returns the twelve solves ending at `solve_index`.
    pub fn ao12_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 11 || solve_index >= self.times.len() {
            return None;
        }
        self.times.get(solve_index - 11..=solve_index)
    }

    /// Returns the fifty solves ending at `solve_index`.
    pub fn ao50_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 49 || solve_index >= self.times.len() {
            return None;
        }
        self.times.get(solve_index - 49..=solve_index)
    }

    /// Returns the hundred solves ending at `solve_index`.
    pub fn ao100_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 99 || solve_index >= self.times.len() {
            return None;
        }
        self.times.get(solve_index - 99..=solve_index)
    }

    /// Renders the scrollable history list with an optional selection highlight.
    pub fn render_with_theme(
        &self,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeColors,
        highlight: Option<bool>,
    ) {
        let highlight = highlight.unwrap_or(true);
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
            let style = if highlight && self.selected.is_some() && i == selected {
                ratatui::style::Style::default()
                    .bg(theme.selection())
                    .fg(theme.selection_text())
            } else {
                ratatui::style::Style::default().fg(theme.text())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scramble::WcaEvent::Cube3x3;

    fn time_with_ms(ms: u64) -> Time {
        Time::new_with_meta(ms, Cube3x3, Cow::Borrowed(""), 0, Modifier::None)
    }

    fn time_with_modifier(ms: u64, modifier: Modifier) -> Time {
        Time::new_with_meta(ms, Cube3x3, Cow::Borrowed(""), 0, modifier)
    }

    fn history(times: Vec<Time>) -> History {
        let mut h = History::new();
        for t in times {
            h.add(t);
        }
        h
    }

    fn reference_average(times: &[Time]) -> AverageValue {
        let mut values: Vec<u64> = times.iter().filter_map(Time::effective_ms).collect();
        let dnf_count = times.len() - values.len();
        if (times.len() == 3 && dnf_count > 0) || dnf_count > 1 {
            return AverageValue::Dnf;
        }
        values.sort_unstable();
        let retained = if times.len() == 3 {
            &values[..]
        } else if dnf_count == 1 {
            &values[1..]
        } else {
            &values[1..values.len() - 1]
        };
        let sum: u128 = retained.iter().map(|&millis| u128::from(millis)).sum();
        AverageValue::Time(u64::try_from(sum / retained.len() as u128).unwrap())
    }

    fn assert_fastest_matches_reference(h: &History) {
        let expected_time = h
            .times()
            .iter()
            .filter_map(|time| time.effective_ms().map(|millis| (millis, time)))
            .min_by_key(|&(millis, _)| millis)
            .map(|(_, time)| time);
        assert_eq!(
            h.get_fastest_time().map(Time::raw_ms),
            expected_time.map(Time::raw_ms)
        );
        for n in AVERAGE_SIZES {
            let expected = h
                .times()
                .windows(n)
                .enumerate()
                .filter_map(|(start, times)| match reference_average(times) {
                    AverageValue::Time(millis) => Some((millis, start + n - 1)),
                    AverageValue::Dnf => None,
                })
                .min_by_key(|&(millis, _)| millis);
            assert_eq!(
                h.fastest_average(n)
                    .map(|best| (best.millis, best.solve_index)),
                expected,
                "fastest average of {n}"
            );
            let expected_value = if h.len() < n {
                None
            } else {
                Some(expected.map_or(Cow::Borrowed("DNF"), |(millis, _)| {
                    Cow::Owned(format_millis(millis))
                }))
            };
            assert_eq!(h.fastest_average_value(n), expected_value);
            assert_eq!(h.fastest_average_index(n), expected.map(|(_, index)| index));
        }
    }

    #[test]
    fn averages_match_sorted_reference_for_all_window_sizes() {
        let h = history(
            (0..240)
                .map(|index| {
                    let modifier = match index % 113 {
                        25 | 26 => Modifier::DNF,
                        value if value % 7 == 0 => Modifier::PlusTwo,
                        _ => Modifier::None,
                    };
                    time_with_modifier(12_300 + (index * 1009) % 23_000, modifier)
                })
                .collect(),
        );
        for n in AVERAGE_SIZES {
            for index in 0..=h.len() {
                let expected = index
                    .checked_sub(n)
                    .map(|start| reference_average(&h.times()[start..index]));
                assert_eq!(h.get_avg(index, n), expected, "n={n}, index={index}");
            }
            assert_eq!(h.get_avg(h.len() + 1, n), None);
        }
        assert_fastest_matches_reference(&h);
    }

    #[test]
    fn fastest_statistics_stay_current_after_appends_modifiers_and_deletions() {
        let mut h = History::new();
        assert_fastest_matches_reference(&h);
        for index in 0..120 {
            let modifier = if index % 11 == 0 {
                Modifier::DNF
            } else {
                Modifier::None
            };
            h.add(time_with_modifier(20_000 - index * 101, modifier));
            assert_fastest_matches_reference(&h);
        }
        for index in [0, 11, 99, 119] {
            h.select_index(index);
            for modifier in [Modifier::DNF, Modifier::DNF, Modifier::PlusTwo] {
                h.set_modifier(modifier);
                assert_fastest_matches_reference(&h);
            }
        }
        for index in [0, 50, 117] {
            h.select_index(index);
            h.delete_selected();
            assert_fastest_matches_reference(&h);
        }
    }

    #[test]
    fn fastest_statistics_keep_first_window_when_averages_tie() {
        let mut h = history(vec![time_with_ms(10_000); 100]);
        assert_fastest_matches_reference(&h);
        h.add(time_with_ms(10_001));
        for n in AVERAGE_SIZES {
            assert_eq!(h.fastest_average_index(n), Some(n - 1));
        }
        assert!(std::ptr::eq(
            h.get_fastest_time().unwrap(),
            h.times().as_ptr()
        ));
    }

    #[test]
    fn fastest_statistics_are_not_persisted_and_rebuild_after_loading() {
        let h = history((1..=120).map(|index| time_with_ms(index * 100)).collect());
        assert_fastest_matches_reference(&h);
        let serialized = serde_json::to_value(&h).unwrap();
        assert_eq!(serialized.as_object().unwrap().len(), 1);
        assert!(serialized.get("times").is_some());
        let mut restored: History = serde_json::from_value(serialized).unwrap();
        assert_fastest_matches_reference(&restored);
        restored.select_index(1);
        restored.set_modifier(Modifier::DNF);
        assert_fastest_matches_reference(&restored);
        assert_fastest_matches_reference(&h);
    }

    #[test]
    fn average_accumulation_handles_large_discarded_values() {
        let h = history(vec![
            time_with_ms(0),
            time_with_ms(0),
            time_with_ms(0),
            time_with_ms(u64::MAX),
            time_with_ms(u64::MAX),
        ]);
        assert_eq!(h.get_ao5(5), Some(AverageValue::Time(u64::MAX / 3)));
        assert_eq!(h.get_ao5(5), Some(reference_average(h.times())));
    }

    #[test]
    fn fastest_mo3_returns_smallest_window_sum() {
        let h = history(vec![
            time_with_ms(10_000),
            time_with_ms(9_000),
            time_with_ms(8_000),
            time_with_ms(5_000),
        ]);
        // Window sums: 27_000, 22_000
        // 27_000/3 = 9_000, 22_000/3 = 7_333
        assert_eq!(h.get_fastest_mo3().unwrap(), "00:07.333");
    }

    #[test]
    fn fastest_mo3_dnf_when_all_dnf() {
        let h = history(vec![
            time_with_modifier(10_000, Modifier::DNF),
            time_with_modifier(9_000, Modifier::DNF),
            time_with_modifier(8_000, Modifier::DNF),
        ]);
        assert_eq!(h.get_fastest_mo3().unwrap(), "DNF");
    }

    #[test]
    fn fastest_time_ignores_a_fast_dnf() {
        let h = history(vec![
            time_with_modifier(100, Modifier::DNF),
            time_with_ms(1_000),
            time_with_ms(2_000),
        ]);

        assert_eq!(h.get_fastest_time().unwrap().raw_ms(), 1_000);
    }

    #[test]
    fn fastest_average_ignores_a_fast_dnf() {
        let h = history(vec![
            time_with_modifier(100, Modifier::DNF),
            time_with_ms(1_000),
            time_with_ms(1_100),
            time_with_ms(1_200),
            time_with_ms(1_300),
        ]);

        assert_eq!(h.get_fastest_ao5().unwrap(), "00:01.200");
    }

    #[test]
    fn fastest_mo3_none_when_too_few_solves() {
        let h = history(vec![time_with_ms(10_000), time_with_ms(9_000)]);
        assert!(h.get_fastest_mo3().is_none());
    }

    #[test]
    fn fastest_ao5_drops_best_and_worst() {
        // Window: 1, 2, 3, 4, 5 (in 100ms units)
        let h = history(vec![
            time_with_ms(100),
            time_with_ms(200),
            time_with_ms(300),
            time_with_ms(400),
            time_with_ms(500),
        ]);
        // trimmed sum: 200 + 300 + 400 = 900, / 3 = 300
        assert_eq!(h.get_fastest_ao5().unwrap(), "00:00.300");
    }

    #[test]
    fn fastest_ao5_handles_one_dnf() {
        let h = history(vec![
            time_with_ms(100),
            time_with_ms(200),
            time_with_modifier(300, Modifier::DNF),
            time_with_ms(400),
            time_with_ms(500),
        ]);
        // drop DNF, then drop best (100) => 200 + 400 + 500 = 1100 / 3 = 366
        assert_eq!(h.get_fastest_ao5().unwrap(), "00:00.366");
    }

    #[test]
    fn fastest_ao5_dnf_when_two_dnfs() {
        let h = history(vec![
            time_with_modifier(100, Modifier::DNF),
            time_with_ms(200),
            time_with_modifier(300, Modifier::DNF),
            time_with_ms(400),
            time_with_ms(500),
        ]);
        assert_eq!(h.get_fastest_ao5().unwrap(), "DNF");
    }

    #[test]
    fn latest_mo3_requires_three_solves() {
        assert!(history(vec![]).latest_mo3_index().is_none());
        assert!(history(vec![time_with_ms(1)]).latest_mo3_index().is_none());
        assert!(
            history(vec![time_with_ms(1), time_with_ms(2)])
                .latest_mo3_index()
                .is_none()
        );
        assert_eq!(
            history(vec![time_with_ms(1), time_with_ms(2), time_with_ms(3)]).latest_mo3_index(),
            Some(2)
        );
    }

    #[test]
    fn latest_ao5_requires_five_solves() {
        assert!(
            history(vec![time_with_ms(1); 4])
                .latest_ao5_index()
                .is_none()
        );
        assert_eq!(
            history(vec![time_with_ms(1); 5]).latest_ao5_index(),
            Some(4)
        );
    }

    #[test]
    fn mo3_times_at_returns_correct_slice() {
        let h = history(vec![
            time_with_ms(1),
            time_with_ms(2),
            time_with_ms(3),
            time_with_ms(4),
            time_with_ms(5),
        ]);
        // mo3_at(3) uses indices 1..=3 (raw_ms 2, 3, 4)
        let times = h.mo3_times_at(3).unwrap();
        assert_eq!(times.len(), 3);
        assert_eq!(times[0].raw_ms(), 2);
        assert_eq!(times[2].raw_ms(), 4);
    }

    #[test]
    fn ao5_times_at_returns_correct_slice() {
        let h = history(vec![time_with_ms(1), time_with_ms(2), time_with_ms(3)]);
        // ao5 at index 2 -> 5 is required
        assert!(h.ao5_times_at(2).is_none());
        let h = history(vec![
            time_with_ms(1),
            time_with_ms(2),
            time_with_ms(3),
            time_with_ms(4),
            time_with_ms(5),
            time_with_ms(6),
        ]);
        // ao5_at(4) uses indices 0..=4 (raw_ms 1..=5)
        let times = h.ao5_times_at(4).unwrap();
        assert_eq!(times.len(), 5);
        assert_eq!(times[0].raw_ms(), 1);
        assert_eq!(times[4].raw_ms(), 5);
    }

    #[test]
    fn fastest_ao12_drops_best_and_worst() {
        // 12 times from 100 to 1200 (in ms)
        let times: Vec<Time> = (1..=12).map(|i| time_with_ms(i * 100)).collect();
        let h = history(times);
        // trimmed sum: 200 + 300 + ... + 1100 = 6500, / 10 = 650
        assert_eq!(h.get_fastest_ao12().unwrap(), "00:00.650");
    }

    #[test]
    fn latest_ao12_requires_twelve_solves() {
        assert!(
            history(vec![time_with_ms(1); 11])
                .latest_ao12_index()
                .is_none()
        );
        assert_eq!(
            history(vec![time_with_ms(1); 12]).latest_ao12_index(),
            Some(11)
        );
    }

    #[test]
    fn ao12_times_at_returns_correct_slice() {
        let h = history((1..=15).map(time_with_ms).collect());
        // ao12_at(14) uses indices 3..=14 (raw_ms 4..=15)
        let times = h.ao12_times_at(14).unwrap();
        assert_eq!(times.len(), 12);
        assert_eq!(times[0].raw_ms(), 4);
        assert_eq!(times[11].raw_ms(), 15);
    }
}
