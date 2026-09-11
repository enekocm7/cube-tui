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
use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use clap::Parser;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};
use ratatui::crossterm::{
    SynchronizedUpdate,
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
};

use crate::cli::{Cli, Command};
use crate::handler::update;
use crate::model::{InspectionState, Model, TimerState};
use crate::msg::{INSPECTION_LIMIT_MS, Msg, map_key_to_msg};
use crate::utils::print_as_link;
use crate::view::view;

/// Parses command-line options and dispatches the selected application mode.
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
        Cli { theme: true, .. } => {
            if let Some(theme_dir) = persistence::themes_dir() {
                print_as_link(&theme_dir);
            } else {
                eprintln!("Error: Could not determine theme directory");
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
            ratatui::run(run);
        }
    }
}

/// Imports a csTimer file into persistent storage and exits with a status code.
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

/// Exports persisted sessions to a csTimer-compatible JSON file.
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
    /// Enables press/release keyboard events and remembers whether it succeeded.
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
    /// Restores the terminal keyboard protocol when the guard leaves scope.
    fn drop(&mut self) {
        if self.active {
            let _ = execute!(self.stdout, PopKeyboardEnhancementFlags);
        }
    }
}

const TICK_RATE: Duration = Duration::from_millis(30);

/// Returns whether a timer state needs periodic frames at `now`.
///
/// A held inspection timer remains animated until its limit so the UI draws
/// the final zero before switching to event-driven idle rendering.
fn timer_is_animating(state: TimerState, now: Instant) -> bool {
    match state {
        TimerState::Running(_) | TimerState::Inspection(InspectionState::Running(_)) => true,
        TimerState::Inspection(InspectionState::Pulsed(start)) => {
            now.saturating_duration_since(start) < Duration::from_millis(INSPECTION_LIMIT_MS)
        }
        TimerState::Idle | TimerState::Pulsed => false,
    }
}

/// Applies a terminal event and reports whether it requires a redraw.
///
/// `ControlFlow::Break` requests application exit. A continued `true` value
/// means the event changed visible state, including resize and focus events.
fn handle_terminal_event(model: &mut Model, event: &Event) -> ControlFlow<(), bool> {
    match event {
        Event::Key(key) => {
            let Some(msg) = map_key_to_msg(*key, model.settings().keybinds()) else {
                return ControlFlow::Continue(false);
            };
            if matches!(msg, Msg::Quit) {
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(update(model, msg))
        }
        Event::Resize(_, _) | Event::FocusGained => ControlFlow::Continue(true),
        _ => ControlFlow::Continue(false),
    }
}

/// Reads a terminal event, optionally waiting only for the supplied duration.
///
/// With no timeout this blocks until an event arrives. With a timeout it
/// returns `Ok(None)` when no event arrives before the deadline.
fn read_terminal_event(timeout: Option<Duration>) -> std::io::Result<Option<Event>> {
    if let Some(timeout) = timeout
        && !event::poll(timeout)?
    {
        return Ok(None);
    }
    event::read().map(Some)
}

/// Runs the event-driven terminal UI until the user quits or input fails.
///
/// Idle screens block for input. Active timers and Bluetooth receivers wake at
/// [`TICK_RATE`] so only changing screens incur rendering work.
fn run(terminal: &mut DefaultTerminal) {
    let _keyboard_enhancements = KeyboardEnhancementGuard::enable();
    let mut stdout = std::io::stdout();

    let mut model = Model::new();
    if let Some(data) = persistence::load() {
        model.restore_from_history(data);
    }
    match persistence::load_config() {
        Ok(Some(settings)) => model.set_settings(settings),
        Ok(None) => {}
        Err(error) => {
            eprintln!(
                "Warning: {error}. Using default settings; the file will not be overwritten."
            );
        }
    }
    let mut last_tick = Instant::now();
    let mut redraw = true;
    let mut timer_was_animating = false;

    loop {
        // Draw the final zero even if an ignored input event returns just as
        // inspection expires, before its next scheduled tick.
        let animate_timer = timer_is_animating(model.timer_state(), Instant::now());
        let timer_animation_just_ended = timer_was_animating && !animate_timer;
        if timer_animation_just_ended {
            redraw = true;
        }
        timer_was_animating = animate_timer;
        if redraw {
            // Include autoresize's clear and the full frame in one visible update.
            stdout
                .sync_update(|_| {
                    terminal
                        .draw(|frame| view(frame.area(), frame.buffer_mut(), &mut model))
                        .map(|_| ())
                })
                .ok();
        }

        #[cfg(feature = "bluetooth")]
        let needs_tick = animate_timer || model.bluetooth_needs_poll();
        #[cfg(not(feature = "bluetooth"))]
        let needs_tick = animate_timer;

        // Blocking reads have no periodic wakeups when nothing is changing.
        // Active timers and Bluetooth retain their existing 30 ms tick cadence.
        let timeout = needs_tick.then(|| TICK_RATE.saturating_sub(last_tick.elapsed()));
        let Ok(event) = read_terminal_event(timeout) else {
            return;
        };
        if !needs_tick {
            last_tick = Instant::now();
        }

        redraw = if let Some(event) = event {
            let ControlFlow::Continue(changed) = handle_terminal_event(&mut model, &event) else {
                return;
            };
            changed
        } else {
            false
        };

        if needs_tick && last_tick.elapsed() >= TICK_RATE {
            let model_changed = update(&mut model, Msg::Tick);
            if animate_timer || model_changed {
                redraw = true;
            }
            last_tick = Instant::now();
        }
    }
}

#[cfg(test)]
mod event_loop_tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;

    #[test]
    fn idle_and_armed_timers_do_not_need_periodic_frames() {
        let now = Instant::now();
        assert!(!timer_is_animating(TimerState::Idle, now));
        assert!(!timer_is_animating(TimerState::Pulsed, now));
        assert!(timer_is_animating(TimerState::Running(now), now));
    }

    #[test]
    fn inspection_keeps_updating_until_its_final_frame() {
        let start = Instant::now();
        let deadline = start + Duration::from_millis(INSPECTION_LIMIT_MS);
        assert!(timer_is_animating(
            TimerState::Inspection(InspectionState::Running(start)),
            deadline
        ));
        let held = TimerState::Inspection(InspectionState::Pulsed(start));
        assert!(timer_is_animating(
            held,
            deadline.checked_sub(TICK_RATE).unwrap()
        ));
        assert!(!timer_is_animating(held, deadline));
    }

    #[test]
    fn resize_requests_a_frame_even_while_idle() {
        let mut model = Model::new();
        assert_eq!(
            handle_terminal_event(&mut model, &Event::Resize(120, 40)),
            ControlFlow::Continue(true)
        );
    }

    #[test]
    fn key_input_updates_state_and_requests_an_immediate_frame() {
        let mut model = Model::new();
        let help_key = Event::Key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert_eq!(
            handle_terminal_event(&mut model, &help_key),
            ControlFlow::Continue(true)
        );
        assert!(model.show_help());
        assert_eq!(
            handle_terminal_event(&mut model, &Event::FocusLost),
            ControlFlow::Continue(false)
        );
    }
}
