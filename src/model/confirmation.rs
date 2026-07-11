use crate::model::Model;
use crate::widgets::confirmation::Selection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmationAction {
    DeleteSession,
    DeleteTime,
}

impl ConfirmationAction {
    pub const fn message(self) -> &'static str {
        match self {
            Self::DeleteSession => "Delete this session?",
            Self::DeleteTime => "Delete this time?",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Confirmation {
    pub message: String,
    pub selection: Selection,
    pub action: ConfirmationAction,
}

impl Model {
    pub fn open_confirmation(&mut self, action: ConfirmationAction) {
        self.confirmation = Some(Confirmation {
            message: action.message().to_owned(),
            selection: Selection::No,
            action,
        });
    }

    pub fn close_confirmation(&mut self) {
        self.confirmation = None;
    }

    pub const fn confirmation_selection_left(&mut self) {
        if let Some(confirmation) = &mut self.confirmation {
            confirmation.selection = Selection::No;
        }
    }

    pub const fn confirmation_selection_right(&mut self) {
        if let Some(confirmation) = &mut self.confirmation {
            confirmation.selection = Selection::Yes;
        }
    }

    pub const fn confirmation(&self) -> Option<&Confirmation> {
        self.confirmation.as_ref()
    }
}
