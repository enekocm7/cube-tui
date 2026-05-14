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
