#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::sync::OnceLock;

use super::time::mark_single_records;
use super::{History, Time, format_millis};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AverageValue {
    Time(u64),
    Dnf,
}

pub(super) const AVERAGE_SIZES: [usize; 5] = [3, 5, 12, 50, 100];

#[derive(Clone, Copy, Debug)]
pub(super) struct BestAverage {
    millis: u64,
    solve_index: usize,
}

impl History {
    /// Updates previously requested fastest statistics for the newest solve.
    ///
    /// Uninitialized caches are left untouched so bulk imports do not calculate
    /// statistics that the UI may never request.
    pub(super) fn update_fastest_after_append(&mut self) {
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
    pub(super) fn invalidate_fastest(&mut self) {
        mark_single_records(&mut self.times);
        self.fastest_time.take();
        for cache in &mut self.fastest_averages {
            cache.take();
        }
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
        Some(Self::average_value(attempts.iter(), n))
    }

    fn average_value<'a>(attempts: impl Iterator<Item = &'a Time>, n: usize) -> AverageValue {
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
                return AverageValue::Dnf;
            }
            return AverageValue::Time(u64::try_from(sum / 3).expect("the mean fits in u64"));
        }

        if dnf_count >= 2 {
            return AverageValue::Dnf;
        }

        sum -= u128::from(best);
        if dnf_count == 0 {
            sum -= u128::from(worst);
        }
        AverageValue::Time(u64::try_from(sum / (n - 2) as u128).expect("the mean fits in u64"))
    }

    /// Converts an average result to the text shown in statistics views.
    fn format_average_value(value: AverageValue) -> Cow<'static, str> {
        match value {
            AverageValue::Time(ms) => Cow::Owned(format_millis(ms)),
            AverageValue::Dnf => Cow::Borrowed("DNF"),
        }
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

    /// Whether the latest complete average strictly improved the session best.
    pub fn latest_average_is_record(&self, n: usize) -> bool {
        self.times
            .len()
            .checked_sub(1)
            .is_some_and(|latest| self.fastest_average_index(n) == Some(latest))
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
}
