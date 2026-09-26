use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::model::Model;
use crate::model::settings::{Settings, ThemeColors};
use crate::widgets::history::History;

/// Returns the application data directory for the current platform.
pub fn data_dir() -> Result<PathBuf> {
    let mut dir = dirs::data_dir()
        .context("Could not determine the application data directory")?
        .join("cube-tui");
    if cfg!(debug_assertions) {
        dir = dir.join("debug");
    }
    fs::create_dir_all(&dir).with_context(|| format!("Could not create {}", dir.display()))?;
    Ok(dir)
}

/// Returns the persisted session-history file path.
fn data_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("times.json"))
}

/// Returns the directory containing user-editable theme files.
pub fn themes_dir() -> Result<PathBuf> {
    let dir = data_dir()?.join("themes");
    fs::create_dir_all(&dir).with_context(|| format!("Could not create {}", dir.display()))?;
    Ok(dir)
}

/// Loads a named theme and reports missing, unreadable, or invalid files.
pub fn load_theme(name: &str) -> Result<ThemeColors> {
    let mut name = name.to_owned();
    let has_toml_ext = Path::new(&name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"));
    if !has_toml_ext {
        name.push_str(".toml");
    }
    let path = themes_dir()?.join(name);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Could not read theme {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Invalid theme in {}", path.display()))
}

/// Creates the default theme file if the user does not already have one.
pub fn ensure_default_theme() -> Result<()> {
    let path = themes_dir()?.join("default.toml");
    if path.try_exists()? {
        return Ok(());
    }
    let content = toml::to_string_pretty(&ThemeColors::default())
        .context("Could not serialize the default theme")?;
    fs::write(&path, content)
        .with_context(|| format!("Could not create default theme {}", path.display()))
}

/// Returns the application settings file path.
pub fn config_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("config.toml"))
}

/// Serializes all session histories without cloning their solve data.
pub fn save(model: &Model) -> Result<()> {
    let path = data_file()?;
    let data: Vec<&History> = model.all_sessions_history().collect();
    let json = serde_json::to_string_pretty(&data).context("Could not serialize solve history")?;
    fs::write(&path, json)
        .with_context(|| format!("Could not save solve history to {}", path.display()))
}

/// Loads histories, distinguishing first use from unreadable or invalid data.
pub fn load() -> Result<Option<Vec<History>>> {
    load_history_at(&data_file()?)
}

fn load_history_at(path: &Path) -> Result<Option<Vec<History>>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("Could not read {}", path.display()));
        }
    };
    serde_json::from_reader(BufReader::new(file))
        .map(Some)
        .with_context(|| format!("Invalid solve history in {}", path.display()))
}

/// Loads settings, distinguishing a missing file from invalid configuration.
pub fn load_config() -> Result<Option<Settings>> {
    load_config_at(&config_file()?)
}

fn load_config_at(path: &Path) -> Result<Option<Settings>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("Could not read {}", path.display()));
        }
    };
    toml::from_str(&content)
        .map(Some)
        .with_context(|| format!("Invalid configuration in {}", path.display()))
}

/// Saves settings, preserving an existing file if it cannot be loaded.
pub fn save_config(settings: &Settings) -> Result<()> {
    save_config_at(&config_file()?, settings)
}

fn save_config_at(path: &Path, settings: &Settings) -> Result<()> {
    load_config_at(path)
        .context("Settings were not saved; repair the existing configuration first")?;
    let content = toml::to_string_pretty(settings).context("Could not serialize settings")?;
    fs::write(path, content)
        .with_context(|| format!("Could not save settings to {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn temporary_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "cube-toast-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn history_loading_distinguishes_missing_corrupt_and_unreadable_files() {
        let path = temporary_path();
        assert!(load_history_at(&path).unwrap().is_none());
        fs::write(&path, "not json").unwrap();
        assert!(load_history_at(&path).is_err());
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(load_history_at(&path).is_err());
        fs::remove_dir(&path).unwrap();
    }

    #[test]
    fn invalid_configuration_is_reported_and_preserved_when_saving() {
        let path = temporary_path();
        let original = "[timer\ninvalid configuration";
        fs::write(&path, original).unwrap();
        assert!(load_config_at(&path).is_err());
        assert!(save_config_at(&path, &Settings::default()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn invalid_unicode_theme_colors_are_rejected_without_panicking() {
        assert!(crate::model::settings::ColorSettings::from_hex("#aéabc").is_none());
    }

    #[test]
    fn settings_write_errors_are_reported() {
        let path = temporary_path().join("missing-parent").join("config.toml");
        assert!(save_config_at(&path, &Settings::default()).is_err());
    }
}
