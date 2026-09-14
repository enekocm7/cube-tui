use ratatui::crossterm::event::{KeyEvent, KeyEventKind};

use crate::model::Model;
use crate::model::keybinds::{Action, Keybinds};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    Press,
    Release,
    Reset,
    Tick,
    SelectUp,
    SelectDown,
    Quit,
    Help,
    NextEventOpenEditor,
    PrevEvent,
    NextSession,
    PrevSession,
    NewSession,
    DeleteSession,
    ToggleInspection,
    NextScramble,
    Enter,
    Esc,
    OpenDetailedStats,
    OpenThemeSelector,
    DeleteTime,
    NavLeft,
    NavRight,
    ToggleFocus,
    #[cfg(feature = "bluetooth")]
    ToggleBluetooth,
    #[cfg(feature = "bluetooth")]
    DisconnectBluetooth,
    ToggleZen,
}

/// Maps a terminal key event to the configured application message.
///
/// Key-repeat events are deliberately ignored so held keys cannot trigger
/// repeated state transitions.
pub fn map_key_to_msg(key: KeyEvent, keybinds: &Keybinds) -> Option<Msg> {
    let action = keybinds.action_for(key)?;
    if action == Action::Timer {
        return match key.kind {
            KeyEventKind::Press => Some(Msg::Press),
            KeyEventKind::Release => Some(Msg::Release),
            KeyEventKind::Repeat => None,
        };
    }
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match action {
        Action::Quit => Some(Msg::Quit),
        Action::ResetTimer => Some(Msg::Reset),
        Action::SelectUp => Some(Msg::SelectUp),
        Action::SelectDown => Some(Msg::SelectDown),
        Action::NavigateLeft => Some(Msg::NavLeft),
        Action::NavigateRight => Some(Msg::NavRight),
        Action::ToggleFocus => Some(Msg::ToggleFocus),
        Action::NextEvent => Some(Msg::NextEventOpenEditor),
        Action::PreviousEvent => Some(Msg::PrevEvent),
        Action::NextSession => Some(Msg::NextSession),
        Action::PreviousSession => Some(Msg::PrevSession),
        Action::NewSession => Some(Msg::NewSession),
        Action::DeleteSession => Some(Msg::DeleteSession),
        Action::NextScramble => Some(Msg::NextScramble),
        Action::Help => Some(Msg::Help),
        Action::ToggleInspection => Some(Msg::ToggleInspection),
        Action::DetailedStats => Some(Msg::OpenDetailedStats),
        Action::ThemeSelector => Some(Msg::OpenThemeSelector),
        Action::DeleteTime => Some(Msg::DeleteTime),
        #[cfg(feature = "bluetooth")]
        Action::Bluetooth => Some(Msg::ToggleBluetooth),
        #[cfg(feature = "bluetooth")]
        Action::DisconnectBluetooth => Some(Msg::DisconnectBluetooth),
        #[cfg(not(feature = "bluetooth"))]
        Action::Bluetooth | Action::DisconnectBluetooth => None,
        Action::ToggleZen => Some(Msg::ToggleZen),
        Action::Enter => Some(Msg::Enter),
        Action::Back => Some(Msg::Esc),
        Action::Timer => unreachable!(),
    }
}

/// Returns whether a message is valid for the currently active modal or screen.
///
/// Modal interfaces capture input so unrelated global actions cannot mutate the
/// obscured main screen; ticking and quitting remain available everywhere.
pub const fn allowed_msg(model: &Model, msg: Msg) -> bool {
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        return matches!(
            msg,
            Msg::SelectUp
                | Msg::SelectDown
                | Msg::Enter
                | Msg::Esc
                | Msg::ToggleBluetooth
                | Msg::DisconnectBluetooth
                | Msg::Tick
                | Msg::Quit
        );
    }
    if model.show_help() {
        return matches!(
            msg,
            Msg::SelectUp | Msg::SelectDown | Msg::Help | Msg::Esc | Msg::Tick | Msg::Quit
        );
    }
    if model.confirmation().is_some() {
        return matches!(
            msg,
            Msg::NavLeft | Msg::NavRight | Msg::Enter | Msg::Esc | Msg::Tick | Msg::Quit
        );
    }
    if model.theme_selector.is_some() {
        return matches!(
            msg,
            Msg::SelectUp
                | Msg::SelectDown
                | Msg::Esc
                | Msg::Tick
                | Msg::Quit
                | Msg::OpenThemeSelector
                | Msg::NextEventOpenEditor
                | Msg::Enter
        );
    }

    if model.show_details() {
        return matches!(
            msg,
            Msg::SelectUp
                | Msg::SelectDown
                | Msg::NavLeft
                | Msg::NavRight
                | Msg::Press
                | Msg::Release
                | Msg::DeleteTime
                | Msg::Esc
                | Msg::Tick
                | Msg::Quit
        );
    }
    if model.show_mean_detail() {
        return matches!(
            msg,
            Msg::SelectUp | Msg::SelectDown | Msg::Enter | Msg::Esc | Msg::Tick | Msg::Quit
        );
    }
    if model.show_detailed_stats() {
        return matches!(
            msg,
            Msg::SelectUp
                | Msg::SelectDown
                | Msg::NavLeft
                | Msg::NavRight
                | Msg::Enter
                | Msg::Esc
                | Msg::Tick
                | Msg::Quit
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    use super::{Msg, map_key_to_msg};
    use crate::model::keybinds::Keybinds;

    #[test]
    fn default_timer_binding_maps_press_and_release() {
        let keybinds = Keybinds::default();
        let press = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let release = KeyEvent {
            code: KeyCode::Char(' '),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };

        assert_eq!(map_key_to_msg(press, &keybinds), Some(Msg::Press));
        assert_eq!(map_key_to_msg(release, &keybinds), Some(Msg::Release));
    }

    #[test]
    fn custom_binding_maps_with_modifiers() {
        let keybinds: Keybinds = toml::from_str("next_scramble = \"Ctrl+n\"").unwrap();
        let event = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);

        assert_eq!(map_key_to_msg(event, &keybinds), Some(Msg::NextScramble));
    }

    #[test]
    fn repeat_events_are_ignored() {
        let keybinds = Keybinds::default();
        let event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Repeat,
            state: KeyEventState::NONE,
        };

        assert_eq!(map_key_to_msg(event, &keybinds), None);
    }
}
