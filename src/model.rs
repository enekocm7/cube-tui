use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::scramble::{self, Scramble, WcaEvent};
use crate::widgets::history::{History, Modifier};

pub const MAX_SESSIONS: usize = 99;

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
    pub scramble: Scramble,
    pub last_time_ms: u64,
    pub event: WcaEvent,
}

impl Session {
    pub fn new() -> Self {
        let event = WcaEvent::Cube3x3;
        let scramble = scramble::generate_scramble(event);
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
        self.scramble = scramble::generate_scramble(self.event);
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Settings {
    inspection: bool,
}

impl Settings {
    pub const fn set_inspection(&mut self, inspection: bool) {
        self.inspection = inspection;
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self { inspection: true }
    }
}

pub struct Model {
    sessions: Vec<Session>,
    settings: Settings,
    current_session_index: usize,
    show_help: bool,
    show_details: bool,
    details_modifier_index: usize,
}

impl Model {
    pub fn new() -> Self {
        Self {
            sessions: vec![Session::new()],
            settings: Settings::default(),
            current_session_index: 0,
            show_help: false,
            show_details: false,
            details_modifier_index: 0,
        }
    }

    pub const fn settings(&self) -> &Settings {
        &self.settings
    }

    pub const fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    pub const fn current_session_index(&self) -> usize {
        self.current_session_index
    }

    pub const fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub const fn is_at_max_sessions(&self) -> bool {
        self.sessions.len() >= MAX_SESSIONS
    }

    pub fn get_current_session(&self) -> &Session {
        &self.sessions[self.current_session_index]
    }

    pub fn get_current_session_mut(&mut self) -> &mut Session {
        &mut self.sessions[self.current_session_index]
    }

    pub fn add_session(&mut self) -> bool {
        if self.is_at_max_sessions() {
            return false;
        }
        self.sessions.push(Session::new());
        self.current_session_index = self.sessions.len() - 1;
        true
    }

    pub const fn next_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        self.current_session_index = (self.current_session_index + 1) % self.sessions.len();
    }

    pub const fn prev_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        if self.current_session_index == 0 {
            self.current_session_index = self.sessions.len() - 1;
        } else {
            self.current_session_index -= 1;
        }
    }

    pub fn reset_timer(&mut self) {
        self.get_current_session_mut().reset_timer();
    }

    pub fn start_inspection(&mut self) {
        self.get_current_session_mut().start_inspection();
    }

    pub fn start_timer(&mut self) {
        self.get_current_session_mut().start_timer();
    }

    pub fn stop_timer(&mut self) {
        self.get_current_session_mut().stop_timer();
    }

    pub fn pulse_timer(&mut self) {
        self.get_current_session_mut().pulse_timer();
    }

    pub const fn toggle_inspection(&mut self) {
        self.settings.set_inspection(!self.settings.inspection);
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.get_current_session().elapsed_ms()
    }

    pub fn next_scramble(&mut self) {
        self.get_current_session_mut().next_scramble();
    }

    pub fn next_event(&mut self) {
        self.get_current_session_mut().next_event();
    }

    pub fn prev_event(&mut self) {
        self.get_current_session_mut().prev_event();
    }

    pub fn timer_state(&self) -> TimerState {
        self.get_current_session().timer_state
    }

    pub fn set_timer_state(&mut self, timer_state: TimerState) {
        self.get_current_session_mut().timer_state = timer_state;
    }

    pub fn set_last_time_ms(&mut self, ms: u64) {
        self.get_current_session_mut().last_time_ms = ms;
    }

    pub fn scramble(&self) -> &Scramble {
        &self.get_current_session().scramble
    }

    pub fn event(&self) -> WcaEvent {
        self.get_current_session().event
    }

    pub fn history(&self) -> &History {
        &self.get_current_session().history
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.get_current_session_mut().history
    }

    pub const fn show_help(&self) -> bool {
        self.show_help
    }

    pub const fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub const fn show_details(&self) -> bool {
        self.show_details
    }

    pub fn open_details(&mut self) {
        self.show_details = true;
        self.details_modifier_index = match self.history().selected_time().map(|t| t.modifier()) {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
    }

    pub const fn close_details(&mut self) {
        self.show_details = false;
    }

    pub fn next_details_modifier(&mut self) {
        self.details_modifier_index = (self.details_modifier_index + 1).min(1);
    }

    pub fn prev_details_modifier(&mut self) {
        self.details_modifier_index = self.details_modifier_index.saturating_sub(1);
    }

    pub const fn selected_details_modifier_index(&self) -> usize {
        self.details_modifier_index
    }

    pub const fn selected_details_modifier(&self) -> Modifier {
        if self.details_modifier_index == 0 {
            Modifier::PlusTwo
        } else {
            Modifier::DNF
        }
    }

    pub const fn inspection_enabled(&self) -> bool {
        self.settings.inspection
    }

    pub fn all_sessions_history(&self) -> Vec<History> {
        self.sessions.iter().map(|s| s.history.clone()).collect()
    }

    pub fn restore_from_history(&mut self, data: Vec<History>) {
        self.sessions.clear();
        for history in data {
            let mut session = Session::new();
            if let Some(last_time) = history.last() {
                session.event = last_time.event();
                session.scramble = scramble::generate_scramble(session.event);
            }
            session.history = history;
            self.sessions.push(session);
        }
        if self.sessions.is_empty() {
            self.sessions.push(Session::new());
        }
        self.current_session_index = 0;
        self.show_details = false;
        self.details_modifier_index = 0;
    }
}
