use std::time::Instant;

use crate::scramble;
use crate::widgets::history::History;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimerState {
    Idle,
    Inspection(InspectionState),
    Running(Instant),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InspectionState {
    Pulsed(Instant),
    Running(Instant),
}

pub struct Model {
    pub timer_state: TimerState,
    pub history: History,
    pub scramble: &'static str,
    pub last_time_ms: u64,
    scramble_index: usize,
}

impl Model {
    pub const fn new() -> Self {
        Self {
            timer_state: TimerState::Idle,
            history: History::new(),
            scramble: scramble::generate_scramble(0),
            last_time_ms: 0,
            scramble_index: 0,
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
            TimerState::Idle => self.last_time_ms,
        }
    }

    pub const fn next_scramble(&mut self) {
        self.scramble_index = self.scramble_index.wrapping_add(1);
        self.scramble = scramble::generate_scramble(self.scramble_index);
    }
}
