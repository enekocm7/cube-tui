use super::MAX_SESSIONS;
#[cfg(feature = "bluetooth")]
use crate::model::bluetooth::BluetoothState;
use crate::model::help::HelpState;
use crate::model::main_focus::{MainFocus, MainStatsSelection};
use crate::model::screen::Screen;
use crate::model::Model;
use crate::scramble::{generate_scramble, Scramble, WcaEvent};
use crate::utils::runtime::runtime;
use crate::widgets::history::History;
use std::sync::{Arc, Mutex};
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
    next_scramble: Arc<Mutex<Option<Scramble>>>,
    next_scramble_tx: flume::Sender<Scramble>,
    next_scramble_rx: flume::Receiver<Scramble>,
    pub last_time_ms: u64,
    pub event: WcaEvent,
}

impl Session {
    pub fn new() -> Self {
        let (tx, rx) = flume::bounded(1);
        Self {
            timer_state: TimerState::Idle,
            history: History::new(),
            scramble: None,
            next_scramble: Arc::new(Mutex::new(None)),
            next_scramble_tx: tx,
            next_scramble_rx: rx,
            last_time_ms: 0,
            event: WcaEvent::Cube3x3,
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
        let mut next = self.next_scramble.lock().unwrap();
        if next.is_some() {
            self.scramble = next.take();
        } else {
            let event = self.event;
            self.scramble = Some(generate_scramble(event));
        }
    }

    pub fn next_event(&mut self) {
        self.event = self.event.next();
        self.next_scramble();
    }

    pub fn prev_event(&mut self) {
        self.event = self.event.prev();
        self.next_scramble();
    }

    pub fn spawn_scramble_receiver(&mut self) {
        let next_scramble = Arc::clone(&self.next_scramble);
        let rx = self.next_scramble_rx.clone();
        runtime().spawn_blocking(move || {
            while let Ok(scramble) = rx.recv() {
                *next_scramble.lock().unwrap() = Some(scramble);
            }
        });
    }

    pub fn spawn_scramble_generator(&mut self) {
        let tx = self.next_scramble_tx.clone();
        let next_scramble = Arc::clone(&self.next_scramble);
        let event = self.event;
        runtime().spawn_blocking(move || {
            loop {
                if next_scramble.lock().unwrap().is_none() {
                    let scramble = generate_scramble(event);
                    if tx.send(scramble).is_err() {
                        break;
                    }
                }
            }
        });
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
                let event = last_time.event();
                if index == 0 {
                    session.scramble = Some(generate_scramble(event));
                }
                session.spawn_scramble_generator();
                session.spawn_scramble_receiver();
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
