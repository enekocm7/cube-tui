use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};

use crate::model::Model;
use crate::scramble::classify_event;
use crate::widgets::history::{History, Modifier, Time};

#[derive(Debug, Clone)]
pub struct CstimerFile {
    pub sessions: HashMap<String, Vec<CstimerSolve>>,
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
        let raw: HashMap<String, serde_json::Value> = HashMap::deserialize(deserializer)?;
        let mut sessions = HashMap::new();
        for (key, value) in raw {
            if parse_session_index(&key).is_none() {
                continue;
            }
            let solves = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            sessions.insert(key, solves);
        }
        Ok(Self { sessions })
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
pub fn import(path: &Path) -> anyhow::Result<Vec<History>> {
    let path_string = std::fs::read_to_string(path)?;
    let parsed: CstimerFile = serde_json::from_str(&path_string)?;
    let mut sessions: Vec<(usize, Vec<CstimerSolve>)> = parsed
        .sessions
        .into_iter()
        .filter_map(|(key, value)| parse_session_index(&key).map(|index| (index, value)))
        .collect();
    sessions.sort_by_key(|(index, _)| *index);

    let histories = sessions
        .into_iter()
        .map(|(_, solves)| {
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
                    solve.scramble,
                    solved_at_ms,
                    modifier,
                );
                history.add(time);
            }
            history
        })
        .collect();

    Ok(histories)
}

/// Exports the current model sessions to a csTimer JSON file.
///
/// Each session is written as `sessionN`, and solves include penalties, scramble,
/// and timestamps normalized to seconds.
///
/// # Errors
/// Returns an error if the file cannot be written or serialization fails.
pub fn export(path: &Path, model: &Model) -> anyhow::Result<()> {
    let histories = model.all_sessions_history();
    let mut root = serde_json::Map::new();
    for (index, history) in histories.iter().enumerate() {
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
    std::fs::write(path, json)?;
    Ok(())
}