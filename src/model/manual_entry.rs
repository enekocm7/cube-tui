//! Manual mode for entering solves timed on an external timer.
//!
//! Pressing the configured `manual_entry` shortcut on the idle main screen
//! toggles manual mode. While it is active, digits, `:` and `.` build the
//! duration and Enter records a valid solve. The timer key is disabled, while
//! every other binding keeps working so solves can still be navigated and
//! edited. Pressing the shortcut again returns to the normal timer.

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::model::keybinds::Action;
use crate::model::screen::Screen;
use crate::model::solve_editor::validation::{parse_time, validate_duration};
use crate::model::{Model, TimerState};
use crate::widgets::history::{Modifier, Time};

#[derive(Default)]
pub struct ManualEntry {
    /// Duration exactly as typed, before any penalty.
    pub time_text: String,
    pub error: Option<String>,
}

impl ManualEntry {
    fn push_text(&mut self, text: &str) -> bool {
        let mut changed = false;
        for character in text.chars() {
            changed |= self.push_char(character);
        }
        changed
    }

    /// Accepts up to two leading digits, an optional `:` with up to two more
    /// digits, and an optional `.` with up to three more digits.
    fn push_char(&mut self, character: char) -> bool {
        if !self.accepts(character) {
            return false;
        }
        self.time_text.push(character);
        self.error = None;
        true
    }

    fn accepts(&self, character: char) -> bool {
        let text = &self.time_text;
        let colon = text.find(':');
        let dot = text.find('.');
        match character {
            digit if digit.is_ascii_digit() => {
                if let Some(dot) = dot {
                    text.len() - dot - 1 < 3
                } else if let Some(colon) = colon {
                    text.len() - colon - 1 < 2
                } else {
                    text.len() < 2
                }
            }
            ':' => colon.is_none() && dot.is_none() && !text.is_empty(),
            '.' => {
                dot.is_none()
                    && !text.is_empty()
                    && colon.is_none_or(|colon| text.len() > colon + 1)
            }
            _ => false,
        }
    }

    fn backspace(&mut self) -> bool {
        let changed = self.time_text.pop().is_some();
        if changed {
            self.error = None;
        }
        changed
    }

    fn validate(&self) -> Result<u64, String> {
        if self.time_text.is_empty() {
            return Err("Enter a time like 8.98 or 1:32.49.".to_owned());
        }
        let time_ms = parse_time(&self.time_text)
            .map_err(|_| "Invalid time. Use e.g. 8.98 or 1:32.49.".to_owned())?;
        validate_duration(time_ms, Modifier::None)?;
        Ok(time_ms)
    }
}

impl Model {
    /// Returns whether manual time entry is active.
    pub const fn show_manual_entry(&self) -> bool {
        self.manual_entry.is_some()
    }

    /// Opens manual mode on the idle main screen.
    ///
    /// The active event and scramble stay untouched until Enter records the
    /// solve, at which point they are consumed like a timed solve's.
    pub fn open_manual_entry(&mut self) {
        if self.manual_entry.is_some()
            || self.timer_state() != TimerState::Idle
            || !matches!(self.screen, Screen::Main)
        {
            return;
        }
        self.manual_entry = Some(ManualEntry::default());
    }

    /// Leaves manual mode and returns to the normal timer.
    pub fn close_manual_entry(&mut self) {
        self.manual_entry = None;
    }

    /// Toggles manual mode with the configured shortcut.
    pub fn toggle_manual_entry(&mut self) {
        if self.manual_entry.is_some() {
            self.close_manual_entry();
        } else {
            self.open_manual_entry();
        }
    }

    /// Captures manual-mode input.
    ///
    /// Digits, `:`, `.`, Backspace, and Enter edit and record the duration.
    /// The manual-entry shortcut and quit key still work, the timer key
    /// (Space by default) is disabled on the main screen, and every other
    /// binding falls through to normal application handling so history
    /// navigation and solve editing remain available.
    pub fn handle_manual_entry_event(&mut self, event: &Event) -> Option<bool> {
        if !matches!(event, Event::Key(_) | Event::Paste(_)) {
            return None;
        }
        self.manual_entry.as_ref()?;
        let typing = self.manual_typing_active();
        if let Event::Paste(text) = event {
            if !typing {
                return None;
            }
            let entry = self.manual_entry.as_mut()?;
            return Some(entry.push_text(text));
        }
        let Event::Key(key) = event else {
            return Some(false);
        };
        let action = self.settings().keybinds().action_for(*key);
        if action == Some(Action::ManualEntry) && key.kind == KeyEventKind::Press {
            self.close_manual_entry();
            return Some(true);
        }
        // Let the normal quit path close the application.
        if action == Some(Action::Quit) {
            return None;
        }
        if !typing {
            return None;
        }
        // Space must not start the timer while manual input is active.
        if key.code == KeyCode::Char(' ') || action == Some(Action::Timer) {
            return Some(false);
        }
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if key.code == KeyCode::Enter && key.kind == KeyEventKind::Press {
            let empty = self
                .manual_entry
                .as_ref()
                .is_some_and(|entry| entry.time_text.is_empty());
            if empty {
                // With nothing typed, Enter keeps its normal details binding.
                return None;
            }
            self.record_manual_solve();
            return Some(true);
        }
        let entry = self.manual_entry.as_mut()?;
        match key.code {
            KeyCode::Backspace => Some(entry.backspace()),
            KeyCode::Char(character)
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL
                        | KeyModifiers::ALT
                        | KeyModifiers::SUPER
                        | KeyModifiers::HYPER
                        | KeyModifiers::META,
                ) =>
            {
                let changed = entry.push_text(&character.to_string());
                changed.then_some(true)
            }
            _ => None,
        }
    }

    /// Returns whether manual input currently owns the main-screen keys.
    fn manual_typing_active(&self) -> bool {
        matches!(self.screen, Screen::Main)
            && !self.show_help()
            && self.confirmation().is_none()
            && self.theme_selector.is_none()
            && {
                #[cfg(feature = "bluetooth")]
                {
                    !self.show_bluetooth()
                }
                #[cfg(not(feature = "bluetooth"))]
                {
                    true
                }
            }
    }

    /// Records the typed duration when it is valid and clears the input.
    fn record_manual_solve(&mut self) {
        let Some(entry) = self.manual_entry.as_ref() else {
            return;
        };
        let time_ms = match entry.validate() {
            Ok(time_ms) => time_ms,
            Err(error) => {
                if let Some(entry) = self.manual_entry.as_mut() {
                    entry.error = Some(error);
                }
                return;
            }
        };
        if self.timer_state() != TimerState::Idle {
            if let Some(entry) = self.manual_entry.as_mut() {
                entry.error =
                    Some("Finish or reset the current attempt before adding a solve.".to_owned());
            }
            return;
        }
        let session = self.current_session_mut();
        let event = session.event;
        let scramble = session
            .scramble
            .take()
            .expect("active session should have a scramble");
        session.last_time_ms = time_ms;
        session.last_modifier = Modifier::None;
        session.history.add(Time::new_with_modifier(
            time_ms,
            event,
            scramble,
            Modifier::None,
        ));
        self.next_scramble();
        self.notify_personal_bests();
        self.save_history();
        // Stay in manual mode for the next solve, exactly as if the timer had
        // been reset after recording.
        if let Some(entry) = self.manual_entry.as_mut() {
            entry.time_text.clear();
            entry.error = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::{Buffer, Cell};
    use ratatui::crossterm::event::KeyEvent;
    use ratatui::layout::Rect;

    use super::*;
    use crate::scramble::{Scramble, WcaEvent};
    use crate::widgets::history::History;

    fn key(model: &mut Model, code: KeyCode) {
        let _ = crate::handle_terminal_event(
            model,
            &Event::Key(KeyEvent::new(code, KeyModifiers::NONE)),
        );
    }

    fn ctrl_key(model: &mut Model, character: char) {
        let _ = crate::handle_terminal_event(
            model,
            &Event::Key(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::CONTROL,
            )),
        );
    }

    fn type_text(model: &mut Model, text: &str) {
        for character in text.chars() {
            key(model, KeyCode::Char(character));
        }
    }

    fn manual_model() -> Model {
        let mut model = Model::new();
        model.history_load_failed = true; // Never write the user's history from tests.
        model.current_session_mut().scramble = Some(Scramble::new("R U F"));
        key(&mut model, KeyCode::Char('m'));
        assert!(model.show_manual_entry());
        model
    }

    #[test]
    fn manual_mode_toggles_and_swallows_the_timer_key() {
        let mut model = manual_model();
        key(&mut model, KeyCode::Char(' '));
        assert_eq!(model.timer_state(), TimerState::Idle);
        assert!(model.show_manual_entry());

        key(&mut model, KeyCode::Char('m'));
        assert!(!model.show_manual_entry());
        model.set_settings(toml::from_str("[timer]\ninspection = false").unwrap());
        key(&mut model, KeyCode::Char(' '));
        assert_eq!(model.timer_state(), TimerState::Pulsed);
    }

    #[test]
    fn enter_records_valid_solves_and_keeps_manual_mode_active() {
        let mut model = manual_model();
        type_text(&mut model, "12:34");
        key(&mut model, KeyCode::Enter);
        let time = model.history().last().unwrap();
        assert_eq!(time.raw_ms(), 754_000);
        assert_eq!(time.modifier(), Modifier::None);
        assert_eq!(time.scramble(), "R U F");
        assert_eq!(time.event(), crate::scramble::WcaEvent::Cube3x3);
        assert!(time.solved_at_unix_ms() > 0);
        assert_eq!(model.elapsed_ms(), 754_000);
        assert_eq!(model.displayed_modifier(), Modifier::None);
        assert_ne!(model.scramble(), "R U F");
        assert!(model.show_manual_entry());
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "");

        type_text(&mut model, "12.345");
        key(&mut model, KeyCode::Enter);
        assert_eq!(model.history().len(), 2);
        assert_eq!(model.history().last().unwrap().raw_ms(), 12_345);

        let json = serde_json::to_string(model.history()).unwrap();
        let restored: History = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.last().unwrap().raw_ms(), 12_345);
    }

    #[test]
    fn typed_input_allows_short_values_and_caps_each_part() {
        let mut model = manual_model();
        type_text(&mut model, "123");
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "12");
        type_text(&mut model, ":");
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "12:");
        type_text(&mut model, "345");
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "12:34");
        type_text(&mut model, ".");
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "12:34.");
        type_text(&mut model, "5678");
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "12:34.567");

        for _ in 0..9 {
            key(&mut model, KeyCode::Backspace);
        }
        type_text(&mut model, "0.76");
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "0.76");
        type_text(&mut model, "89");
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "0.768");
        type_text(&mut model, ":");
        assert_eq!(model.manual_entry.as_ref().unwrap().time_text, "0.768");
    }

    #[test]
    fn short_seconds_and_minutes_formats_record_correctly() {
        let mut model = manual_model();
        for (text, expected) in [
            ("0.5", 500),
            ("0.76", 760),
            ("8.98", 8_980),
            ("1:32.49", 92_490),
            ("8", 8_000),
        ] {
            type_text(&mut model, text);
            key(&mut model, KeyCode::Enter);
            assert_eq!(model.history().last().unwrap().raw_ms(), expected, "{text}");
        }
        assert_eq!(model.history().len(), 5);
    }

    #[test]
    fn invalid_input_is_kept_editable_and_never_records() {
        let mut model = manual_model();
        let scramble = model.scramble().to_owned();
        type_text(&mut model, "12:99");
        key(&mut model, KeyCode::Enter);
        assert!(model.history().is_empty());
        assert_eq!(model.scramble(), scramble);
        assert!(model.show_manual_entry());
        assert!(
            model
                .manual_entry
                .as_ref()
                .unwrap()
                .error
                .as_ref()
                .unwrap()
                .contains("Invalid time")
        );

        key(&mut model, KeyCode::Backspace);
        key(&mut model, KeyCode::Backspace);
        type_text(&mut model, "34");
        key(&mut model, KeyCode::Enter);
        assert_eq!(model.history().last().unwrap().raw_ms(), 754_000);
    }

    #[test]
    fn solve_editing_still_works_while_manual_mode_is_active() {
        let mut model = manual_model();
        model.history_mut().add_ms(1_000, WcaEvent::Cube3x3, "R U");

        key(&mut model, KeyCode::Enter); // Empty manual field keeps details binding.
        assert!(model.show_details());
        key(&mut model, KeyCode::Char('h'));
        assert!(model.solve_editor.is_some());

        key(&mut model, KeyCode::Enter); // Edit the time field.
        assert!(model.text_input.is_some());
        key(&mut model, KeyCode::Home);
        for _ in 0..9 {
            key(&mut model, KeyCode::Delete);
        }
        let _ = crate::handle_terminal_event(&mut model, &Event::Paste("2.5".into()));
        key(&mut model, KeyCode::Enter);
        assert_eq!(model.solve_editor.as_ref().unwrap().draft.time_ms, 2_500);

        let _ = crate::handle_terminal_event(
            &mut model,
            &Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)),
        );
        assert!(model.solve_editor.is_none());
        assert_eq!(model.history().last().unwrap().raw_ms(), 2_500);
        assert!(model.show_manual_entry());
    }

    #[test]
    fn other_bindings_keep_working_while_space_stays_disabled() {
        let mut model = manual_model();
        model.history_mut().add_ms(1_000, WcaEvent::Cube3x3, "R U");
        model.history_mut().add_ms(2_000, WcaEvent::Cube3x3, "R U");

        key(&mut model, KeyCode::Up);
        assert_eq!(model.history().selected, Some(0));
        key(&mut model, KeyCode::Down);
        assert_eq!(model.history().selected, Some(1));

        // With nothing typed, Enter keeps its details binding.
        key(&mut model, KeyCode::Enter);
        assert!(model.show_details());
        key(&mut model, KeyCode::Esc);
        assert!(!model.show_details());
        assert!(model.show_manual_entry());

        let scramble = model.scramble().to_owned();
        key(&mut model, KeyCode::Char('n'));
        assert_ne!(model.scramble(), scramble);

        key(&mut model, KeyCode::Char(' '));
        assert_eq!(model.timer_state(), TimerState::Idle);
        assert!(model.show_manual_entry());

        let result = crate::handle_terminal_event(
            &mut model,
            &Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
        );
        assert!(result.is_break());
    }

    #[test]
    fn shortcut_is_configurable_and_requires_the_idle_main_screen() {
        let mut model = Model::new();
        model.set_settings(toml::from_str("[keybinds]\nmanual_entry = 'Ctrl+m'").unwrap());
        key(&mut model, KeyCode::Char('m'));
        assert!(!model.show_manual_entry());
        ctrl_key(&mut model, 'm');
        assert!(model.show_manual_entry());
        ctrl_key(&mut model, 'm');
        assert!(!model.show_manual_entry());

        model.history_mut().add_ms(1_000, WcaEvent::Cube3x3, "R U");
        model.open_details();
        ctrl_key(&mut model, 'm');
        assert!(!model.show_manual_entry());
        model.close_current_screen();

        model.set_timer_state(TimerState::Pulsed);
        ctrl_key(&mut model, 'm');
        assert!(!model.show_manual_entry());
        model.reset_timer();
        ctrl_key(&mut model, 'm');
        assert!(model.show_manual_entry());
    }

    #[test]
    fn timer_shows_the_placeholder_until_typing_starts() {
        let mut model = manual_model();
        model.set_settings(toml::from_str("[display]\nbig_timer = false").unwrap());
        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        crate::view::view(area, &mut buffer, &mut model);
        let text = buffer.content.iter().map(Cell::symbol).collect::<String>();
        assert!(text.contains("00:00.000"));
        assert!(text.contains("Timer | Manual mode"));
        assert!(
            text.find("Timer | Manual mode").unwrap() < text.find("Inspection: On").unwrap(),
            "manual mode should follow the Timer label"
        );

        type_text(&mut model, "07.250");
        let mut buffer = Buffer::empty(area);
        crate::view::view(area, &mut buffer, &mut model);
        let text = buffer.content.iter().map(Cell::symbol).collect::<String>();
        assert!(text.contains("07.250"));
        assert!(!text.contains("00:00.000"));
        assert!(text.contains("Manual mode"));
    }
}
