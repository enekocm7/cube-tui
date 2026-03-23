use std::fs;
use std::path::PathBuf;

use crate::model::{Model, Settings};
use crate::widgets::history::History;

pub fn data_dir() -> Option<PathBuf> {
    let dir = dirs::data_dir()?.join("cube-tui");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn data_file() -> Option<PathBuf> {
    Some(data_dir()?.join("times.json"))
}

fn settings_file() -> Option<PathBuf> {
    Some(data_dir()?.join("options.json"))
}

pub fn save(model: &Model) {
    let Some(path) = data_file() else { return };
    let data = model.all_sessions_history();

    if let Ok(json) = serde_json::to_string_pretty(&data) {
        fs::write(path, json).ok();
    }
}

pub fn load() -> Option<Vec<History>> {
    let path = data_file()?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn load_settings() -> Option<Settings> {
    let path = settings_file()?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_settings(settings: Settings) {
    let Some(path) = settings_file() else { return };

    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        fs::write(path, json).ok();
    }
}
