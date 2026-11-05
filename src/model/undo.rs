//! Undo and redo for solve and session deletions during the current run.

use super::MAX_SESSIONS;
use super::session::SavedSession;
use crate::model::Model;
use crate::model::keybinds::Action;
use crate::widgets::history::Time;

/// How many deletions Undo remembers.
const UNDO_LIMIT: usize = 100;

/// Something deleted that can still be brought back.
enum Deleted {
    Solve {
        session: usize,
        index: usize,
        solve: Time,
    },
    Session {
        index: usize,
        session: Box<SavedSession>,
    },
}

/// An undone deletion, by where the restored data now is.
enum Undone {
    Solve { session: usize, index: usize },
    Session { index: usize },
}

#[derive(Default)]
pub(super) struct UndoHistory {
    /// Deletions Undo can restore, oldest first.
    undo: Vec<Deleted>,
    /// Undone deletions Redo can repeat, until a new deletion clears them.
    redo: Vec<Undone>,
}

impl UndoHistory {
    fn push(&mut self, deleted: Deleted) {
        self.undo.push(deleted);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
    }

    /// Remembers a new deletion and drops the redo history.
    fn record(&mut self, deleted: Deleted) {
        self.redo.clear();
        self.push(deleted);
    }
}

impl Model {
    /// Deletes the selected solve so that Undo can bring it back.
    pub fn delete_selected_solve(&mut self) {
        let session = self.current_session_index();
        if let Some((index, solve)) = self.history_mut().delete_selected() {
            self.undo.record(Deleted::Solve {
                session,
                index,
                solve,
            });
        }
    }

    /// Remembers a session that was just deleted.
    pub(super) fn record_session_deletion(&mut self, index: usize, session: SavedSession) {
        self.undo.record(Deleted::Session {
            index,
            session: Box::new(session),
        });
    }

    /// Returns the toast for a deletion of `what`, naming the key that undoes it.
    pub fn deleted_message(&self, what: &str) -> String {
        let key = self.settings().keybinds().label(Action::UndoDelete);
        format!("{what} deleted. Press {key} to undo.")
    }

    /// Restores the most recent deletion that can still be restored, returning
    /// what to tell the user once that is saved.
    pub fn undo_delete(&mut self) -> Option<String> {
        while let Some(deleted) = self.undo.undo.pop() {
            match deleted {
                Deleted::Solve {
                    session,
                    index,
                    solve,
                } => {
                    if session >= self.session_count() {
                        continue;
                    }
                    self.switch_to_session(session);
                    self.prepare_current_session();
                    let index = self.history_mut().insert_at(index, solve);
                    self.sync_details_modifier();
                    self.undo.redo.push(Undone::Solve { session, index });
                    return Some("Solve restored.".to_owned());
                }
                Deleted::Session { index, session } => {
                    if self.session_count() >= MAX_SESSIONS {
                        self.undo.undo.push(Deleted::Session { index, session });
                        self.toast_warning(format!(
                            "Cannot restore the session: the limit is {MAX_SESSIONS}."
                        ));
                        return None;
                    }
                    let index = self.insert_session(index, *session);
                    self.prepare_current_session();
                    self.undo.redo.push(Undone::Session { index });
                    return Some("Session restored.".to_owned());
                }
            }
        }
        self.toast_info("There is no deletion to undo.");
        None
    }

    /// Repeats the most recently undone deletion, returning what to tell the
    /// user once that is saved.
    pub fn redo_delete(&mut self) -> Option<String> {
        while let Some(undone) = self.undo.redo.pop() {
            match undone {
                Undone::Solve { session, index } => {
                    if session >= self.session_count() {
                        continue;
                    }
                    self.switch_to_session(session);
                    self.prepare_current_session();
                    let Some(solve) = self.history_mut().remove_at(index) else {
                        continue;
                    };
                    self.undo.push(Deleted::Solve {
                        session,
                        index,
                        solve,
                    });
                    if self.show_details() && self.history().is_empty() {
                        self.close_current_screen();
                    }
                    self.sync_details_modifier();
                    return Some(self.deleted_message("Solve"));
                }
                Undone::Session { index } => {
                    let Some(session) = self.remove_session(index) else {
                        continue;
                    };
                    self.undo.push(Deleted::Session {
                        index,
                        session: Box::new(session),
                    });
                    self.prepare_current_session();
                    return Some(self.deleted_message("Session"));
                }
            }
        }
        self.toast_info("There is no deletion to redo.");
        None
    }

    /// Gives the active session the scramble it may not have generated yet.
    fn prepare_current_session(&mut self) {
        if self.current_session().scramble.is_none() {
            self.next_scramble();
        }
    }

    /// Returns how many deletions Undo and Redo hold.
    #[cfg(test)]
    fn undo_depth(&self) -> (usize, usize) {
        (self.undo.undo.len(), self.undo.redo.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Model;
    use crate::scramble::WcaEvent;
    use crate::widgets::history::Modifier;

    fn model_with(times: &[u64]) -> Model {
        let mut model = Model::new();
        add_solves(&mut model, times);
        model
    }

    fn add_solves(model: &mut Model, times: &[u64]) {
        for &ms in times {
            model.history_mut().add_ms(ms, WcaEvent::Cube3x3, "R U");
        }
    }

    fn times(model: &Model) -> Vec<u64> {
        model.history().times().iter().map(Time::raw_ms).collect()
    }

    fn delete_at(model: &mut Model, index: usize) {
        model.history_mut().select_index(index);
        model.delete_selected_solve();
    }

    #[test]
    fn undo_restores_a_solve_where_it_was_and_selects_it() {
        let mut model = model_with(&[10, 20, 30]);
        delete_at(&mut model, 1);
        assert_eq!(times(&model), [10, 30]);

        assert_eq!(model.undo_delete().unwrap(), "Solve restored.");

        assert_eq!(times(&model), [10, 20, 30]);
        assert_eq!(model.history().selected, Some(1));
    }

    #[test]
    fn undo_restores_statistics() {
        let mut model = model_with(&[30, 10, 20]);
        delete_at(&mut model, 1);
        assert_eq!(model.history().get_fastest_time().unwrap().raw_ms(), 20);

        model.undo_delete();

        assert_eq!(model.history().get_fastest_time().unwrap().raw_ms(), 10);
    }

    #[test]
    fn undo_keeps_a_solves_metadata() {
        let mut model = model_with(&[10]);
        model.history_mut().select_index(0);
        model.history_mut().set_modifier(Modifier::PlusTwo);
        delete_at(&mut model, 0);

        model.undo_delete();

        assert_eq!(model.history().times()[0].modifier(), Modifier::PlusTwo);
    }

    #[test]
    fn repeated_undo_walks_back_and_redo_walks_forward() {
        let mut model = model_with(&[10, 20, 30]);
        delete_at(&mut model, 2);
        delete_at(&mut model, 0);
        assert_eq!(times(&model), [20]);

        model.undo_delete();
        assert_eq!(times(&model), [10, 20]);
        model.undo_delete();
        assert_eq!(times(&model), [10, 20, 30]);

        model.redo_delete();
        assert_eq!(times(&model), [10, 20]);
        model.redo_delete();
        assert_eq!(times(&model), [20]);
        assert_eq!(model.undo_depth(), (2, 0));
    }

    #[test]
    fn a_new_deletion_drops_the_redo_history() {
        let mut model = model_with(&[10, 20]);
        delete_at(&mut model, 0);
        model.undo_delete();
        delete_at(&mut model, 1);

        assert!(model.redo_delete().is_none());
        assert_eq!(times(&model), [10]);
    }

    #[test]
    fn a_solve_recorded_since_stays_after_undo() {
        let mut model = model_with(&[10, 20]);
        delete_at(&mut model, 0);
        add_solves(&mut model, &[30]);

        model.undo_delete();

        assert_eq!(times(&model), [10, 20, 30]);
    }

    #[test]
    fn undo_returns_to_the_session_the_solve_was_in() {
        let mut model = model_with(&[10, 20]);
        delete_at(&mut model, 0);
        model.add_session();
        assert_eq!(model.current_session_index(), 1);
        model.open_details();

        model.undo_delete();

        assert_eq!(model.current_session_index(), 0);
        assert_eq!(times(&model), [10, 20]);
        assert!(!model.show_details());
    }

    #[test]
    fn undo_restores_a_session_in_place_with_its_selection_and_event() {
        let mut model = model_with(&[10]);
        model.add_session();
        model.next_scramble();
        model.next_event();
        let event = model.event();
        add_solves(&mut model, &[20, 30]);
        model.history_mut().set_session_name("Practice".to_owned());
        model.history_mut().select_index(0);
        let scramble = model.scramble().to_owned();
        model.add_session();
        model.prev_session();

        assert!(model.delete_current_session());
        assert_eq!(model.session_count(), 2);
        assert_eq!(model.current_session_index(), 1);

        assert_eq!(model.undo_delete().unwrap(), "Session restored.");

        assert_eq!(model.session_count(), 3);
        assert_eq!(model.current_session_index(), 1);
        assert_eq!(model.history().session_name(), "Practice");
        assert_eq!(times(&model), [20, 30]);
        assert_eq!(model.history().selected, Some(0));
        assert_eq!(model.event(), event);
        assert_eq!(model.scramble(), scramble);
    }

    #[test]
    fn redo_deletes_a_session_again_after_moving_away() {
        let mut model = model_with(&[10]);
        model.add_session();
        add_solves(&mut model, &[20]);
        model.delete_current_session();
        model.undo_delete();
        model.prev_session();

        model.redo_delete();

        assert_eq!(model.session_count(), 1);
        assert_eq!(times(&model), [10]);
        model.undo_delete();
        assert_eq!(times(&model), [20]);
    }

    #[test]
    fn undo_goes_through_a_session_to_the_solve_deleted_from_it() {
        let mut model = model_with(&[10]);
        model.add_session();
        add_solves(&mut model, &[20, 30]);
        delete_at(&mut model, 0);
        model.delete_current_session();

        model.undo_delete();
        model.undo_delete();

        assert_eq!(model.current_session_index(), 1);
        assert_eq!(times(&model), [20, 30]);
    }

    #[test]
    fn a_session_is_not_restored_past_the_limit_and_stays_undoable() {
        let mut model = model_with(&[10]);
        model.add_session();
        model.delete_current_session();
        while model.add_session() {}

        assert!(model.undo_delete().is_none());
        assert_eq!(model.session_count(), MAX_SESSIONS);
        assert_eq!(model.undo_depth(), (1, 0));
    }

    #[test]
    fn undo_remembers_a_bounded_number_of_deletions() {
        let mut model = Model::new();
        for ms in 0..=UNDO_LIMIT as u64 {
            add_solves(&mut model, &[ms]);
            delete_at(&mut model, 0);
        }
        assert_eq!(model.undo_depth(), (UNDO_LIMIT, 0));
    }

    #[test]
    fn there_being_nothing_to_undo_changes_nothing() {
        let mut model = model_with(&[10]);
        assert!(model.undo_delete().is_none());
        assert!(model.redo_delete().is_none());
        assert_eq!(times(&model), [10]);
    }

    #[test]
    fn the_details_screen_follows_undo_and_closes_when_redo_empties_the_session() {
        let mut model = model_with(&[10]);
        model.history_mut().select_index(0);
        model.history_mut().set_modifier(Modifier::DNF);
        model.open_details();
        delete_at(&mut model, 0);

        model.undo_delete();
        assert!(model.show_details());
        assert_eq!(model.selected_details_modifier(), Modifier::DNF);

        model.redo_delete();
        assert!(!model.show_details());
    }

    #[test]
    fn the_deletion_message_names_the_configured_key() {
        let model = Model::new();
        assert_eq!(
            model.deleted_message("Solve"),
            "Solve deleted. Press Ctrl+z to undo."
        );
    }
}
