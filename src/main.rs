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
    event::{
        DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};

use crate::cli::{Cli, Command};
use crate::handler::update;
use crate::model::{Model, TimerState};
use crate::msg::{Msg, map_key_to_msg};
use crate::utils::print_as_link;
use crate::view::view;

/// Parses command-line options and dispatches the selected application mode.
fn main() {
    let cli = Cli::parse();

    match cli {
        Cli { config: true, .. } => match persistence::config_file() {
            Ok(path) => print_as_link(&path),
            Err(error) => {
                eprintln!("Error: {error:#}");
                std::process::exit(1);
            }
        },
        Cli { data: true, .. } => match persistence::data_dir() {
            Ok(path) => print_as_link(&path),
            Err(error) => {
                eprintln!("Error: {error:#}");
                std::process::exit(1);
            }
        },
        Cli { theme: true, .. } => match persistence::themes_dir() {
            Ok(path) => print_as_link(&path),
            Err(error) => {
                eprintln!("Error: {error:#}");
                std::process::exit(1);
            }
        },
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
            if let Err(error) = ratatui::run(run) {
                eprintln!("Terminal UI failed: {error}");
                std::process::exit(1);
            }
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
            if let Err(error) = persistence::save(&model) {
                eprintln!("Import could not be saved: {error:#}");
                std::process::exit(1);
            }
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
    let histories = match persistence::load() {
        Ok(histories) => histories.unwrap_or_default(),
        Err(error) => {
            eprintln!("Export failed: {error:#}");
            std::process::exit(1);
        }
    };
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

struct TerminalInputGuard {
    stdout: Stdout,
    keyboard_active: bool,
    paste_active: bool,
}

impl TerminalInputGuard {
    /// Enables press/release events and bracketed paste until the guard drops.
    fn enable() -> Self {
        let mut stdout = std::io::stdout();
        let keyboard_active = execute!(
            stdout,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        )
        .is_ok();
        let paste_active = execute!(stdout, EnableBracketedPaste).is_ok();
        Self {
            stdout,
            keyboard_active,
            paste_active,
        }
    }
}

impl Drop for TerminalInputGuard {
    /// Restores the terminal keyboard protocol when the guard leaves scope.
    fn drop(&mut self) {
        if self.paste_active {
            let _ = execute!(self.stdout, DisableBracketedPaste);
        }
        if self.keyboard_active {
            let _ = execute!(self.stdout, PopKeyboardEnhancementFlags);
        }
    }
}

const TICK_RATE: Duration = Duration::from_millis(30);

/// Returns whether a timer state needs periodic frames at `now`.
///
/// Inspection remains animated while the user holds beyond the limit because
/// the eventual `+2` or DNF is determined from the exact timer start instant.
fn timer_is_animating(state: TimerState) -> bool {
    match state {
        TimerState::Running { .. } | TimerState::Inspection { .. } => true,
        TimerState::Idle | TimerState::Pulsed => false,
    }
}

/// Applies a terminal event and reports whether it requires a redraw.
///
/// `ControlFlow::Break` requests application exit. A continued `true` value
/// means the event changed visible state, including resize and focus events.
fn handle_terminal_event(model: &mut Model, event: &Event) -> ControlFlow<(), bool> {
    if let Some(changed) = model.handle_text_input_event(event) {
        return ControlFlow::Continue(changed);
    }
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
/// Idle screens block for input until the next toast expiry, if any. Active
/// timers and Bluetooth receivers wake at [`TICK_RATE`].
fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let _input_guard = TerminalInputGuard::enable();
    let mut stdout = std::io::stdout();

    let mut model = Model::new();
    model.load_persisted_state();
    let mut last_tick = Instant::now();
    let mut redraw = true;
    let mut timer_was_animating = false;

    loop {
        if model.toasts.remove_expired(Instant::now()) {
            redraw = true;
        }
        // Draw the final zero even if an ignored input event returns just as
        // inspection expires, before its next scheduled tick.
        let animate_timer = timer_is_animating(model.timer_state());
        let timer_animation_just_ended = timer_was_animating && !animate_timer;
        if timer_animation_just_ended {
            redraw = true;
        }
        timer_was_animating = animate_timer;
        if redraw {
            // Include autoresize's clear and the full frame in one visible update.
            stdout.sync_update(|_| {
                terminal
                    .draw(|frame| view(frame.area(), frame.buffer_mut(), &mut model))
                    .map(|_| ())
            })??;
        }

        #[cfg(feature = "bluetooth")]
        let needs_tick = animate_timer || model.bluetooth_needs_poll();
        #[cfg(not(feature = "bluetooth"))]
        let needs_tick = animate_timer;

        // Blocking reads have no periodic wakeups when nothing is changing.
        // Active timers and Bluetooth retain their existing 30 ms tick cadence.
        let timeout = needs_tick.then(|| TICK_RATE.saturating_sub(last_tick.elapsed()));
        let timeout = next_wakeup(timeout, model.toasts.next_expiration(Instant::now()));
        let event = read_terminal_event(timeout)?;
        if !needs_tick {
            last_tick = Instant::now();
        }

        redraw = if let Some(event) = event {
            let ControlFlow::Continue(changed) = handle_terminal_event(&mut model, &event) else {
                return Ok(());
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

/// Wakes for the earliest animation tick or toast expiry, or blocks if idle.
fn next_wakeup(tick: Option<Duration>, toast: Option<Duration>) -> Option<Duration> {
    match (tick, toast) {
        (Some(tick), Some(toast)) => Some(tick.min(toast)),
        (timeout, None) | (None, timeout) => timeout,
    }
}

#[cfg(test)]
mod event_loop_tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::widgets::history::Modifier;
    use crate::widgets::text_input::TextInputResult;

    #[test]
    fn rename_shortcut_edits_only_the_current_session_and_cancellation_preserves_it() {
        let mut model = Model::new();
        // Exercise submission without writing to the user's actual history file.
        model.history_load_failed = true;
        model.history_mut().set_session_name("First".into());
        model.add_session();
        let rename = Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert_eq!(
            handle_terminal_event(&mut model, &rename),
            ControlFlow::Continue(true)
        );
        assert!(model.text_input.is_some());
        let _ = handle_terminal_event(&mut model, &Event::Paste("  Practice 界  ".into()));
        let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let _ = handle_terminal_event(&mut model, &enter);
        assert!(model.text_input.is_none());
        assert_eq!(model.history().session_name(), "Practice 界");

        let _ = handle_terminal_event(&mut model, &rename);
        let _ = handle_terminal_event(&mut model, &enter);
        assert_eq!(
            model.history().session_name(),
            "Practice 界",
            "prompt should prefill the existing name"
        );
        let _ = handle_terminal_event(&mut model, &rename);
        let _ = handle_terminal_event(&mut model, &Event::Paste("discard".into()));
        let escape = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let _ = handle_terminal_event(&mut model, &escape);
        assert_eq!(model.history().session_name(), "Practice 界");
        model.prev_session();
        assert_eq!(model.history().session_name(), "First");
    }

    #[test]
    fn rename_is_blocked_outside_idle_main_and_rejects_blank_names() {
        let mut model = Model::new();
        model.history_load_failed = true;
        let rename = Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        model.toggle_help();
        let _ = handle_terminal_event(&mut model, &rename);
        assert!(model.text_input.is_none());
        model.toggle_help();
        model.set_timer_state(TimerState::Pulsed);
        let _ = handle_terminal_event(&mut model, &rename);
        assert!(model.text_input.is_none());
        model.reset_timer();
        let _ = handle_terminal_event(&mut model, &rename);
        let _ = handle_terminal_event(&mut model, &Event::Paste("   ".into()));
        let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let _ = handle_terminal_event(&mut model, &enter);
        assert_eq!(model.history().session_name(), "");
        assert!(
            model
                .toasts
                .iter_mut()
                .any(|toast| toast.message.contains("cannot be empty"))
        );
    }

    #[test]
    fn prompt_captures_shortcuts_and_returns_text_to_its_caller() {
        let mut model = Model::new();
        model.toggle_help();
        assert!(model.request_text_input("Input", "", |model, result| {
            assert!(model.text_input.is_none());
            assert!(model.show_help());
            assert_eq!(result, TextInputResult::Submitted("q ?pasted".into()));
            model.set_last_time_ms(123);
        }));
        for character in ['q', ' ', '?'] {
            let event = Event::Key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
            assert_eq!(
                handle_terminal_event(&mut model, &event),
                ControlFlow::Continue(true)
            );
        }
        assert_eq!(model.timer_state(), TimerState::Idle);
        assert!(model.show_help());
        assert_eq!(
            handle_terminal_event(&mut model, &Event::Paste("pasted".into())),
            ControlFlow::Continue(true)
        );
        let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(
            handle_terminal_event(&mut model, &enter),
            ControlFlow::Continue(true)
        );
        assert_eq!(model.elapsed_ms(), 123);
        assert!(model.text_input.is_none());
        let quit = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(
            handle_terminal_event(&mut model, &quit),
            ControlFlow::Break(())
        );
    }

    #[test]
    fn cancellation_preserves_underlying_screen_and_can_open_the_next_prompt() {
        let mut model = Model::new();
        model.toggle_help();
        assert!(
            model.request_text_input("First", "discard", |model, result| {
                assert_eq!(result, TextInputResult::Cancelled);
                assert!(model.request_text_input("Second", "", |model, result| {
                    assert_eq!(result, TextInputResult::Submitted(String::new()));
                    model.set_last_time_ms(456);
                }));
            })
        );
        assert!(!model.request_text_input("Rejected", "", |_, _| panic!("replaced active prompt")));
        let escape = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(
            handle_terminal_event(&mut model, &escape),
            ControlFlow::Continue(true)
        );
        assert!(model.show_help());
        assert!(model.text_input.is_some());
        let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(
            handle_terminal_event(&mut model, &enter),
            ControlFlow::Continue(true)
        );
        assert!(model.text_input.is_none());
        assert_eq!(model.elapsed_ms(), 456);
    }

    #[test]
    fn toast_expiry_wakes_idle_input_and_preserves_faster_timer_ticks() {
        let short = Duration::from_secs(3);
        assert_eq!(next_wakeup(None, None), None);
        assert_eq!(next_wakeup(None, Some(short)), Some(short));
        assert_eq!(next_wakeup(Some(TICK_RATE), None), Some(TICK_RATE));
        assert_eq!(next_wakeup(Some(TICK_RATE), Some(short)), Some(TICK_RATE));
        assert_eq!(
            next_wakeup(Some(TICK_RATE), Some(Duration::ZERO)),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn idle_and_armed_timers_do_not_need_periodic_frames() {
        assert!(!timer_is_animating(TimerState::Idle));
        assert!(!timer_is_animating(TimerState::Pulsed));
        assert!(timer_is_animating(TimerState::Running {
            time: Instant::now(),
            inspection_modifier: Modifier::None,
        }));
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
