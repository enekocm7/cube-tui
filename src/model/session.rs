use super::MAX_SESSIONS;
use crate::model::Model;
#[cfg(feature = "bluetooth")]
use crate::model::bluetooth::BluetoothState;
use crate::model::help::HelpState;
use crate::model::main_focus::{MainFocus, MainStatsSelection};
use crate::model::screen::Screen;
use crate::scramble::{self, Scramble, WcaEvent};
use crate::widgets::history::History;
use std::time::Instant;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimerState {
    Idle,
    Pulsed,
    Inspection(InspectionState),
    Running(Instant),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InspectionState {
    Pulsed(Instant),
    Running(Instant),
}

pub struct Session {
    pub timer_state: TimerState,
    pub history: History,
    pub scramble: Option<Scramble>,
    pub last_time_ms: u64,
    pub event: WcaEvent,
}

impl Session {
    pub fn new() -> Self {
        let event = WcaEvent::Cube3x3;
        let scramble = None;
        Self {
            timer_state: TimerState::Idle,
            history: History::new(),
            scramble,
            last_time_ms: 0,
            event,
        }
    }

    pub const fn reset_timer(&mut self) {
        self.timer_state = TimerState::Idle;
        self.last_time_ms = 0;
    }

    pub fn start_inspection(&mut self) {
        self.timer_state = TimerState::Inspection(InspectionState::Running(Instant::now()));
    }

    pub fn start_timer(&mut self) {
        self.timer_state = TimerState::Running(Instant::now());
    }

    pub const fn stop_timer(&mut self) {
        self.timer_state = TimerState::Idle;
    }

    pub const fn pulse_timer(&mut self) {
        if let TimerState::Inspection(InspectionState::Running(start)) = self.timer_state {
            self.timer_state = TimerState::Inspection(InspectionState::Pulsed(start));
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        match self.timer_state {
            TimerState::Inspection(state) => match state {
                InspectionState::Running(start) | InspectionState::Pulsed(start) => {
                    u64::try_from(start.elapsed().as_millis()).unwrap()
                }
            },
            TimerState::Running(start) => u64::try_from(start.elapsed().as_millis()).unwrap(),
            TimerState::Idle | TimerState::Pulsed => self.last_time_ms,
        }
    }

    pub fn next_scramble(&mut self) {
        self.scramble = Some(scramble::generate_scramble(self.event));
    }

    pub fn next_event(&mut self) {
        self.event = self.event.next();
        self.next_scramble();
    }

    pub fn prev_event(&mut self) {
        self.event = self.event.prev();
        self.next_scramble();
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SessionState {
    pub sessions: Vec<Session>,
    pub current_session_index: usize,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            sessions: vec![Session::new()],
            current_session_index: 0,
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub const fn current_session_index(&self) -> usize {
        self.session_state.current_session_index
    }

    pub const fn session_count(&self) -> usize {
        self.session_state.sessions.len()
    }

    pub const fn is_at_max_sessions(&self) -> bool {
        self.session_state.sessions.len() >= MAX_SESSIONS
    }

    pub fn current_session(&self) -> &Session {
        &self.session_state.sessions[self.session_state.current_session_index]
    }

    pub fn current_session_mut(&mut self) -> &mut Session {
        &mut self.session_state.sessions[self.session_state.current_session_index]
    }

    pub fn add_session(&mut self) -> bool {
        if self.is_at_max_sessions() {
            return false;
        }
        self.session_state.sessions.push(Session::new());
        self.session_state.current_session_index = self.session_state.sessions.len() - 1;
        true
    }

    pub fn delete_current_session(&mut self) -> bool {
        if self.session_state.sessions.len() <= 1 {
            return false;
        }

        self.session_state
            .sessions
            .remove(self.session_state.current_session_index);
        if self.session_state.current_session_index >= self.session_state.sessions.len() {
            self.session_state.current_session_index = self.session_state.sessions.len() - 1;
        }

        self.screen = Screen::default();
        #[cfg(feature = "bluetooth")]
        {
            self.bluetooth_state = BluetoothState::default();
        }

        true
    }

    pub const fn next_session(&mut self) {
        if self.session_state.sessions.is_empty() {
            return;
        }
        self.session_state.current_session_index =
            (self.session_state.current_session_index + 1) % self.session_state.sessions.len();
    }

    pub const fn prev_session(&mut self) {
        if self.session_state.sessions.is_empty() {
            return;
        }
        if self.session_state.current_session_index == 0 {
            self.session_state.current_session_index = self.session_state.sessions.len() - 1;
        } else {
            self.session_state.current_session_index -= 1;
        }
    }

    pub fn all_sessions_history(&self) -> Vec<History> {
        self.session_state
            .sessions
            .iter()
            .map(|s| s.history.clone())
            .collect()
    }

    pub fn restore_from_history(&mut self, data: Vec<History>) {
        self.session_state.sessions.clear();
        for (index, history) in data.into_iter().enumerate() {
            let mut session = Session::new();
            if let Some(last_time) = history.last() {
                session.event = last_time.event();
                if index == 0 {
                    session.scramble = Some(scramble::generate_scramble(session.event));
                }
            }
            session.history = history;
            session.history.select_last();
            self.session_state.sessions.push(session);
        }
        if self.session_state.sessions.is_empty() {
            self.session_state.sessions.push(Session::new());
        }
        self.session_state.current_session_index = 0;
        self.help_state = HelpState::default();
        self.screen = Screen::default();
        #[cfg(feature = "bluetooth")]
        {
            self.bluetooth_state = BluetoothState::default();
        }
        self.main_focus = MainFocus::History;
        self.main_stats_selection = MainStatsSelection::default();
    }
}
