//! Application integration for the reusable text-input modal.
//!
//! [`Model::request_text_input`] opens a prompt above screens, dialogs, and
//! toasts. Keyboard and paste events are captured before global keybindings,
//! including quit and the timer key. Completion is delivered through a callback
//! while the normal event loop continues handling resizing and background work.
//! The underlying screen is preserved and a running timer is not paused;
//! action handlers should decide when opening a prompt is appropriate.

use std::ops::ControlFlow;

use ratatui::crossterm::event::Event;

use crate::model::Model;
use crate::widgets::text_input::{TextInput, TextInputResult};

type OnComplete = Box<dyn FnOnce(&mut Model, TextInputResult)>;

pub struct TextInputPrompt {
    pub input: TextInput,
    on_complete: OnComplete,
}

impl Model {
    /// Opens a text prompt above every screen and overlay.
    ///
    /// Returns `false` if a prompt is already open, preserving its input and
    /// callback. Otherwise the callback runs once on submission or cancellation,
    /// after closing the prompt, so it can update the model or open another one.
    ///
    /// `initial` supplies the starting text; use `""` for an empty input.
    /// The callback receives the owned submitted string (which may be empty)
    /// or [`TextInputResult::Cancelled`]. It can capture owned context with
    /// `move` and receives mutable access to the model. The event loop keeps
    /// running while input is collected.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::widgets::text_input::TextInputResult;
    ///
    /// model.request_text_input("Enter a name", "", |model, result| {
    ///     match result {
    ///         TextInputResult::Submitted(text) => {
    ///             // Store the owned String or pass it to the next action.
    ///             model.toast_info(format!("Entered: {text}"));
    ///         }
    ///         TextInputResult::Cancelled => {}
    ///     }
    /// });
    /// ```
    pub fn request_text_input(
        &mut self,
        title: impl Into<String>,
        initial: &str,
        on_complete: impl FnOnce(&mut Self, TextInputResult) + 'static,
    ) -> bool {
        if self.text_input.is_some() {
            return false;
        }
        self.text_input = Some(TextInputPrompt {
            input: TextInput::new(title, initial),
            on_complete: Box::new(on_complete),
        });
        true
    }

    /// Captures keyboard and paste before global keybinds are considered.
    /// `None` means the event belongs to the normal application event handler.
    pub fn handle_text_input_event(&mut self, event: &Event) -> Option<bool> {
        if !matches!(event, Event::Key(_) | Event::Paste(_)) {
            return None;
        }
        let prompt = self.text_input.as_mut()?;
        match prompt.input.handle_event(event) {
            ControlFlow::Continue(changed) => Some(changed),
            ControlFlow::Break(result) => {
                let prompt = self.text_input.take().expect("text prompt is open");
                (prompt.on_complete)(self, result);
                Some(true)
            }
        }
    }
}
