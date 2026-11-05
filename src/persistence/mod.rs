//! Reading and writing the data directory.
//!
//! `times.json` is shared with the desktop app, cube-app, and so is the
//! `backups` folder beside it: both apps use the same names and timing there.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};

use crate::model::Model;
use crate::model::settings::{Settings, ThemeColors};
use crate::widgets::history::History;

/// How many times a rename is tried when another program may be in the way.
const ATTEMPTS: u32 = 5;
/// The wait between attempts; the UI thread waits this, so it stays short.
const RETRY_DELAY: Duration = Duration::from_millis(50);
/// The least time between two automatic backups, so a busy session does not
/// rotate older backups away.
const BACKUP_INTERVAL: TimeDelta = TimeDelta::hours(6);
/// The `chrono` format of the UTC time in a backup's name.
const BACKUP_TIME_FORMAT: &str = "%Y%m%dT%H%M%SZ";
/// How old a leftover temporary file must be to count as abandoned.
const STALE_TEMPORARY_AGE: Duration = Duration::from_hours(1);

/// Returns the application data directory for the current platform.
///
/// `CUBE_DATA_DIR` replaces the location entirely, as it does in cube-app.
pub fn data_dir() -> Result<PathBuf> {
    let dir = if let Some(dir) = std::env::var_os("CUBE_DATA_DIR") {
        PathBuf::from(dir)
    } else {
        let dir = dirs::data_dir()
            .context("Could not determine the application data directory")?
            .join("cube-tui");
        if cfg!(debug_assertions) {
            dir.join("debug")
        } else {
            dir
        }
    };
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
///
/// Returns the reason no backup was made, if any: that does not stop the save.
pub fn save(model: &Model) -> Result<Option<anyhow::Error>> {
    let data: Vec<&History> = model.all_sessions_history().collect();
    let json = serde_json::to_vec_pretty(&data).context("Could not serialize solve history")?;
    save_at(&data_file()?, &json, model.settings().backup_limit())
}

/// Replaces the history at `path` with `json`, backing up what it held first.
///
/// A file that cannot be read is left alone, and `json` goes to a sibling file
/// so the solves survive the app closing.
fn save_at(path: &Path, json: &[u8], backup_limit: usize) -> Result<Option<anyhow::Error>> {
    let current = read_valid(path).map_err(|error| keep_unsaved(path, json, error))?;
    if current.as_deref() == Some(json) {
        return Ok(None);
    }
    let backup_error = current.and_then(|bytes| {
        let dir = backups_dir(path);
        back_up_if_due(&dir, &bytes, Utc::now(), backup_limit)
            .err()
            .map(|error| backup_failure(&dir, error))
    });
    write_atomic(path, json)
        .with_context(|| format!("Could not save solve history to {}", path.display()))?;
    Ok(backup_error)
}

/// Writes `json` beside the unreadable history and describes where.
fn keep_unsaved(path: &Path, json: &[u8], error: anyhow::Error) -> anyhow::Error {
    let unsaved = path.with_extension("unsaved.json");
    match write_atomic(&unsaved, json) {
        Ok(()) => error.context(format!("Your solves were kept in {}", unsaved.display())),
        Err(_) => error,
    }
}

fn backup_failure(dir: &Path, error: io::Error) -> anyhow::Error {
    anyhow::Error::new(error).context(format!("Could not write a backup to {}", dir.display()))
}

/// Loads histories, distinguishing first use from unreadable or invalid data.
pub fn load() -> Result<Option<Vec<History>>> {
    load_history_at(&data_file()?)
}

fn load_history_at(path: &Path) -> Result<Option<Vec<History>>> {
    let bytes =
        read_if_present(path).with_context(|| format!("Could not read {}", path.display()))?;
    bytes.map(|bytes| parse_history(&bytes, path)).transpose()
}

/// A history loaded when the app starts.
pub struct Opened {
    pub history: Option<Vec<History>>,
    /// Why the history as it was could not be backed up, if it could not.
    pub backup_error: Option<anyhow::Error>,
}

/// Loads the history for a run of the app, backing it up when one is due and
/// clearing away what an interrupted write left behind.
pub fn open(backup_limit: usize) -> Result<Opened> {
    open_at(&data_file()?, backup_limit)
}

fn open_at(path: &Path, backup_limit: usize) -> Result<Opened> {
    remove_stale_temporaries(path);
    let bytes =
        read_if_present(path).with_context(|| format!("Could not read {}", path.display()))?;
    let Some(bytes) = bytes else {
        return Ok(Opened {
            history: None,
            backup_error: None,
        });
    };
    let history = parse_history(&bytes, path)?;
    let dir = backups_dir(path);
    let backup_error = back_up_if_due(&dir, &bytes, Utc::now(), backup_limit)
        .err()
        .map(|error| backup_failure(&dir, error));
    Ok(Opened {
        history: Some(history),
        backup_error,
    })
}

fn parse_history(bytes: &[u8], path: &Path) -> Result<Vec<History>> {
    serde_json::from_slice(bytes)
        .with_context(|| format!("Invalid solve history in {}", path.display()))
}

/// Reads `path`; `None` when it does not exist.
fn read_if_present(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Reads the history at `path`, failing unless it parses or does not exist.
fn read_valid(path: &Path) -> Result<Option<Vec<u8>>> {
    let bytes =
        read_if_present(path).with_context(|| format!("Could not read {}", path.display()))?;
    if let Some(bytes) = &bytes {
        parse_history(bytes, path)?;
    }
    Ok(bytes)
}

/// Returns the folder of backups for the history at `path`.
fn backups_dir(path: &Path) -> PathBuf {
    path.with_file_name("backups")
}

/// Lists the backups in `dir`, oldest first. Other files are ignored.
fn list_backups(dir: &Path) -> io::Result<Vec<(DateTime<Utc>, PathBuf)>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut backups = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let time = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix("times-")?.strip_suffix(".json"))
            .and_then(|time| NaiveDateTime::parse_from_str(time, BACKUP_TIME_FORMAT).ok());
        if let Some(time) = time {
            backups.push((time.and_utc(), path));
        }
    }
    backups.sort();
    Ok(backups)
}

/// Copies `contents` into `dir` unless a recent backup exists. Only ever given
/// a history that parsed, so a damaged file cannot push good backups out.
fn back_up_if_due(dir: &Path, contents: &[u8], now: DateTime<Utc>, limit: usize) -> io::Result<()> {
    if let Some((newest, _)) = list_backups(dir)?.last() {
        let age = now - *newest;
        if age >= TimeDelta::zero() && age < BACKUP_INTERVAL {
            return Ok(());
        }
    }
    write_backup(dir, contents, now, limit).map(|_| ())
}

/// Copies `contents` into `dir` under a name of its own, then drops the oldest
/// other backups past `limit`.
fn write_backup(
    dir: &Path,
    contents: &[u8],
    now: DateTime<Utc>,
    limit: usize,
) -> io::Result<PathBuf> {
    let backups = list_backups(dir)?;
    fs::create_dir_all(dir)?;
    let mut time = now;
    let path = loop {
        let path = dir.join(format!("times-{}.json", time.format(BACKUP_TIME_FORMAT)));
        if !path.try_exists()? {
            break path;
        }
        time += TimeDelta::seconds(1);
    };
    write_atomic(&path, contents)?;
    let excess = (backups.len() + 1).saturating_sub(limit).min(backups.len());
    for (_, old) in &backups[..excess] {
        fs::remove_file(old)?;
    }
    Ok(path)
}

/// Replaces the whole history with `contents`, as an import does, first
/// keeping what the file held. Returns where it went, if anywhere.
pub fn replace_history(contents: &[u8], backup_limit: usize) -> Result<Option<PathBuf>> {
    replace_history_at(&data_file()?, contents, backup_limit)
}

fn replace_history_at(path: &Path, contents: &[u8], limit: usize) -> Result<Option<PathBuf>> {
    let current =
        read_if_present(path).with_context(|| format!("Could not read {}", path.display()))?;
    let kept = match current {
        Some(bytes) if bytes == contents => return Ok(None),
        Some(bytes) if parse_history(&bytes, path).is_ok() => {
            let dir = backups_dir(path);
            let backup = write_backup(&dir, &bytes, Utc::now(), limit)
                .map_err(|error| backup_failure(&dir, error))?;
            Some(backup)
        }
        // Damaged files stay out of the backups, which must all be restorable.
        Some(_) => {
            Some(keep_invalid(path).with_context(|| format!("Could not keep {}", path.display()))?)
        }
        None => None,
    };
    write_atomic(path, contents)
        .with_context(|| format!("Could not save solve history to {}", path.display()))?;
    Ok(kept)
}

/// Copies a file that could not be read to a name of its own.
fn keep_invalid(path: &Path) -> io::Result<PathBuf> {
    let mut kept = OsString::from(path.as_os_str());
    kept.push(format!(
        ".invalid-{}",
        Utc::now().format(BACKUP_TIME_FORMAT)
    ));
    let kept = PathBuf::from(kept);
    fs::copy(path, &kept)?;
    Ok(kept)
}

/// A backup as `cube backups` shows it.
pub struct BackupEntry {
    pub time: DateTime<Utc>,
    pub path: PathBuf,
    /// The sessions and solves it holds, or why it cannot be restored.
    pub contents: Result<(usize, usize)>,
}

/// Returns the folder holding the automatic backups.
pub fn backups_folder() -> Result<PathBuf> {
    Ok(backups_dir(&data_file()?))
}

/// Lists the backups newest first, the order `cube restore` numbers them in.
pub fn backups() -> Result<Vec<BackupEntry>> {
    backups_at(&data_file()?)
}

fn backups_at(path: &Path) -> Result<Vec<BackupEntry>> {
    let dir = backups_dir(path);
    let backups =
        list_backups(&dir).with_context(|| format!("Could not read {}", dir.display()))?;
    Ok(backups
        .into_iter()
        .rev()
        .map(|(time, path)| {
            let contents = read_backup(&path).map(|sessions| {
                let solves = sessions.iter().map(History::len).sum();
                (sessions.len(), solves)
            });
            BackupEntry {
                time,
                path,
                contents,
            }
        })
        .collect())
}

fn read_backup(path: &Path) -> Result<Vec<History>> {
    let bytes = fs::read(path).with_context(|| format!("Could not read {}", path.display()))?;
    parse_history(&bytes, path)
}

/// The result of a restore.
pub struct Restored {
    pub backup: PathBuf,
    /// Where the history that was replaced went, if there was one.
    pub kept: Option<PathBuf>,
}

/// Restores the backup `selector` names: its number in `cube backups`, or its
/// file name.
pub fn restore(selector: &str, backup_limit: usize) -> Result<Restored> {
    restore_at(&data_file()?, selector, backup_limit)
}

fn restore_at(path: &Path, selector: &str, limit: usize) -> Result<Restored> {
    let dir = backups_dir(path);
    let mut backups =
        list_backups(&dir).with_context(|| format!("Could not read {}", dir.display()))?;
    backups.reverse();
    let found = match selector.parse::<usize>() {
        Ok(number) => number.checked_sub(1).and_then(|index| backups.get(index)),
        Err(_) => backups
            .iter()
            .find(|(_, backup)| backup.file_name().is_some_and(|name| name == selector)),
    };
    let Some((_, backup)) = found else {
        bail!("No backup matches `{selector}`. Run `cube backups` to list them");
    };
    let contents =
        fs::read(backup).with_context(|| format!("Could not read {}", backup.display()))?;
    parse_history(&contents, backup).context("The backup cannot be restored")?;
    let kept = replace_history_at(path, &contents, limit)?;
    Ok(Restored {
        backup: backup.clone(),
        kept,
    })
}

/// Returns the file name of the newest backup, to offer when the history
/// cannot be read.
pub fn latest_backup() -> Option<String> {
    let (_, newest) = list_backups(&backups_dir(&data_file().ok()?)).ok()?.pop()?;
    Some(newest.file_name()?.to_string_lossy().into_owned())
}

/// Writes through a sibling temporary file, so a crash or power cut leaves
/// either the old contents or the new, never a mix.
fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    write_atomic_with(path, contents, replace)
}

fn write_atomic_with(
    path: &Path,
    contents: &[u8],
    replace: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let temporary = temporary_sibling(path);
    let written = write_synced(&temporary, contents).and_then(|()| replace(&temporary, path));
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written?;
    sync_parent(path);
    Ok(())
}

/// Returns a temporary name no other write is using, even in another process.
fn temporary_sibling(path: &Path) -> PathBuf {
    static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);
    let mut temporary = OsString::from(path.as_os_str());
    temporary.push(format!(
        ".{}-{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    PathBuf::from(temporary)
}

/// Writes `path` and waits for the data to reach the disk, so the rename that
/// follows cannot land first and leave an empty file after a power cut.
fn write_synced(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

/// Renames `from` over `to`. Windows refuses while anything has `to` open, such
/// as cube-app reading it or an antivirus scan, so this waits a moment for that.
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    let mut attempt = 1;
    loop {
        match fs::rename(from, to) {
            Err(error) if error.kind() != ErrorKind::NotFound && attempt < ATTEMPTS => {
                attempt += 1;
                thread::sleep(RETRY_DELAY);
            }
            result => return result,
        }
    }
}

/// Makes a rename inside the parent folder durable. Windows has no equivalent,
/// and the file is already safe by now, so a failure here changes nothing.
fn sync_parent(path: &Path) {
    if cfg!(unix)
        && let Some(parent) = path.parent()
        && let Ok(parent) = File::open(parent)
    {
        let _ = parent.sync_all();
    }
}

/// Deletes temporary files a crash left beside `path`. Recent ones are left
/// alone, since they may belong to a write still in progress.
fn remove_stale_temporaries(path: &Path) {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    let prefix = format!("{}.", name.to_string_lossy());
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let abandoned = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age >= STALE_TEMPORARY_AGE);
        if abandoned && file_name.starts_with(&prefix) && file_name.ends_with(".tmp") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Loads settings, distinguishing a missing file from invalid configuration.
pub fn load_config() -> Result<Option<Settings>> {
    load_config_at(&config_file()?)
}

/// Returns the configured backup limit, or the default without a usable config.
pub fn backup_limit() -> usize {
    load_config()
        .ok()
        .flatten()
        .unwrap_or_default()
        .backup_limit()
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

    /// A folder of its own holding a history that does not exist yet.
    fn temporary_history() -> PathBuf {
        let dir = temporary_path();
        fs::create_dir(&dir).unwrap();
        dir.join("times.json")
    }

    fn remove_folder_of(path: &Path) {
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// A history whose session name tells it apart.
    fn json(name: &str) -> Vec<u8> {
        let mut history = History::new();
        history.set_session_name(name.to_owned());
        serde_json::to_vec_pretty(&[history]).unwrap()
    }

    fn at(hours: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_760_000_000, 0).unwrap() + TimeDelta::hours(hours)
    }

    fn backup_count(path: &Path) -> usize {
        list_backups(&backups_dir(path)).unwrap().len()
    }

    fn files_beside(path: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn atomic_writes_replace_the_file_and_leave_nothing_behind() {
        let path = temporary_history();
        write_atomic(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(files_beside(&path), ["times.json"]);
        remove_folder_of(&path);
    }

    #[test]
    fn a_failed_replacement_keeps_the_previous_history() {
        let path = temporary_history();
        fs::write(&path, b"old").unwrap();
        let failed = write_atomic_with(&path, b"new", |_, _| Err(ErrorKind::Other.into()));
        assert!(failed.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old");
        assert_eq!(files_beside(&path), ["times.json"]);
        remove_folder_of(&path);
    }

    #[test]
    fn a_write_into_a_missing_folder_is_reported() {
        let path = temporary_history();
        let missing = path.with_file_name("missing").join("times.json");
        assert!(save_at(&missing, &json("a"), 12).is_err());
        remove_folder_of(&path);
    }

    #[test]
    fn only_old_temporaries_are_cleaned_up() {
        let path = temporary_history();
        let old = path.with_file_name("times.json.1-0.tmp");
        let recent = path.with_file_name("times.json.1-1.tmp");
        let other = path.with_file_name("other.json.1-0.tmp");
        for file in [&old, &recent, &other] {
            fs::write(file, b"partial").unwrap();
        }
        let long_ago = SystemTime::now() - STALE_TEMPORARY_AGE - Duration::from_secs(60);
        for file in [&old, &other] {
            File::options()
                .write(true)
                .open(file)
                .unwrap()
                .set_modified(long_ago)
                .unwrap();
        }
        remove_stale_temporaries(&path);
        assert!(!old.exists());
        assert!(recent.exists());
        assert!(other.exists());
        remove_folder_of(&path);
    }

    #[test]
    fn saving_backs_up_what_the_file_held() {
        let path = temporary_history();
        assert!(save_at(&path, &json("first"), 12).unwrap().is_none());
        assert_eq!(backup_count(&path), 0);

        assert!(save_at(&path, &json("second"), 12).unwrap().is_none());

        let backups = backups_at(&path).unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(&backups[0].path).unwrap(), json("first"));
        assert_eq!(fs::read(&path).unwrap(), json("second"));
        remove_folder_of(&path);
    }

    #[test]
    fn an_unchanged_history_is_not_written_or_backed_up() {
        let path = temporary_history();
        save_at(&path, &json("a"), 12).unwrap();
        save_at(&path, &json("a"), 12).unwrap();
        assert_eq!(backup_count(&path), 0);
        remove_folder_of(&path);
    }

    #[test]
    fn backups_keep_session_names_and_solve_metadata() {
        let path = temporary_history();
        let original = r#"[{"session_name":"Practice","times":[{"timestamp_in_millis":12345,
            "event":"Cube3x3","scramble":"R U","solved_at_unix_ms":1760000000000,
            "modifier":"PlusTwo","comment":"nice","changes":[{"changed_at_unix_ms":1760000000001,
            "before":{"time_ms":12345,"event":"Cube3x3","scramble":"R U","solved_at_unix_ms":1,"modifier":"None",
            "comment":""},"after":{"time_ms":12345,"event":"Cube3x3","scramble":"R U","solved_at_unix_ms":1,
            "modifier":"PlusTwo","comment":"nice"}}]}]}]"#;
        fs::write(&path, original).unwrap();
        open_at(&path, 12).unwrap();
        save_at(&path, &json("new"), 12).unwrap();

        restore_at(&path, "1", 12).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        let history = load_history_at(&path).unwrap().unwrap();
        assert_eq!(history[0].session_name(), "Practice");
        assert_eq!(history[0].len(), 1);
        remove_folder_of(&path);
    }

    #[test]
    fn opening_backs_up_the_file_as_it_was() {
        let path = temporary_history();
        fs::write(&path, json("a")).unwrap();
        let opened = open_at(&path, 12).unwrap();
        assert_eq!(opened.history.unwrap().len(), 1);
        assert!(opened.backup_error.is_none());
        assert_eq!(backup_count(&path), 1);
        remove_folder_of(&path);
    }

    #[test]
    fn opening_a_damaged_history_fails_and_leaves_it_and_the_backups_alone() {
        let path = temporary_history();
        fs::write(&path, b"not json").unwrap();
        assert!(open_at(&path, 12).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not json");
        assert_eq!(backup_count(&path), 0);
        remove_folder_of(&path);
    }

    #[test]
    fn backups_are_spaced_out_and_limited() {
        let path = temporary_history();
        let dir = backups_dir(&path);
        back_up_if_due(&dir, b"0", at(0), 3).unwrap();
        back_up_if_due(&dir, b"too soon", at(1), 3).unwrap();
        assert_eq!(list_backups(&dir).unwrap().len(), 1);

        for step in 1..=4 {
            back_up_if_due(&dir, b"later", at(step * 6), 3).unwrap();
        }

        let kept: Vec<_> = list_backups(&dir)
            .unwrap()
            .into_iter()
            .map(|(time, _)| time)
            .collect();
        assert_eq!(kept, [at(12), at(18), at(24)]);
        remove_folder_of(&path);
    }

    #[test]
    fn a_backup_dated_in_the_future_does_not_stop_new_ones_or_outlive_them() {
        let path = temporary_history();
        let dir = backups_dir(&path);
        write_backup(&dir, b"future", at(100), 12).unwrap();
        back_up_if_due(&dir, b"now", at(0), 1).unwrap();
        let kept = list_backups(&dir).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(fs::read(&kept[0].1).unwrap(), b"now");
        remove_folder_of(&path);
    }

    #[test]
    fn backups_made_in_the_same_second_keep_each_other() {
        let path = temporary_history();
        let dir = backups_dir(&path);
        let first = write_backup(&dir, b"first", at(0), 12).unwrap();
        let second = write_backup(&dir, b"second", at(0), 12).unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read(&first).unwrap(), b"first");
        remove_folder_of(&path);
    }

    #[test]
    fn other_files_in_the_backups_folder_are_left_alone() {
        let path = temporary_history();
        let dir = backups_dir(&path);
        fs::create_dir(&dir).unwrap();
        let manual = dir.join("times-manual-20260101T000000Z.json");
        fs::write(&manual, b"mine").unwrap();
        for step in 0..3 {
            back_up_if_due(&dir, b"auto", at(step * 6), 1).unwrap();
        }
        assert!(manual.exists());
        assert_eq!(list_backups(&dir).unwrap().len(), 1);
        remove_folder_of(&path);
    }

    #[test]
    fn a_damaged_history_is_not_overwritten_or_backed_up_and_solves_are_kept_beside_it() {
        let path = temporary_history();
        fs::write(&path, b"not json").unwrap();

        let error = save_at(&path, &json("new"), 12).unwrap_err();

        assert_eq!(fs::read(&path).unwrap(), b"not json");
        assert_eq!(backup_count(&path), 0);
        let unsaved = path.with_file_name("times.unsaved.json");
        assert_eq!(fs::read(&unsaved).unwrap(), json("new"));
        assert!(format!("{error:#}").contains("times.unsaved.json"));
        remove_folder_of(&path);
    }

    #[test]
    fn a_failed_backup_is_reported_without_stopping_the_save() {
        let path = temporary_history();
        fs::write(&path, json("old")).unwrap();
        fs::write(backups_dir(&path), b"a file in the way").unwrap();

        let backup_error = save_at(&path, &json("new"), 12).unwrap();

        assert!(backup_error.is_some());
        assert_eq!(fs::read(&path).unwrap(), json("new"));
        remove_folder_of(&path);
    }

    #[test]
    fn replacing_the_history_always_backs_up_a_valid_one() {
        let path = temporary_history();
        fs::write(&path, json("old")).unwrap();
        write_backup(&backups_dir(&path), &json("recent"), Utc::now(), 12).unwrap();

        let kept = replace_history_at(&path, &json("imported"), 12)
            .unwrap()
            .unwrap();

        assert_eq!(fs::read(kept).unwrap(), json("old"));
        assert_eq!(fs::read(&path).unwrap(), json("imported"));
        assert_eq!(backup_count(&path), 2);
        remove_folder_of(&path);
    }

    #[test]
    fn replacing_a_damaged_history_keeps_it_out_of_the_backups() {
        let path = temporary_history();
        fs::write(&path, b"not json").unwrap();

        let kept = replace_history_at(&path, &json("imported"), 12)
            .unwrap()
            .unwrap();

        assert_eq!(fs::read(&kept).unwrap(), b"not json");
        assert!(kept.to_string_lossy().contains(".invalid-"));
        assert_eq!(backup_count(&path), 0);
        remove_folder_of(&path);
    }

    #[test]
    fn a_history_that_cannot_be_kept_is_not_replaced() {
        let path = temporary_history();
        fs::write(&path, json("old")).unwrap();
        fs::write(backups_dir(&path), b"a file in the way").unwrap();

        assert!(replace_history_at(&path, &json("imported"), 12).is_err());

        assert_eq!(fs::read(&path).unwrap(), json("old"));
        remove_folder_of(&path);
    }

    #[test]
    fn restoring_round_trips_and_can_be_undone() {
        let path = temporary_history();
        let dir = backups_dir(&path);
        write_backup(&dir, &json("older"), at(0), 12).unwrap();
        write_backup(&dir, &json("newer"), at(6), 12).unwrap();
        fs::write(&path, json("current")).unwrap();

        let restored = restore_at(&path, "2", 12).unwrap();

        assert_eq!(fs::read(&path).unwrap(), json("older"));
        assert_eq!(fs::read(&restored.backup).unwrap(), json("older"));
        let kept = restored.kept.unwrap();
        assert_eq!(fs::read(&kept).unwrap(), json("current"));

        let name = kept.file_name().unwrap().to_str().unwrap();
        restore_at(&path, name, 12).unwrap();
        assert_eq!(fs::read(&path).unwrap(), json("current"));
        remove_folder_of(&path);
    }

    #[test]
    fn restoring_the_oldest_backup_at_the_limit_still_works() {
        let path = temporary_history();
        let dir = backups_dir(&path);
        write_backup(&dir, &json("oldest"), at(0), 2).unwrap();
        write_backup(&dir, &json("newest"), at(6), 2).unwrap();
        fs::write(&path, json("current")).unwrap();

        restore_at(&path, "2", 2).unwrap();

        assert_eq!(fs::read(&path).unwrap(), json("oldest"));
        assert_eq!(backup_count(&path), 2);
        remove_folder_of(&path);
    }

    #[test]
    fn a_corrupt_backup_is_listed_as_such_and_never_restored() {
        let path = temporary_history();
        write_backup(&backups_dir(&path), b"not json", at(0), 12).unwrap();
        fs::write(&path, json("current")).unwrap();

        assert!(backups_at(&path).unwrap()[0].contents.is_err());
        let error = restore_at(&path, "1", 12).err().unwrap();

        assert!(format!("{error:#}").contains("cannot be restored"));
        assert_eq!(fs::read(&path).unwrap(), json("current"));
        assert_eq!(backup_count(&path), 1);
        remove_folder_of(&path);
    }

    #[test]
    fn restoring_recovers_a_damaged_history() {
        let path = temporary_history();
        write_backup(&backups_dir(&path), &json("good"), at(0), 12).unwrap();
        fs::write(&path, b"not json").unwrap();

        let restored = restore_at(&path, "1", 12).unwrap();

        assert_eq!(fs::read(&path).unwrap(), json("good"));
        assert_eq!(fs::read(restored.kept.unwrap()).unwrap(), b"not json");
        remove_folder_of(&path);
    }

    #[test]
    fn unknown_backups_are_rejected() {
        let path = temporary_history();
        write_backup(&backups_dir(&path), &json("a"), at(0), 12).unwrap();
        for selector in ["0", "2", "times-19990101T000000Z.json", "../times.json"] {
            assert!(restore_at(&path, selector, 12).is_err());
        }
        remove_folder_of(&path);
    }

    #[test]
    fn backups_are_listed_newest_first_with_their_size() {
        let path = temporary_history();
        let dir = backups_dir(&path);
        write_backup(&dir, b"[]", at(0), 12).unwrap();
        write_backup(&dir, &json("a"), at(6), 12).unwrap();

        let listed = backups_at(&path).unwrap();

        assert_eq!(listed[0].time, at(6));
        assert_eq!(*listed[0].contents.as_ref().unwrap(), (1, 0));
        assert_eq!(*listed[1].contents.as_ref().unwrap(), (0, 0));
        remove_folder_of(&path);
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
