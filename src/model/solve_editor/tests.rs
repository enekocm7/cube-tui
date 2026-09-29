use ratatui::buffer::{Buffer, Cell};
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;

use super::*;
use crate::widgets::history::{History, Time};

fn model_with_solve() -> Model {
    let mut model = Model::new();
    model.history_load_failed = true; // Never write the user's history from tests.
    model.history_mut().add(Time::new_with_meta(
        1234,
        WcaEvent::Cube3x3,
        "R U F2".into(),
        1_700_000_000_123,
        Modifier::None,
    ));
    model.open_details();
    model.open_solve_editor();
    model
}

fn key(model: &mut Model, code: KeyCode) {
    let _ =
        crate::handle_terminal_event(model, &Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

fn replace_field(model: &mut Model, field: usize, text: &str) {
    let editor = model.solve_editor.as_mut().unwrap();
    editor.selected = field;
    let len = editor.value(field).chars().count();
    key(model, KeyCode::Enter);
    key(model, KeyCode::Home);
    for _ in 0..len {
        key(model, KeyCode::Delete);
    }
    let _ = crate::handle_terminal_event(model, &Event::Paste(text.to_owned()));
    key(model, KeyCode::Enter);
}

#[test]
fn full_editor_saves_all_fields_atomically_with_a_persistent_revision() {
    let mut model = model_with_solve();
    let before = model.history().last().unwrap().snapshot();
    replace_field(&mut model, 0, "1:02.345");
    model.solve_editor.as_mut().unwrap().selected = 1;
    key(&mut model, KeyCode::Enter);
    key(&mut model, KeyCode::Down); // 4x4
    key(&mut model, KeyCode::Enter);
    replace_field(&mut model, 2, "Rw U F2");
    model.solve_editor.as_mut().unwrap().selected = 3;
    key(&mut model, KeyCode::Enter); // +2
    replace_field(&mut model, 4, "2026-09-29T14:30:00.123+02:00");
    replace_field(&mut model, 5, "PB 界 👩‍💻");
    assert_eq!(model.history().last().unwrap().snapshot(), before);
    assert!(model.history().last().unwrap().changes().is_empty());
    model.solve_editor.as_mut().unwrap().selected = 7;
    key(&mut model, KeyCode::Enter);
    assert!(model.solve_editor.is_none());
    assert!(model.show_details());
    let time = model.history().last().unwrap();
    assert_eq!(time.raw_ms(), 62_345);
    assert_eq!(time.effective_ms(), Some(64_345));
    assert_eq!(time.event(), WcaEvent::Cube4x4);
    assert_eq!(time.scramble(), "Rw U F2");
    assert_eq!(time.comment(), "PB 界 👩‍💻");
    assert_eq!(
        date_text(time.solved_at_unix_ms()),
        "2026-09-29T12:30:00.123Z"
    );
    assert_eq!(time.changes().len(), 1);
    assert_eq!(time.changes()[0].before, before);
    assert_eq!(time.changes()[0].after, time.snapshot());
    assert!(time.changes()[0].changed_at_unix_ms > 0);
    assert_eq!(model.elapsed_ms(), 62_345);
    assert_eq!(model.displayed_modifier(), Modifier::PlusTwo);
    let json = serde_json::to_string(model.history()).unwrap();
    let restored: History = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.last().unwrap().snapshot(), time.snapshot());
    assert_eq!(restored.last().unwrap().changes()[0].before, before);
    model.open_solve_editor();
    model.save_solve_editor();
    assert_eq!(model.history().last().unwrap().changes().len(), 1);
}

#[test]
fn invalid_input_stays_editable_and_cancellation_discards_the_entire_draft() {
    let mut model = model_with_solve();
    let before = model.history().last().unwrap().snapshot();
    replace_field(&mut model, 0, "1:99");
    assert!(model.text_input.is_some());
    assert_eq!(
        model.solve_editor.as_ref().unwrap().draft.time_ms,
        before.time_ms
    );
    // Correct the same rejected input in place.
    key(&mut model, KeyCode::Backspace);
    key(&mut model, KeyCode::Backspace);
    let _ = crate::handle_terminal_event(&mut model, &Event::Paste("02.5".into()));
    key(&mut model, KeyCode::Enter);
    assert!(model.text_input.is_none());
    assert_eq!(model.solve_editor.as_ref().unwrap().draft.time_ms, 62_500);
    replace_field(&mut model, 5, "discard me");
    for code in [KeyCode::Char(' '), KeyCode::Char('s'), KeyCode::Char('d')] {
        assert!(
            !crate::handle_terminal_event(
                &mut model,
                &Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
            )
            .is_break()
        );
    }
    assert_eq!(model.session_count(), 1);
    assert_eq!(model.timer_state(), TimerState::Idle);
    key(&mut model, KeyCode::Esc);
    assert!(model.solve_editor.is_none());
    assert!(model.show_details());
    assert_eq!(model.history().last().unwrap().snapshot(), before);
    assert!(model.history().last().unwrap().changes().is_empty());
}

#[test]
fn quit_exits_every_editor_page_and_respects_custom_bindings() {
    for binding in ["q", "Ctrl+q"] {
        let mut model = model_with_solve();
        model.set_settings(toml::from_str(&format!("[keybinds]\nquit = '{binding}'")).unwrap());
        let modifiers = if binding == "q" {
            KeyModifiers::NONE
        } else {
            KeyModifiers::CONTROL
        };
        let quit = Event::Key(KeyEvent::new(KeyCode::Char('q'), modifiers));
        for page in [EditorPage::Fields, EditorPage::Events, EditorPage::History] {
            model.solve_editor.as_mut().unwrap().page = page;
            assert!(crate::handle_terminal_event(&mut model, &quit).is_break());
        }
        if binding != "q" {
            let plain_q = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
            assert!(!crate::handle_terminal_event(&mut model, &plain_q).is_break());
        }
        let mut released = KeyEvent::new(KeyCode::Char('q'), modifiers);
        released.kind = KeyEventKind::Release;
        assert!(!crate::handle_terminal_event(&mut model, &Event::Key(released)).is_break());
    }
}

#[test]
fn quit_letter_is_editable_text_inside_a_solve_field() {
    let mut model = model_with_solve();
    model.solve_editor.as_mut().unwrap().selected = 5;
    key(&mut model, KeyCode::Enter);
    let quit = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    assert!(!crate::handle_terminal_event(&mut model, &quit).is_break());
    key(&mut model, KeyCode::Enter);
    assert_eq!(model.solve_editor.as_ref().unwrap().draft.comment, "q");
    assert!(crate::handle_terminal_event(&mut model, &quit).is_break());
    assert_eq!(model.history().last().unwrap().comment(), "");
}

#[test]
fn event_changes_revalidate_scrambles_and_zero_times_require_dnf() {
    let mut model = model_with_solve();
    model.solve_editor.as_mut().unwrap().draft.event = WcaEvent::Clock;
    model.save_solve_editor();
    assert!(
        model
            .solve_editor
            .as_ref()
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("Clock")
    );
    assert_eq!(model.history().last().unwrap().event(), WcaEvent::Cube3x3);
    replace_field(&mut model, 2, "UR3+ DR2- y2 U0+");
    replace_field(&mut model, 0, "0");
    model.save_solve_editor();
    assert!(model.solve_editor.is_some());
    model.solve_editor.as_mut().unwrap().draft.modifier = Modifier::DNF;
    model.save_solve_editor();
    assert!(model.solve_editor.is_none());
    assert_eq!(model.history().last().unwrap().modifier(), Modifier::DNF);
    assert_eq!(model.history().last().unwrap().raw_ms(), 0);
}

#[test]
fn home_and_end_jump_to_editor_and_event_picker_boundaries() {
    let mut model = model_with_solve();
    key(&mut model, KeyCode::End);
    assert_eq!(
        model.solve_editor.as_ref().unwrap().selected,
        FIELD_NAMES.len() - 1
    );
    key(&mut model, KeyCode::Home);
    assert_eq!(model.solve_editor.as_ref().unwrap().selected, 0);
    key(&mut model, KeyCode::Down);
    key(&mut model, KeyCode::Enter);
    key(&mut model, KeyCode::End);
    assert_eq!(
        model.solve_editor.as_ref().unwrap().event_selection,
        WcaEvent::ALL.len() - 1
    );
    key(&mut model, KeyCode::Home);
    assert_eq!(model.solve_editor.as_ref().unwrap().event_selection, 0);
    key(&mut model, KeyCode::Enter);
    assert_eq!(
        model.solve_editor.as_ref().unwrap().draft.event,
        WcaEvent::Cube2x2
    );
}

#[test]
fn editor_is_details_only_and_honors_the_configured_shortcut() {
    let mut model = Model::new();
    let settings = toml::from_str("[keybinds]\nedit_solve = 'Ctrl+e'").unwrap();
    model.set_settings(settings);
    let edit = Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    let _ = crate::handle_terminal_event(&mut model, &edit);
    assert!(model.solve_editor.is_none());
    model.history_mut().add_ms(1000, WcaEvent::Cube3x3, "R U");
    model.open_details();
    key(&mut model, KeyCode::F(3));
    assert!(model.solve_editor.is_none());
    model.set_timer_state(TimerState::Pulsed);
    let _ = crate::handle_terminal_event(&mut model, &edit);
    assert!(model.solve_editor.is_none());
    model.reset_timer();
    let _ = crate::handle_terminal_event(&mut model, &edit);
    assert!(model.solve_editor.is_some());
}

#[test]
fn saved_edits_invalidate_statistics_and_penalty_toggles_are_audited() {
    let mut history = History::new();
    for time in [1000, 2000, 3000, 4000, 5000] {
        history.add_ms(time, WcaEvent::Cube3x3, "R U");
    }
    assert_eq!(history.get_fastest_time().unwrap().raw_ms(), 1000);
    assert_eq!(history.get_fastest_mo3().unwrap(), "00:02.000");
    assert_eq!(history.get_fastest_ao5().unwrap(), "00:03.000");
    let mut after = history.times()[0].snapshot();
    after.time_ms = 10_000;
    assert!(history.edit_solve(0, after));
    assert_eq!(history.get_fastest_time().unwrap().raw_ms(), 2000);
    assert_eq!(history.get_fastest_mo3().unwrap(), "00:03.000");
    assert_eq!(history.get_fastest_ao5().unwrap(), "00:04.000");
    history.select_index(0);
    history.set_modifier(Modifier::PlusTwo);
    history.set_modifier(Modifier::PlusTwo);
    let time = history.selected_time().unwrap();
    assert_eq!(time.changes().len(), 3);
    assert_eq!(time.changes()[2].before.modifier, Modifier::PlusTwo);
    assert_eq!(time.changes()[2].after.modifier, Modifier::None);
    let legacy: Time =
        serde_json::from_str(r#"{"timestamp_in_millis":1000,"event":"Cube3x3","scramble":"R U"}"#)
            .unwrap();
    assert_eq!(legacy.comment(), "");
    assert!(legacy.changes().is_empty());
}

#[test]
fn editor_and_scrollable_history_render_with_before_and_after_values() {
    let mut model = model_with_solve();
    replace_field(&mut model, 5, "note");
    model.save_solve_editor();
    model.open_solve_editor();
    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    crate::view::view(area, &mut buffer, &mut model);
    let text = |buffer: &Buffer| buffer.content.iter().map(Cell::symbol).collect::<String>();
    assert!(text(&buffer).contains("Edit solve #1"));
    model.solve_editor.as_mut().unwrap().selected = 6;
    key(&mut model, KeyCode::Enter);
    crate::view::view(area, &mut buffer, &mut model);
    let rendered = text(&buffer);
    assert!(rendered.contains("Before: (empty)"));
    assert!(rendered.contains("After:  note"));
    key(&mut model, KeyCode::Esc);
    assert!(model.solve_editor.as_ref().unwrap().page == EditorPage::Fields);
}
