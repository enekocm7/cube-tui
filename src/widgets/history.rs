use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};

use crate::model::settings::ThemeSettings;
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

    pub fn scramble(&self) -> &str {
        &self.scramble
    }

    pub const fn solved_at_unix_ms(&self) -> u64 {
        self.solved_at_unix_ms
    }

    pub const fn raw_ms(&self) -> u64 {
        self.timestamp_in_millis
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
}

impl History {
    pub const fn new() -> Self {
        Self {
            times: Vec::new(),
            selected: None,
        }
    }

    pub fn add_ms(
        &mut self,
        timestamp_in_millis: u64,
        event: WcaEvent,
        scramble: impl Into<Cow<'static, str>>,
    ) {
        self.add(Time::new(timestamp_in_millis, event, scramble));
    }

    pub fn add(&mut self, item: Time) {
        self.times.push(item);
        self.selected = Some(self.times.len() - 1);
    }

    pub fn times(&self) -> &[Time] {
        &self.times
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

    pub fn get_latest_mo3(&self) -> Option<Cow<'static, str>> {
        self.get_mo3(self.times.len())
            .map(Self::format_average_value)
    }

    pub fn get_fastest_mo3(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(3)
    }

    pub fn get_latest_ao5(&self) -> Option<Cow<'static, str>> {
        self.get_ao5(self.times.len())
            .map(Self::format_average_value)
    }

    pub fn get_fastest_ao5(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(5)
    }

    pub fn get_latest_ao12(&self) -> Option<Cow<'static, str>> {
        self.get_ao12(self.times.len())
            .map(Self::format_average_value)
    }

    pub fn get_fastest_ao12(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(12)
    }

    pub fn get_latest_ao50(&self) -> Option<Cow<'static, str>> {
        self.get_ao50(self.times.len())
            .map(Self::format_average_value)
    }

    pub fn get_fastest_ao50(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(50)
    }

    pub fn get_latest_ao100(&self) -> Option<Cow<'static, str>> {
        self.get_ao100(self.times.len())
            .map(Self::format_average_value)
    }

    pub fn get_fastest_ao100(&self) -> Option<Cow<'static, str>> {
        self.fastest_average_value(100)
    }

    fn fastest_average_value(&self, n: usize) -> Option<Cow<'static, str>> {
        let mut fastest = u64::MAX;
        let mut any_valid = false;
        let mut any_computable = false;

        for index in n..=self.times.len() {
            any_computable = true;
            if let Some(AverageValue::Time(value)) = self.get_avg(index, n) {
                any_valid = true;
                fastest = fastest.min(value);
            }
        }

        if any_valid {
            return Some(Cow::Owned(format_millis(fastest)));
        }
        if any_computable {
            return Some(Cow::Borrowed("DNF"));
        }
        None
    }

    fn get_mo3(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 3)
    }

    fn get_ao5(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 5)
    }

    fn get_ao12(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 12)
    }

    fn get_ao50(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 50)
    }

    fn get_ao100(&self, index: usize) -> Option<AverageValue> {
        self.get_avg(index, 100)
    }

    fn get_avg(&self, index: usize, n: usize) -> Option<AverageValue> {
        if index < n {
            return None;
        }

        let attempts = self.times.get(index.saturating_sub(n)..index)?;
        let mut vals: Vec<Option<u64>> = vec![None; n];
        let mut dnf_count = 0;
        for (i, t) in attempts.iter().enumerate() {
            vals[i] = Self::effective_millis(t);
            if vals[i].is_none() {
                dnf_count += 1;
            }
        }

        if n == 3 {
            if dnf_count > 0 {
                return Some(AverageValue::Dnf);
            }
            let sum: u64 = vals.iter().map(|v| v.unwrap()).sum();
            return Some(AverageValue::Time(sum / 3));
        }

        if dnf_count >= 2 {
            return Some(AverageValue::Dnf);
        }

        let valid: Vec<u64> = vals.iter().flatten().copied().collect();
        let count = u64::try_from(n - 2).expect("n is small");

        if dnf_count == 1 {
            let best = valid.iter().min().copied().unwrap_or(0);
            let sum: u64 = valid.iter().sum::<u64>() - best;
            return Some(AverageValue::Time(sum / count));
        }

        let mut sorted = valid;
        sorted.sort_unstable();
        let sum: u64 = sorted[1..n - 1].iter().sum();
        Some(AverageValue::Time(sum / count))
    }

    const fn effective_millis(time: &Time) -> Option<u64> {
        match time.modifier {
            Modifier::None => Some(time.timestamp_in_millis),
            Modifier::PlusTwo => Some(time.timestamp_in_millis + 2000),
            Modifier::DNF => None,
        }
    }

    fn format_average_value(value: AverageValue) -> Cow<'static, str> {
        match value {
            AverageValue::Time(ms) => Cow::Owned(format_millis(ms)),
            AverageValue::Dnf => Cow::Borrowed("DNF"),
        }
    }

    pub const fn len(&self) -> usize {
        self.times.len()
    }

    pub fn get_time_at(&self, index: usize) -> Option<&Time> {
        self.times.get(index)
    }

    pub fn mo3_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_mo3(solve_index + 1)
            .map(Self::format_average_value)
    }

    pub fn ao5_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_ao5(solve_index + 1)
            .map(Self::format_average_value)
    }

    pub fn ao12_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_ao12(solve_index + 1)
            .map(Self::format_average_value)
    }

    pub fn ao50_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_ao50(solve_index + 1)
            .map(Self::format_average_value)
    }

    pub fn ao100_at(&self, solve_index: usize) -> Option<Cow<'static, str>> {
        self.get_ao100(solve_index + 1)
            .map(Self::format_average_value)
    }

    pub const fn latest_mo3_index(&self) -> Option<usize> {
        if self.times.len() >= 3 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    pub const fn latest_ao5_index(&self) -> Option<usize> {
        if self.times.len() >= 5 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    pub const fn latest_ao12_index(&self) -> Option<usize> {
        if self.times.len() >= 12 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    pub const fn latest_ao50_index(&self) -> Option<usize> {
        if self.times.len() >= 50 {
            Some(self.times.len() - 1)
        } else {
            None
        }
    }

    pub const fn latest_ao100_index(&self) -> Option<usize> {
        if self.times.len() >= 100 {
            Some(self.times.len() - 1)
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

    pub fn fastest_ao12_index(&self) -> Option<usize> {
        self.fastest_average_index(12, Self::get_ao12)
    }

    pub fn fastest_ao50_index(&self) -> Option<usize> {
        self.fastest_average_index(50, Self::get_ao50)
    }

    pub fn fastest_ao100_index(&self) -> Option<usize> {
        self.fastest_average_index(100, Self::get_ao100)
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

    pub fn ao12_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 11 || solve_index >= self.times.len() {
            return None;
        }
        self.get_ao12(solve_index + 1)?;
        self.times.get(solve_index - 11..=solve_index)
    }

    pub fn ao50_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 49 || solve_index >= self.times.len() {
            return None;
        }
        self.get_ao50(solve_index + 1)?;
        self.times.get(solve_index - 49..=solve_index)
    }

    pub fn ao100_times_at(&self, solve_index: usize) -> Option<&[Time]> {
        if solve_index < 99 || solve_index >= self.times.len() {
            return None;
        }
        self.get_ao100(solve_index + 1)?;
        self.times.get(solve_index - 99..=solve_index)
    }

    pub fn render_with_theme(
        &self,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeSettings,
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
