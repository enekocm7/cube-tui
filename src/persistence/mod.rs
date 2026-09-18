use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use crate::model::Model;
use crate::model::settings::{Settings, ThemeColors};
use crate::widgets::history::History;

/// Returns the application data directory for the current platform.
pub fn data_dir() -> Option<PathBuf> {
    let mut dir = dirs::data_dir()?.join("cube-tui");
    if cfg!(debug_assertions) {
        dir = dir.join("debug");
    }
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Returns the persisted session-history file path.
fn data_file() -> Option<PathBuf> {
    Some(data_dir()?.join("times.json"))
}

/// Returns the directory containing user-editable theme files.
pub fn themes_dir() -> Option<PathBuf> {
    let dir = data_dir()?.join("themes");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Loads a named theme, returning `None` when it is missing or invalid.
pub fn load_theme(name: &str) -> Option<ThemeColors> {
    let mut name = name.to_owned();
    let has_toml_ext = std::path::Path::new(&name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"));
    if !has_toml_ext {
        name.push_str(".toml");
    }
    let path = themes_dir()?.join(name);
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

/// Creates the default theme file if the user does not already have one.
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

/// Returns the application settings file path.
pub fn config_file() -> Option<PathBuf> {
    Some(data_dir()?.join("config.toml"))
}

/// Serializes all session histories without cloning their solve data.
pub fn save(model: &Model) {
    let Some(path) = data_file() else { return };
    let data: Vec<&History> = model.all_sessions_history().collect();

    if let Ok(json) = serde_json::to_string_pretty(&data) {
        fs::write(path, json).ok();
    }
}

/// Loads all persisted session histories.
pub fn load() -> Option<Vec<History>> {
    let path = data_file()?;
    let reader = BufReader::new(File::open(path).ok()?);
    serde_json::from_reader(reader).ok()
}

/// Loads settings, distinguishing a missing file from invalid configuration.
pub fn load_config() -> Result<Option<Settings>, String> {
    ensure_default_theme();
    let Some(path) = config_file() else {
        return Ok(None);
    };
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };
    toml::from_str(&content)
        .map(Some)
        .map_err(|error| format!("invalid configuration in {}: {error}", path.display()))
}

/// Serializes the supplied settings to the platform configuration file.
pub fn save_config(settings: &Settings) {
    let Some(path) = config_file() else { return };

    // Do not replace a user's invalid config with in-memory defaults.
    if path.exists() && load_config().is_err() {
        return;
    }

    if let Ok(toml) = toml::to_string_pretty(settings) {
        fs::write(path, toml).ok();
    }
}
