use ratatui::crossterm::event::{KeyCode, KeyEventKind};

use crate::model::Model;

#[derive(Copy, Clone, Debug)]
pub(crate) enum Msg {
    Press,
    Release,
    Reset,
    Tick,
    SelectUp,
    SelectDown,
    Quit,
    Help,
    NextEvent,
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

pub(crate) const fn map_key_to_msg(code: KeyCode, kind: KeyEventKind) -> Option<Msg> {
    match (code, kind) {
        (KeyCode::Char('q'), KeyEventKind::Press) => Some(Msg::Quit),
        (KeyCode::Char('r'), KeyEventKind::Press) => Some(Msg::Reset),
        (KeyCode::Char(' '), KeyEventKind::Press) => Some(Msg::Press),
        (KeyCode::Char(' '), KeyEventKind::Release) => Some(Msg::Release),
        (KeyCode::Up, KeyEventKind::Press) => Some(Msg::SelectUp),
        (KeyCode::Down, KeyEventKind::Press) => Some(Msg::SelectDown),
        (KeyCode::Left, KeyEventKind::Press) => Some(Msg::NavLeft),
        (KeyCode::Right, KeyEventKind::Press) => Some(Msg::NavRight),
        (KeyCode::Tab, KeyEventKind::Press) => Some(Msg::ToggleFocus),
        (KeyCode::Char('e'), KeyEventKind::Press) => Some(Msg::NextEvent),
        (KeyCode::Char('E'), KeyEventKind::Press) => Some(Msg::PrevEvent),
        (KeyCode::Char(']'), KeyEventKind::Press) => Some(Msg::NextSession),
        (KeyCode::Char('['), KeyEventKind::Press) => Some(Msg::PrevSession),
        (KeyCode::Char('s'), KeyEventKind::Press) => Some(Msg::NewSession),
        (KeyCode::Char('S'), KeyEventKind::Press) => Some(Msg::DeleteSession),
        (KeyCode::Char('n'), KeyEventKind::Press) => Some(Msg::NextScramble),
        (KeyCode::Char('?'), KeyEventKind::Press) => Some(Msg::Help),
        (KeyCode::Char('i'), KeyEventKind::Press) => Some(Msg::ToggleInspection),
        (KeyCode::Char('t'), KeyEventKind::Press) => Some(Msg::OpenDetailedStats),
        (KeyCode::Char('d'), KeyEventKind::Press) => Some(Msg::DeleteTime),
        #[cfg(feature = "bluetooth")]
        (KeyCode::Char('b'), KeyEventKind::Press) => Some(Msg::ToggleBluetooth),
        #[cfg(feature = "bluetooth")]
        (KeyCode::Char('x'), KeyEventKind::Press) => Some(Msg::DisconnectBluetooth),
        (KeyCode::Char('z'), KeyEventKind::Press) => Some(Msg::ToggleZen),
        (KeyCode::Enter, KeyEventKind::Press) => Some(Msg::Enter),
        (KeyCode::Esc, KeyEventKind::Press) => Some(Msg::Esc),
        _ => None,
    }
}

pub(crate) const INSPECTION_LIMIT_MS: u64 = 15_000;

pub(crate) const fn allowed_msg(model: &Model, msg: Msg) -> bool {
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
            Msg::SelectUp | Msg::SelectDown | Msg::Help | Msg::Tick | Msg::Quit
        );
    }
    if model.screen.show_confirm_delete_session() {
        return matches!(
            msg,
            Msg::NavLeft | Msg::NavRight | Msg::Enter | Msg::Esc | Msg::Tick | Msg::Quit
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
