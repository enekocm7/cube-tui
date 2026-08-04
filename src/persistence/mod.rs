use std::fs;
use std::path::PathBuf;

use crate::model::Model;
use crate::model::settings::{Settings, ThemeColors};
use crate::widgets::history::History;

pub fn data_dir() -> Option<PathBuf> {
    let dir = dirs::data_dir()?.join("cube-tui");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn data_file() -> Option<PathBuf> {
    Some(data_dir()?.join("times.json"))
}

fn themes_dir() -> Option<PathBuf> {
    let dir = data_dir()?.join("themes");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

pub fn load_theme(name: &str) -> Option<ThemeColors> {
    let mut name = name.to_owned();
    let has_toml_ext = std::path::Path::new(&name)
        .extension()
        .is_none_or(|ext| ext.eq_ignore_ascii_case("toml"));
    if !has_toml_ext {
        name.push_str(".toml");
    }
    let path = themes_dir()?.join(name);
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

pub fn ensure_default_theme() {
    let Some(dir) = themes_dir() else { return };
    let path = dir.join("default.toml");
    if path.exists() {
        return;
    }
    if let Ok(toml) = toml::to_string_pretty(&ThemeColors::default()) {
        fs::write(path, toml).ok();
    }
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
    ensure_default_theme();
    let path = config_file()?;
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

pub fn save_config(settings: &Settings) {
    let Some(path) = config_file() else { return };

    if let Ok(toml) = toml::to_string_pretty(settings) {
        fs::write(path, toml).ok();
    }
}
