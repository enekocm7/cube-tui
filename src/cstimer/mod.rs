use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::model::Model;
use crate::scramble::classify_event;
use crate::widgets::history::{History, Modifier, Time};

#[derive(Debug, Clone)]
pub struct CstimerFile {
    pub sessions: BTreeMap<usize, Vec<CstimerSolve>>,
}

#[derive(Debug, Clone)]
pub struct CstimerSolve {
    pub penalty_ms: i64,
    pub time_ms: u64,
    pub scramble: String,
    pub comment: String,
    pub timestamp_unix: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct CstimerSolveRaw([i64; 2], String, String, u64);

#[derive(Debug, Clone, Serialize)]
struct CstimerSolveExport([i64; 2], String, String, u64);

impl<'de> Deserialize<'de> for CstimerSolve {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = CstimerSolveRaw::deserialize(deserializer)?;
        let time_ms = u64::try_from(raw.0[1]).map_err(serde::de::Error::custom)?;
        Ok(Self {
            penalty_ms: raw.0[0],
            time_ms,
            scramble: raw.1,
            comment: raw.2,
            timestamp_unix: raw.3,
        })
    }
}

impl<'de> Deserialize<'de> for CstimerFile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SessionsVisitor;

        impl<'de> Visitor<'de> for SessionsVisitor {
            type Value = CstimerFile;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map of csTimer sessions")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut sessions = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    if let Some(index) = parse_session_index(&key) {
                        sessions.insert(index, map.next_value()?);
                    } else {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
                Ok(CstimerFile { sessions })
            }
        }

        deserializer.deserialize_map(SessionsVisitor)
    }
}

fn parse_session_index(key: &str) -> Option<usize> {
    let suffix = key.strip_prefix("session")?;
    suffix.parse().ok()
}

const fn normalize_timestamp_ms(timestamp: u64) -> u64 {
    if timestamp < 1_000_000_000_000 {
        timestamp * 1000
    } else {
        timestamp
    }
}

const fn normalize_timestamp_seconds(timestamp_ms: u64) -> u64 {
    if timestamp_ms >= 1_000_000_000_000 {
        timestamp_ms / 1000
    } else {
        timestamp_ms
    }
}

/// Imports a csTimer JSON export file into session histories.
///
/// Only `sessionN` arrays are parsed; other keys (like `properties`) are ignored.
/// Scrambles are used to infer the event, and timestamps are normalized to
/// milliseconds when needed.
///
/// # Errors
/// Returns an error if the file cannot be read or the JSON is invalid.
pub fn import(path: &Path) -> anyhow::Result<impl Iterator<Item = History>> {
    let reader = BufReader::new(File::open(path)?);
    let parsed: CstimerFile = serde_json::from_reader(reader)?;

    Ok(parsed.sessions.into_values().map(|solves| {
        let mut history = History::new();
        for solve in solves {
            let modifier = match solve.penalty_ms {
                -1 => Modifier::DNF,
                penalty if penalty > 0 => Modifier::PlusTwo,
                _ => Modifier::None,
            };
            let solved_at_ms = normalize_timestamp_ms(solve.timestamp_unix);
            let event = classify_event(&solve.scramble);
            let time = Time::new_with_meta(
                solve.time_ms,
                event,
                solve.scramble.into(),
                solved_at_ms,
                modifier,
            );
            history.add(time);
        }
        history
    }))
}

/// Exports the current model sessions to a csTimer JSON file.
///
/// Each session is written as `sessionN`, and solves include penalties, scramble,
/// and timestamps normalized to seconds.
///
/// # Errors
/// Returns an error if the file cannot be written or serialization fails.
pub fn export(path: &Path, model: &Model) -> anyhow::Result<PathBuf> {
    let histories = model.all_sessions_history();
    let mut root = serde_json::Map::new();
    for (index, history) in histories.enumerate() {
        let key = format!("session{}", index + 1);
        let mut solves_export = Vec::new();
        for time in history.times() {
            let penalty_ms = match time.modifier() {
                Modifier::None => 0,
                Modifier::PlusTwo => 2000,
                Modifier::DNF => -1,
            };
            let time_ms = i64::try_from(time.raw_ms()).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Time too large")
            })?;
            let timestamp = normalize_timestamp_seconds(time.solved_at_unix_ms());
            solves_export.push(CstimerSolveExport(
                [penalty_ms, time_ms],
                time.scramble().to_string(),
                String::new(),
                timestamp,
            ));
        }
        root.insert(key, serde_json::to_value(solves_export)?);
    }

    let json = serde_json::to_string_pretty(&serde_json::Value::Object(root))?;
    let export_path = match path.extension().and_then(|ext| ext.to_str()) {
        Some("txt") => path.to_path_buf(),
        _ => path.with_extension("json"),
    };
    std::fs::write(export_path.clone(), json)?;
    Ok(export_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    fn write_temp_json(json: &str) -> PathBuf {
        // Test thread name is unique per rstest case; sanitize for Windows paths.
        let name = std::thread::current()
            .name()
            .unwrap_or("test")
            .replace([':', '/', '\\'], "_");
        let path = std::env::temp_dir().join(format!("{name}.json"));
        std::fs::write(&path, json).unwrap();
        path
    }

    fn import_histories(path: &Path) -> Vec<History> {
        import(path).unwrap().collect()
    }

    #[rstest]
    #[case::out_of_order(
        r#"{
            "session3": [[[0, 3000], "R U", "", 1700000000]],
            "session1": [[[0, 1000], "R U", "", 1700000100]],
            "session10": [[[0, 10000], "R U", "", 1700000200]],
            "session2": [[[0, 2000], "R U", "", 1700000300]],
            "properties": {"foo": "bar"}
        }"#,
        &[1000, 2000, 3000, 10000]
    )]
    #[case::already_sorted(
        r#"{
            "session1": [[[0, 111], "R U", "", 1700000000]],
            "session2": [[[0, 222], "R U", "", 1700000001]],
            "session3": [[[0, 333], "R U", "", 1700000002]]
        }"#,
        &[111, 222, 333]
    )]
    #[case::gaps(
        r#"{
            "session5": [[[0, 500], "R U", "", 1700000000]],
            "session1": [[[0, 100], "R U", "", 1700000001]]
        }"#,
        &[100, 500]
    )]
    fn import_sorts_sessions_by_index(#[case] json: &str, #[case] expected_times: &[u64]) {
        let path = write_temp_json(json);
        let times: Vec<u64> = import_histories(&path)
            .iter()
            .map(|h| h.times()[0].raw_ms())
            .collect();
        std::fs::remove_file(&path).ok();
        assert_eq!(times, expected_times);
    }

    #[rstest]
    #[case::none(0, Modifier::None)]
    #[case::plus_two(2000, Modifier::PlusTwo)]
    #[case::plus_two_other(1, Modifier::PlusTwo)]
    #[case::dnf(-1, Modifier::DNF)]
    fn import_maps_penalties(#[case] penalty_ms: i64, #[case] expected: Modifier) {
        let path = write_temp_json(&format!(
            r#"{{"session1": [[[{penalty_ms}, 1234], "R U", "", 1700000000]]}}"#
        ));
        let histories = import_histories(&path);
        std::fs::remove_file(&path).ok();

        assert_eq!(histories.len(), 1);
        assert_eq!(histories[0].times().len(), 1);
        assert_eq!(histories[0].times()[0].raw_ms(), 1234);
        assert_eq!(histories[0].times()[0].modifier(), expected);
    }

    #[rstest]
    #[case::seconds(1_700_000_000, 1_700_000_000_000)]
    #[case::millis(1_700_000_000_000, 1_700_000_000_000)]
    #[case::boundary_seconds(999_999_999_999, 999_999_999_999_000)]
    fn import_normalizes_timestamps(#[case] input_ts: u64, #[case] expected_ms: u64) {
        let path = write_temp_json(&format!(
            r#"{{"session1": [[[0, 1000], "R U", "", {input_ts}]]}}"#
        ));
        let histories = import_histories(&path);
        std::fs::remove_file(&path).ok();

        assert_eq!(histories[0].times()[0].solved_at_unix_ms(), expected_ms);
    }

    #[rstest]
    #[case::properties_only(r#"{"properties": {"sessionData": "{}"}}"#, 0)]
    #[case::mixed_keys(
        r#"{
            "properties": {"foo": 1},
            "session1": [[[0, 1000], "R U", "", 1700000000]],
            "notASession": [[[0, 9999], "R U", "", 1700000000]],
            "sessionX": [[[0, 8888], "R U", "", 1700000000]]
        }"#,
        1
    )]
    #[case::empty_object("{}", 0)]
    fn import_ignores_non_session_keys(#[case] json: &str, #[case] expected_sessions: usize) {
        let path = write_temp_json(json);
        let histories = import_histories(&path);
        std::fs::remove_file(&path).ok();
        assert_eq!(histories.len(), expected_sessions);
    }

    #[rstest]
    #[case::multiple_solves(
        r#"{
            "session1": [
                [[0, 1000], "R U", "", 1700000000],
                [[2000, 2000], "R U R", "", 1700000001],
                [[-1, 3000], "R U R'", "", 1700000002]
            ]
        }"#,
        &[1000, 2000, 3000]
    )]
    #[case::empty_session(r#"{"session1": []}"#, &[])]
    fn import_preserves_solves_within_session(
        #[case] json: &str,
        #[case] expected_times: &[u64],
    ) {
        let path = write_temp_json(json);
        let histories = import_histories(&path);
        std::fs::remove_file(&path).ok();

        assert_eq!(histories.len(), 1);
        let times: Vec<u64> = histories[0].times().iter().map(Time::raw_ms).collect();
        assert_eq!(times, expected_times);
    }
}
