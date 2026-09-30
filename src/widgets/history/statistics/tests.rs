use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::super::Modifier;
use super::*;
use crate::model::settings::ThemeColors;
use crate::scramble::WcaEvent::Cube3x3;

fn time_with_ms(ms: u64) -> Time {
    Time::new_with_meta(ms, Cube3x3, Cow::Borrowed(""), 0, Modifier::None)
}

#[test]
fn single_record_flags_exclude_average_records_and_persist_in_history() {
    let mut h = history(
        [12_000, 10_000, 14_000, 15_000, 16_000, 8_000]
            .into_iter()
            .map(time_with_ms)
            .collect(),
    );
    let flags: Vec<_> = h.times().iter().map(Time::was_single_record).collect();
    assert_eq!(flags, [true, true, false, false, false, true]);
    assert_eq!(h.ao5_at(4).as_deref(), Some("00:13.666"));
    let encoded = serde_json::to_string(&h).unwrap();
    assert!(encoded.contains("\"was_single_record\":true"));
    assert!(encoded.contains("\"was_single_record\":false"));
    let restored: History = serde_json::from_str(&encoded).unwrap();
    let restored_flags: Vec<_> = restored
        .times()
        .iter()
        .map(Time::was_single_record)
        .collect();
    assert_eq!(restored_flags, flags);

    let mut edited = h.times()[1].snapshot();
    edited.time_ms = 20_000;
    h.edit_solve(1, edited);
    assert!(!h.times()[1].was_single_record());
    h.select_index(0);
    h.delete_selected();
    assert!(h.times()[0].was_single_record());
    assert!(h.times()[1].was_single_record());
}

#[test]
fn saved_single_record_flags_are_restored_and_legacy_solves_default_to_false() {
    let mut h = history(vec![time_with_ms(1_000), time_with_ms(2_000)]);
    // Loading preserves the saved markers rather than recomputing them.
    h.times[0].was_single_record = false;
    h.times[1].was_single_record = true;
    let restored: History = serde_json::from_str(&serde_json::to_string(&h).unwrap()).unwrap();
    assert!(!restored.times()[0].was_single_record());
    assert!(restored.times()[1].was_single_record());

    let mut legacy = serde_json::to_value(time_with_ms(1_000)).unwrap();
    legacy.as_object_mut().unwrap().remove("was_single_record");
    let restored: Time = serde_json::from_value(legacy).unwrap();
    assert!(!restored.was_single_record());
}

#[test]
fn single_record_flags_respect_penalties_dnfs_and_ties() {
    let h = history(vec![
        time_with_ms(10_000),
        time_with_modifier(9_000, Modifier::PlusTwo),
        time_with_modifier(1, Modifier::DNF),
        time_with_ms(9_000),
        time_with_ms(9_000),
    ]);
    let flags: Vec<_> = h.times().iter().map(Time::was_single_record).collect();
    assert_eq!(flags, [true, false, false, true, false]);
}

#[test]
fn record_times_use_accent_while_selection_and_row_numbers_keep_their_style() {
    let h = history(vec![
        time_with_ms(2_000),
        time_with_ms(1_000),
        time_with_ms(3_000),
    ]);
    let theme = ThemeColors::default();
    let area = Rect::new(2, 3, 22, 5);
    let mut buf = Buffer::empty(area);
    h.render_with_theme(area, &mut buf, &theme, Some(true));
    // Earlier records stay accented after a faster single is added.
    assert_eq!(buf[(5, 3)].fg, theme.accent());
    assert_eq!(buf[(2, 4)].fg, theme.text());
    assert_eq!(buf[(5, 4)].fg, theme.accent());
    assert!(
        buf[(5, 4)]
            .modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    // An average record does not accent the single; selection still works.
    assert!(h.latest_average_is_record(3));
    assert_eq!(buf[(5, 5)].fg, theme.selection_text());
    assert_eq!(buf[(5, 5)].bg, theme.selection());
    assert_eq!(buf[(2, 5)].fg, theme.selection_text());
    let mut narrow = Buffer::empty(Rect::new(0, 0, 2, 5));
    h.render_with_theme(narrow.area, &mut narrow, &theme, Some(false));
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
