use std::fs;
use std::path::PathBuf;

use crate::model::Model;
use crate::model::settings::Settings;
use crate::widgets::history::History;

pub fn data_dir() -> Option<PathBuf> {
    let dir = dirs::data_dir()?.join("cube-tui");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn data_file() -> Option<PathBuf> {
    Some(data_dir()?.join("times.json"))
}

pub fn config_file() -> Option<PathBuf> {
    Some(data_dir()?.join("config.toml"))
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

pub fn load_config() -> Option<Settings> {
    let path = config_file()?;
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

pub fn save_config(settings: Settings) {
    let Some(path) = config_file() else { return };

    if let Ok(toml) = toml::to_string_pretty(&settings) {
        fs::write(path, toml).ok();
    }
}
