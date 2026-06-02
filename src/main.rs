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

use clap::Parser;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
};
use std::time::{Duration, Instant};

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
        } => {
            if !path.exists() {
                eprintln!("File does not exist: {}", path.display());
                std::process::exit(1);
            }
            match cstimer::import(&path) {
                Ok(histories) => {
                    let mut model = Model::new();
                    model.restore_from_history(histories);
                    persistence::save(&model);
                    println!("Imported successfully from: {}", path.display());
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("Import failed: {err}");
                    std::process::exit(1);
                }
            }
        }
        Cli {
            subcommand: Some(Command::Export { path }),
            ..
        } => {
            let histories = persistence::load().unwrap_or_default();
            let mut model = Model::new();
            model.restore_from_history(histories);
            match cstimer::export(&path, &model) {
                Ok(path) => {
                    println!("Exported successfully to: {}", path.display());
                }
                Err(err) => {
                    eprintln!("Export failed: {err}");
                }
            }
            std::process::exit(1);
        }
        #[cfg(feature = "dashboard")]
        Cli {
            subcommand: Some(Command::Dashboard { port }),
            ..
        } => {
            dashboard::run_dashboard(port);
        }
        _ => {
            #[cfg(feature = "wca-scrambles")]
            let _wca_scramble_server = match scramble::start_wca_scramble_server() {
                Ok(server) => Some(server),
                Err(error) => {
                    eprintln!(
                        "Warning: Could not enable WCA scrambles ({error}). Falling back to built-in random scrambles."
                    );
                    None
                }
            };

            ratatui::run(run);
        }
    }
}

fn run(terminal: &mut DefaultTerminal) {
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )
    .ok();

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
        {
            let msg = map_key_to_msg(key.code, key.kind);

            if let Some(msg) = msg {
                if matches!(msg, Msg::Quit) {
                    execute!(stdout, PopKeyboardEnhancementFlags).ok();
                    return;
                }
                update(&mut model, msg);
            }
        }

        terminal
            .draw(|frame| view(frame.area(), frame.buffer_mut(), &mut model))
            .ok();
    }
}
