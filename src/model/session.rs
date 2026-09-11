use super::MAX_SESSIONS;
use crate::model::Model;
#[cfg(feature = "bluetooth")]
use crate::model::bluetooth::BluetoothState;
use crate::model::help::HelpState;
use crate::model::main_focus::{MainFocus, MainStatsSelection};
use crate::model::screen::Screen;
use crate::scramble::{Scramble, WcaEvent, generate_scramble};
use crate::utils::runtime::runtime;
use crate::widgets::history::History;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

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
    scramble_workers_cancelled: Arc<AtomicBool>,
    pub last_time_ms: u64,
    pub event: WcaEvent,
}

impl Session {
    /// Creates a session and starts its background scramble workers.
    ///
    /// The current scramble remains empty until requested, while the generator
    /// prepares the next scramble in the background.
    pub fn new() -> Self {
        let (tx, rx) = flume::bounded(1);
        let mut session = Self {
            timer_state: TimerState::Idle,
            history: History::new(),
            scramble: None,
            next_scramble: Arc::new(Mutex::new(None)),
            next_scramble_tx: tx,
            next_scramble_rx: rx,
            scramble_workers_cancelled: Arc::new(AtomicBool::new(false)),
            last_time_ms: 0,
            event: WcaEvent::Cube3x3,
        };
        session.spawn_scramble_generator();
        session.spawn_scramble_receiver();
        session
    }

    /// Creates a scramble-ready session and starts its background workers.
    pub fn new_with_scramble() -> Self {
        let (tx, rx) = flume::bounded(1);
        let mut session = Self {
            timer_state: TimerState::Idle,
            history: History::new(),
            scramble: Some(generate_scramble(WcaEvent::Cube3x3)),
            next_scramble: Arc::new(Mutex::new(None)),
            next_scramble_tx: tx,
            next_scramble_rx: rx,
            scramble_workers_cancelled: Arc::new(AtomicBool::new(false)),
            last_time_ms: 0,
            event: WcaEvent::Cube3x3,
        };
        session.spawn_scramble_generator();
        session.spawn_scramble_receiver();
        session
    }

    /// Clears the timer state and the previously displayed duration.
    pub const fn reset_timer(&mut self) {
        self.timer_state = TimerState::Idle;
        self.last_time_ms = 0;
    }

    /// Begins the inspection countdown at the current instant.
    pub fn start_inspection(&mut self) {
        self.timer_state = TimerState::Inspection(InspectionState::Running(Instant::now()));
    }

    /// Begins measuring a solve at the current instant.
    pub fn start_timer(&mut self) {
        self.timer_state = TimerState::Running(Instant::now());
    }

    /// Returns the timer to idle while retaining the displayed duration.
    pub const fn stop_timer(&mut self) {
        self.timer_state = TimerState::Idle;
    }

    /// Marks a running inspection as pulsed without changing its start time.
    pub const fn pulse_timer(&mut self) {
        if let TimerState::Inspection(InspectionState::Running(start)) = self.timer_state {
            self.timer_state = TimerState::Inspection(InspectionState::Pulsed(start));
        }
    }

    /// Returns the elapsed duration appropriate to the current timer state.
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

    /// Makes the background-generated scramble current when one is ready.
    ///
    /// This never waits for the background worker. If it has not produced a
    /// scramble yet, generation falls back to the calling thread.
    pub fn next_scramble(&mut self) {
        let mut next = self.next_scramble.lock().unwrap();
        if next.is_some() {
            self.scramble = next.take();
        } else {
            let event = self.event;
            self.scramble = Some(generate_scramble(event));
        }
    }

    /// Advances to the next event and prepares its first scramble.
    pub fn next_event(&mut self) {
        self.event = self.event.next();
        self.respawn_scramble_generators();
        self.next_scramble();
    }

    /// Moves to the previous event and prepares its first scramble.
    pub fn prev_event(&mut self) {
        self.event = self.event.prev();
        self.respawn_scramble_generators();
        self.next_scramble();
    }

    /// Starts the worker that transfers generated scrambles into the ready slot.
    ///
    /// The worker blocks on the channel and replaces the slot whenever the
    /// generator sends a new scramble.
    pub fn spawn_scramble_receiver(&mut self) {
        let next_scramble = Arc::clone(&self.next_scramble);
        let rx = self.next_scramble_rx.clone();
        let cancelled = Arc::clone(&self.scramble_workers_cancelled);
        runtime().spawn_blocking(move || {
            while let Ok(scramble) = rx.recv() {
                if cancelled.load(Ordering::Acquire) {
                    break;
                }
                *next_scramble.lock().unwrap() = Some(scramble);
            }
        });
    }

    /// Starts the persistent background scramble generator for this session.
    ///
    /// It generates for the session's current event whenever the ready slot is
    /// empty and sends the result to the receiver worker.
    pub fn spawn_scramble_generator(&mut self) {
        let tx = self.next_scramble_tx.clone();
        let next_scramble = Arc::clone(&self.next_scramble);
        let cancelled = Arc::clone(&self.scramble_workers_cancelled);
        let event = self.event;
        runtime().spawn_blocking(move || {
            while !cancelled.load(Ordering::Acquire) {
                if next_scramble.lock().unwrap().is_none() {
                    let scramble = generate_scramble(event);
                    if cancelled.load(Ordering::Acquire) || tx.send(scramble).is_err() {
                        break;
                    }
                } else {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        });
    }

    /// Stops the current scramble workers and starts replacements for the event.
    pub fn respawn_scramble_generators(&mut self) {
        self.scramble_workers_cancelled
            .store(true, Ordering::Release);

        let (tx, rx) = flume::bounded(1);
        self.next_scramble = Arc::new(Mutex::new(None));
        self.next_scramble_tx = tx;
        self.next_scramble_rx = rx;
        self.scramble_workers_cancelled = Arc::new(AtomicBool::new(false));
        self.spawn_scramble_generator();
        self.spawn_scramble_receiver();
    }
}

impl Drop for Session {
    /// Signals both background workers to stop when their session is removed.
    fn drop(&mut self) {
        self.scramble_workers_cancelled
            .store(true, Ordering::Release);
    }
}

pub struct SessionState {
    pub sessions: Vec<Session>,
    pub current_session_index: usize,
}

impl SessionState {
    /// Creates state containing one active, scramble-ready session.
    pub fn new() -> Self {
        Self {
            sessions: vec![Session::new_with_scramble()],
            current_session_index: 0,
        }
    }
}

impl Default for SessionState {
    /// Creates the default one-session state.
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Returns the zero-based index of the active session.
    pub const fn current_session_index(&self) -> usize {
        self.session_state.current_session_index
    }

    /// Returns the number of available sessions.
    pub const fn session_count(&self) -> usize {
        self.session_state.sessions.len()
    }

    /// Returns whether another session would exceed [`MAX_SESSIONS`].
    pub const fn is_at_max_sessions(&self) -> bool {
        self.session_state.sessions.len() >= MAX_SESSIONS
    }

    /// Returns the active session.
    pub fn current_session(&self) -> &Session {
        &self.session_state.sessions[self.session_state.current_session_index]
    }

    /// Returns mutable access to the active session.
    pub fn current_session_mut(&mut self) -> &mut Session {
        &mut self.session_state.sessions[self.session_state.current_session_index]
    }

    /// Appends and selects an empty session, returning `false` at the limit.
    pub fn add_session(&mut self) -> bool {
        if self.is_at_max_sessions() {
            return false;
        }
        self.session_state.sessions.push(Session::new());
        self.session_state.current_session_index = self.session_state.sessions.len() - 1;
        true
    }

    /// Deletes the active session, returning `false` when it is the only one.
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

    /// Selects the next session, wrapping at the end.
    pub const fn next_session(&mut self) {
        if self.session_state.sessions.is_empty() {
            return;
        }
        self.session_state.current_session_index =
            (self.session_state.current_session_index + 1) % self.session_state.sessions.len();
    }

    /// Selects the previous session, wrapping at the beginning.
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

    /// Iterates over session histories by reference in session order.
    ///
    /// Borrowing avoids cloning every solve during persistence and export.
    pub fn all_sessions_history(&self) -> impl Iterator<Item = &History> {
        self.session_state.sessions.iter().map(|s| &s.history)
    }

    /// Replaces all sessions with restored histories and activates the first one.
    ///
    /// Each session restores its last event and selection and starts background
    /// workers for that event. The active session also receives a current
    /// scramble immediately.
    pub fn restore_from_history(&mut self, data: impl IntoIterator<Item = History>) {
        self.session_state.sessions.clear();
        let data = data.into_iter();
        self.session_state.sessions.reserve(data.size_hint().0);
        for (index, history) in data.enumerate() {
            let mut session = Session::new();
            if let Some(last_time) = history.last() {
                let event = last_time.event();
                session.event = event;
                if index == 0 {
                    session.scramble = Some(generate_scramble(event));
                }
                session.respawn_scramble_generators();
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a session whose workers can be started independently by a test.
    fn session_without_workers(event: WcaEvent) -> Session {
        let (tx, rx) = flume::bounded(1);
        Session {
            timer_state: TimerState::Idle,
            history: History::new(),
            scramble: None,
            next_scramble: Arc::new(Mutex::new(None)),
            next_scramble_tx: tx,
            next_scramble_rx: rx,
            scramble_workers_cancelled: Arc::new(AtomicBool::new(false)),
            last_time_ms: 0,
            event,
        }
    }

    /// Waits until the receiver worker places a scramble in the ready slot.
    fn wait_for_ready_scramble(session: &Session) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while session.next_scramble.lock().unwrap().is_none() {
            assert!(
                Instant::now() < deadline,
                "background receiver did not fill the ready slot"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Verifies that generation runs on the background runtime and fills its channel.
    #[test]
    fn background_generator_produces_a_scramble() {
        let mut session = session_without_workers(WcaEvent::Fto);
        session.spawn_scramble_generator();

        let scramble = session
            .next_scramble_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("background generator should produce a scramble");

        assert_ne!(scramble.as_str(), "");
    }

    /// Verifies that the receiver prepares a scramble for immediate consumption.
    #[test]
    fn receiver_prepares_scramble_for_next_scramble() {
        let mut session = session_without_workers(WcaEvent::Fto);
        session.spawn_scramble_receiver();
        session
            .next_scramble_tx
            .send(Scramble::new("prepared scramble"))
            .unwrap();
        wait_for_ready_scramble(&session);

        session.next_scramble();

        assert_eq!(
            session.scramble.as_ref().unwrap().as_str(),
            "prepared scramble"
        );
        assert!(session.next_scramble.lock().unwrap().is_none());
    }

    /// Verifies that an event change retires the old workers and their result slot.
    #[test]
    fn changing_events_respawns_scramble_workers() {
        let mut session = session_without_workers(WcaEvent::Pyraminx);
        let old_cancelled = Arc::clone(&session.scramble_workers_cancelled);
        let old_next_scramble = Arc::clone(&session.next_scramble);
        *old_next_scramble.lock().unwrap() = Some(Scramble::new("stale scramble"));

        session.next_event();

        assert_eq!(session.event, WcaEvent::Fto);
        assert!(old_cancelled.load(Ordering::Acquire));
        assert!(!Arc::ptr_eq(&old_next_scramble, &session.next_scramble));
        assert_ne!(
            session.scramble.as_ref().unwrap().as_str(),
            "stale scramble"
        );
    }

    /// Verifies that WCA generation works from a background runtime thread.
    #[cfg(feature = "wca-scrambles")]
    #[test]
    fn background_generator_produces_a_wca_scramble() {
        let mut session = session_without_workers(WcaEvent::Cube4x4);
        session.spawn_scramble_generator();

        let scramble = session
            .next_scramble_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("background generator should produce a WCA scramble");

        assert_ne!(scramble.as_str(), "");
        assert!(scramble.is_wca());
    }
}
