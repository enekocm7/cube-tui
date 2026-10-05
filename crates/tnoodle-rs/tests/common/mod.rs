//! Helpers shared by the differential tests.
//!
//! The fixtures in `tests/fixtures` were produced by running the original Java TNoodle with
//! seeded randomness; these helpers load them.

#![allow(dead_code)]

use std::path::PathBuf;

use serde_json::Value;

/// Loads `tests/fixtures/<name>.json`.
pub fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

pub fn as_str(v: &Value) -> &str {
    v.as_str()
        .unwrap_or_else(|| panic!("expected a string, got {v}"))
}

pub fn opt_str(v: &Value) -> Option<&str> {
    if v.is_null() { None } else { Some(as_str(v)) }
}

pub fn as_i64(v: &Value) -> i64 {
    v.as_i64()
        .unwrap_or_else(|| panic!("expected an integer, got {v}"))
}

pub fn as_i32(v: &Value) -> i32 {
    i32::try_from(as_i64(v)).expect("fits in i32")
}

pub fn as_array(v: &Value) -> &Vec<Value> {
    v.as_array()
        .unwrap_or_else(|| panic!("expected an array, got {v}"))
}

pub fn strings(v: &Value) -> Vec<String> {
    as_array(v).iter().map(|s| as_str(s).to_owned()).collect()
}

pub fn ints(v: &Value) -> Vec<i32> {
    as_array(v).iter().map(as_i32).collect()
}
