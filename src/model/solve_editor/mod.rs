#[cfg(test)]
mod tests;
mod validation;

use chrono::{DateTime, SecondsFormat};
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::model::keybinds::Action;
use crate::model::{Model, TimerState};
use crate::scramble::WcaEvent;
use crate::widgets::history::{Modifier, SolveSnapshot, format_millis};
use crate::widgets::text_input::TextInputResult;
use validation::{parse_date, parse_time, validate_comment, validate_scramble};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditorPage {
    Fields,
    Events,
    History,
}

pub struct SolveEditor {
    pub session: usize,
    pub index: usize,
    original: SolveSnapshot,
    pub draft: SolveSnapshot,
    pub selected: usize,
    pub event_selection: usize,
    pub page: EditorPage,
    pub error: Option<String>,
    pub history_scroll: u16,
    pub history_max_scroll: u16,
}

pub const FIELD_NAMES: [&str; 9] = [
    "Time (before penalty)",
    "Event",
    "Scramble",
    "Penalty",
    "Solved at",
    "Comment",
    "Change history",
    "Save changes",
    "Cancel",
];

pub fn date_text(timestamp: u64) -> String {
    if timestamp == 0 {
        return String::new();
    }
    i64::try_from(timestamp)
        .ok()
        .and_then(DateTime::from_timestamp_millis)
        .map_or_else(
            || timestamp.to_string(),
            |date| date.to_rfc3339_opts(SecondsFormat::Millis, true),
        )
}

pub const fn penalty_text(modifier: Modifier) -> &'static str {
    match modifier {
        Modifier::None => "None",
        Modifier::PlusTwo => "+2",
        Modifier::DNF => "DNF",
    }
}

impl SolveEditor {
    pub fn value(&self, field: usize) -> String {
        match field {
            0 => format_millis(self.draft.time_ms),
            1 => self.draft.event.name().to_owned(),
            2 => self.draft.scramble.clone(),
            3 => penalty_text(self.draft.modifier).to_owned(),
            4 => date_text(self.draft.solved_at_unix_ms),
            5 => self.draft.comment.clone(),
            _ => String::new(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.original != self.draft
    }

    fn validate(&self) -> Result<(), (usize, String)> {
        if self.draft.time_ms >= 86_400_000 {
            return Err((0, "Time must be less than 24 hours.".to_owned()));
        }
        if self.draft.time_ms == 0 && self.draft.modifier != Modifier::DNF {
            return Err((
                0,
                "Time must be positive unless the solve is DNF.".to_owned(),
            ));
        }
        validate_scramble(&self.draft.scramble, self.draft.event).map_err(|e| (2, e))?;
        parse_date(&date_text(self.draft.solved_at_unix_ms)).map_err(|e| (4, e))?;
        validate_comment(&self.draft.comment).map_err(|e| (5, e))?;
        Ok(())
    }
}

impl Model {
    /// Editing is available exclusively from idle solve details.
    pub fn open_solve_editor(&mut self) {
        if !self.show_details()
            || self.timer_state() != TimerState::Idle
            || self.solve_editor.is_some()
        {
            return;
        }
        let Some(index) = self.history().selected else {
            return;
        };
        let Some(time) = self.history().times().get(index) else {
            return;
        };
        let original = time.snapshot();
        self.solve_editor = Some(SolveEditor {
            session: self.current_session_index(),
            index,
            draft: original.clone(),
            original,
            selected: 0,
            event_selection: 0,
            page: EditorPage::Fields,
            error: None,
            history_scroll: 0,
            history_max_scroll: 0,
        });
    }

    /// Captures editor input, letting the configured quit action reach the event loop.
    pub fn handle_solve_editor_event(&mut self, event: &Event) -> Option<bool> {
        if !matches!(event, Event::Key(_) | Event::Paste(_)) {
            return None;
        }
        self.solve_editor.as_ref()?;
        let Event::Key(key) = event else {
            return Some(false);
        };
        if key.kind != KeyEventKind::Press {
            return Some(false);
        }
        let action = self.settings().keybinds().action_for(*key);
        if action == Some(Action::Quit) {
            return None;
        }
        let cancel = key.code == KeyCode::Esc
            || action == Some(Action::Back)
            || (key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL);
        let save = key.code == KeyCode::Char('s') && key.modifiers == KeyModifiers::CONTROL;
        let enter = key.code == KeyCode::Enter || action == Some(Action::Enter);
        let up =
            matches!(key.code, KeyCode::Up | KeyCode::BackTab) || action == Some(Action::SelectUp);
        let down = matches!(key.code, KeyCode::Down | KeyCode::Tab)
            || matches!(action, Some(Action::SelectDown | Action::ToggleFocus));
        let left = key.code == KeyCode::Left || action == Some(Action::NavigateLeft);
        let right = key.code == KeyCode::Right || action == Some(Action::NavigateRight);
        let editor = self.solve_editor.as_mut()?;
        match editor.page {
            EditorPage::History => {
                if cancel {
                    editor.page = EditorPage::Fields;
                } else if up {
                    editor.history_scroll = editor.history_scroll.saturating_sub(1);
                } else if down {
                    editor.history_scroll = editor
                        .history_scroll
                        .saturating_add(1)
                        .min(editor.history_max_scroll);
                } else if key.code == KeyCode::PageUp {
                    editor.history_scroll = editor.history_scroll.saturating_sub(10);
                } else if key.code == KeyCode::PageDown {
                    editor.history_scroll = editor
                        .history_scroll
                        .saturating_add(10)
                        .min(editor.history_max_scroll);
                } else if key.code == KeyCode::Home {
                    editor.history_scroll = 0;
                } else if key.code == KeyCode::End {
                    editor.history_scroll = editor.history_max_scroll;
                }
            }
            EditorPage::Events => {
                if cancel {
                    editor.page = EditorPage::Fields;
                } else if key.code == KeyCode::Home {
                    editor.event_selection = 0;
                } else if key.code == KeyCode::End {
                    editor.event_selection = WcaEvent::ALL.len() - 1;
                } else if up {
                    editor.event_selection = editor.event_selection.saturating_sub(1);
                } else if down {
                    editor.event_selection =
                        (editor.event_selection + 1).min(WcaEvent::ALL.len() - 1);
                } else if enter {
                    editor.draft.event = WcaEvent::ALL[editor.event_selection];
                    editor.page = EditorPage::Fields;
                    editor.error = None;
                }
            }
            EditorPage::Fields => {
                if cancel {
                    self.solve_editor = None;
                } else if save {
                    self.save_solve_editor();
                } else if key.code == KeyCode::Home {
                    editor.selected = 0;
                } else if key.code == KeyCode::End {
                    editor.selected = FIELD_NAMES.len() - 1;
                } else if up {
                    editor.selected = editor.selected.saturating_sub(1);
                } else if down {
                    editor.selected = (editor.selected + 1).min(FIELD_NAMES.len() - 1);
                } else if (left || right) && editor.selected == 3 {
                    editor.draft.modifier = match (editor.draft.modifier, left) {
                        (Modifier::None, false) | (Modifier::DNF, true) => Modifier::PlusTwo,
                        (Modifier::PlusTwo, false) | (Modifier::None, true) => Modifier::DNF,
                        _ => Modifier::None,
                    };
                } else if enter {
                    self.activate_solve_editor_field();
                }
            }
        }
        Some(true)
    }

    fn activate_solve_editor_field(&mut self) {
        let Some(editor) = &mut self.solve_editor else {
            return;
        };
        let field = editor.selected;
        match field {
            1 => {
                editor.event_selection = WcaEvent::ALL
                    .iter()
                    .position(|&event| event == editor.draft.event)
                    .unwrap_or(0);
                editor.page = EditorPage::Events;
            }
            3 => {
                editor.draft.modifier = match editor.draft.modifier {
                    Modifier::None => Modifier::PlusTwo,
                    Modifier::PlusTwo => Modifier::DNF,
                    Modifier::DNF => Modifier::None,
                };
            }
            6 => {
                editor.page = EditorPage::History;
                editor.history_scroll = 0;
            }
            7 => self.save_solve_editor(),
            8 => self.solve_editor = None,
            _ => {
                let initial = editor.value(field);
                let event = editor.draft.event;
                let title = match field {
                    0 => "Time: seconds / M:SS / H:MM:SS.mmm",
                    4 => "Solved at: RFC 3339 / blank = unknown",
                    _ => FIELD_NAMES[field],
                };
                self.request_validated_text_input(
                    title,
                    &initial,
                    move |text| match field {
                        0 => parse_time(text).map(|_| ()),
                        2 => validate_scramble(text, event),
                        4 => parse_date(text).map(|_| ()),
                        5 => validate_comment(text),
                        _ => Ok(()),
                    },
                    move |model, result| {
                        let TextInputResult::Submitted(text) = result else {
                            return;
                        };
                        let Some(editor) = &mut model.solve_editor else {
                            return;
                        };
                        match field {
                            0 => {
                                if let Ok(ms) = parse_time(&text) {
                                    editor.draft.time_ms = ms;
                                }
                            }
                            2 => text.trim().clone_into(&mut editor.draft.scramble),
                            4 => {
                                if let Ok(date) = parse_date(&text) {
                                    editor.draft.solved_at_unix_ms = date;
                                }
                            }
                            5 => editor.draft.comment = text,
                            _ => {}
                        }
                        editor.error = None;
                    },
                );
            }
        }
    }

    fn save_solve_editor(&mut self) {
        let Some(editor) = &mut self.solve_editor else {
            return;
        };
        if let Err((field, error)) = editor.validate() {
            editor.selected = field;
            editor.error = Some(error);
            return;
        }
        let Some(session) = self.session_state.sessions.get_mut(editor.session) else {
            return;
        };
        if session
            .history
            .times()
            .get(editor.index)
            .is_none_or(|time| time.snapshot() != editor.original)
        {
            editor.error =
                Some("This solve changed while editing. Cancel and reopen the editor.".to_owned());
            return;
        }
        let changed = session
            .history
            .edit_solve(editor.index, editor.draft.clone());
        session.history.select_index(editor.index);
        if editor.index + 1 == session.history.len() && session.timer_state == TimerState::Idle {
            session.last_time_ms = editor.draft.time_ms;
            session.last_modifier = editor.draft.modifier;
        }
        self.solve_editor = None;
        // Details must keep its return destination (including mean/stat screens).
        self.sync_details_modifier();
        if changed && self.save_history() {
            self.toast_info("Solve updated. Changes added to history.");
        }
    }
}
