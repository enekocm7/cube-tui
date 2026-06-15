#[cfg(feature = "bluetooth")]
pub mod bluetooth;
mod cli;
pub mod cstimer;
#[cfg(feature = "dashboard")]
mod dashboard;
mod handler;
mod model;
mod msg;
mod persistence;
mod scramble;
mod utils;
mod view;
mod widgets;

use std::io::Stdout;
use std::time::{Duration, Instant};

use clap::Parser;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};
use ratatui::crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
};

use crate::cli::{Cli, Command};
use crate::handler::update;
use crate::model::Model;
use crate::msg::{Msg, map_key_to_msg};
use crate::utils::print_as_link;
use crate::view::view;

fn main() {
    let cli = Cli::parse();

    match cli {
        Cli { config: true, .. } => {
            if let Some(path) = persistence::config_file() {
                print_as_link(&path);
            } else {
                eprintln!("Error: Could not determine config file");
                std::process::exit(1);
            }
        }
        Cli { data: true, .. } => {
            if let Some(dir) = persistence::data_dir() {
                print_as_link(&dir);
            } else {
                eprintln!("Error: Could not determine data directory");
                std::process::exit(1);
            }
        }
        Cli {
            subcommand: Some(Command::Import { path }),
            ..
        } => run_import(&path),
        Cli {
            subcommand: Some(Command::Export { path }),
            ..
        } => run_export(&path),
        #[cfg(feature = "dashboard")]
        Cli {
            subcommand: Some(Command::Dashboard { port }),
            ..
        } => {
            dashboard::run_dashboard(port);
        }
        _ => {
            #[cfg(feature = "wca-scrambles")]
            let _wca_server = {
                let mut settings_model = Model::new();
                if let Some(settings) = persistence::load_config() {
                    settings_model.set_settings(settings);
                }
                let show_logs = settings_model.settings().show_logs();
                match scramble::start_wca_scramble_server(show_logs) {
                    Ok(server) => Some(server),
                    Err(error) => {
                        eprintln!(
                            "Warning: Could not enable WCA scrambles ({error}). Falling back to built-in random scrambles."
                        );
                        None
                    }
                }
            };

            ratatui::run(run);
        }
    }
}

fn run_import(path: &std::path::Path) -> ! {
    if !path.exists() {
        eprintln!("File does not exist: {}", path.display());
        std::process::exit(1);
    }
    match cstimer::import(path) {
        Ok(histories) => {
            let mut model = Model::new();
            model.restore_from_history(histories);
            persistence::save(&model);
            println!("Imported successfully from: {}", path.display());
        }
        Err(err) => {
            eprintln!("Import failed: {err}");
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

fn run_export(path: &std::path::Path) {
    let histories = persistence::load().unwrap_or_default();
    let mut model = Model::new();
    model.restore_from_history(histories);
    match cstimer::export(path, &model) {
        Ok(path) => {
            println!("Exported successfully to: {}", path.display());
        }
        Err(err) => {
            eprintln!("Export failed: {err}");
            std::process::exit(1);
        }
    }
}

struct KeyboardEnhancementGuard {
    stdout: Stdout,
    active: bool,
}

impl KeyboardEnhancementGuard {
    fn enable() -> Self {
        let mut stdout = std::io::stdout();
        let active = execute!(
            stdout,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        )
        .is_ok();
        Self { stdout, active }
    }
}

impl Drop for KeyboardEnhancementGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = execute!(self.stdout, PopKeyboardEnhancementFlags);
        }
    }
}

fn run(terminal: &mut DefaultTerminal) {
    let _keyboard_enhancements = KeyboardEnhancementGuard::enable();

    let mut model = Model::new();
    if let Some(data) = persistence::load() {
        model.restore_from_history(data);
    }
    if let Some(settings) = persistence::load_config() {
        model.set_settings(settings);
    }
    let tick_rate = Duration::from_millis(30);
    let mut last_tick = Instant::now();

    loop {
        if last_tick.elapsed() >= tick_rate {
            update(&mut model, Msg::Tick);
            last_tick = Instant::now();
        }

        if event::poll(Duration::from_millis(10)).unwrap_or(false)
            && let Ok(Event::Key(key)) = event::read()
            && let Some(msg) = map_key_to_msg(key.code, key.kind)
        {
            if matches!(msg, Msg::Quit) {
                return;
            }
            update(&mut model, msg);
        }

        terminal
            .draw(|frame| view(frame.area(), frame.buffer_mut(), &mut model))
            .ok();
    }
}
