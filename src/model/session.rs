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
    next_scramble: Option<(WcaEvent, flume::Receiver<Scramble>)>,
    pub last_time_ms: u64,
    pub event: WcaEvent,
}

impl Session {
    /// Creates an empty session without generating or prefetching a scramble.
    ///
    /// This constructor is used for inactive and restored sessions so they do
    /// not retain background work before the user selects them.
    pub fn new() -> Self {
        Self {
            timer_state: TimerState::Idle,
            history: History::new(),
            scramble: None,
            next_scramble: None,
            last_time_ms: 0,
            event: WcaEvent::Cube3x3,
        }
    }

    /// Creates a session ready for display and starts its next prefetch.
    pub fn new_with_scramble() -> Self {
        let mut session = Self::new();
        session.next_scramble();
        session
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

    /// Makes the next scramble current and starts a one-shot replacement prefetch.
    ///
    /// A matching in-flight prefetch is consumed, which may briefly block until
    /// generation completes. Missing, failed, or stale-event prefetches fall
    /// back to direct generation.
    pub fn next_scramble(&mut self) {
        // A prefetch belongs to the event that requested it. Discard it when
        // switching events, and wait for an in-flight result instead of starting
        // a second expensive generator for the same scramble.
        self.scramble = Some(
            self.next_scramble
                .take()
                .filter(|(event, _)| *event == self.event)
                .and_then(|(_, rx)| rx.recv().ok())
                .unwrap_or_else(|| generate_scramble(self.event)),
        );
        self.prefetch_scramble();
    }

    pub fn next_event(&mut self) {
        self.event = self.event.next();
        self.next_scramble();
    }

    pub fn prev_event(&mut self) {
        self.event = self.event.prev();
        self.next_scramble();
    }

    /// Starts one background job that generates a single scramble.
    ///
    /// The receiver is tagged with the current event so a later event change
    /// cannot accidentally display a stale scramble. The worker exits after
    /// sending one result, or skips generation if the session was dropped.
    fn prefetch_scramble(&mut self) {
        let (tx, rx) = flume::bounded(1);
        let event = self.event;
        self.next_scramble = Some((event, rx));
        // This job generates exactly one result and then releases its worker.
        // Inactive sessions retain only a buffered scramble, not a running task.
        runtime().spawn_blocking(move || {
            if !tx.is_disconnected() {
                let _ = tx.send(generate_scramble(event));
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
            sessions: vec![Session::new_with_scramble()],
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

    /// Iterates over session histories by reference in session order.
    ///
    /// Borrowing avoids cloning every solve during persistence and export.
    pub fn all_sessions_history(&self) -> impl Iterator<Item = &History> {
        self.session_state.sessions.iter().map(|s| &s.history)
    }

    /// Replaces all sessions with restored histories and activates the first one.
    ///
    /// Each session restores its last event and selection. Only the active
    /// session receives a current scramble and prefetch; other sessions remain
    /// lazy until navigation selects them.
    pub fn restore_from_history(&mut self, data: impl IntoIterator<Item = History>) {
        self.session_state.sessions.clear();
        let data = data.into_iter();
        self.session_state.sessions.reserve(data.size_hint().0);
        for history in data {
            let mut session = Session::new();
            if let Some(last_time) = history.last() {
                session.event = last_time.event();
            }
            session.history = history;
            session.history.select_last();
            self.session_state.sessions.push(session);
        }
        if self.session_state.sessions.is_empty() {
            self.session_state.sessions.push(Session::new());
        }
        self.session_state.current_session_index = 0;
        self.next_scramble();
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
    use std::time::Duration;

    #[test]
    fn unused_sessions_do_not_start_prefetch_jobs() {
        let session = Session::new();
        assert!(session.scramble.is_none());
        assert!(session.next_scramble.is_none());
    }

    #[test]
    fn prefetch_produces_one_result_and_finishes() {
        let mut session = Session::new();
        // FTO uses the built-in generator even with WCA support enabled.
        session.event = WcaEvent::Fto;
        session.prefetch_scramble();
        let (event, rx) = session.next_scramble.take().unwrap();
        assert_eq!(event, WcaEvent::Fto);
        assert_ne!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap().as_str(),
            ""
        );
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(5)),
            Err(flume::RecvTimeoutError::Disconnected)
        ));
    }

    #[test]
    fn next_scramble_consumes_prefetch_and_replenishes_it() {
        let mut session = Session::new();
        session.event = WcaEvent::Fto;
        let (tx, rx) = flume::bounded(1);
        tx.send(Scramble::new("prefetched scramble")).unwrap();
        session.next_scramble = Some((session.event, rx));

        session.next_scramble();

        assert_eq!(
            session.scramble.as_ref().unwrap().as_str(),
            "prefetched scramble"
        );
        assert_eq!(session.next_scramble.as_ref().unwrap().0, WcaEvent::Fto);
    }

    #[test]
    fn changing_events_discards_stale_prefetch() {
        let mut session = Session::new();
        session.event = WcaEvent::Pyraminx;
        let (tx, rx) = flume::bounded(1);
        tx.send(Scramble::new("stale scramble")).unwrap();
        session.next_scramble = Some((session.event, rx));

        session.next_event();

        assert_eq!(session.event, WcaEvent::Fto);
        assert_ne!(
            session.scramble.as_ref().unwrap().as_str(),
            "stale scramble"
        );
        assert_eq!(session.next_scramble.as_ref().unwrap().0, WcaEvent::Fto);
    }

    #[test]
    fn disconnected_prefetch_falls_back_and_replenishes() {
        let mut session = Session::new();
        session.event = WcaEvent::Fto;
        let (tx, rx) = flume::bounded(1);
        session.next_scramble = Some((session.event, rx));
        drop(tx);

        session.next_scramble();

        assert_ne!(session.scramble.as_ref().unwrap().as_str(), "");
        assert_eq!(session.next_scramble.as_ref().unwrap().0, WcaEvent::Fto);
    }

    #[test]
    fn restoring_sessions_only_prepares_the_active_session() {
        let mut first_history = History::new();
        first_history.add_ms(1000, WcaEvent::Cube2x2, "first solve");
        first_history.add_ms(2000, WcaEvent::Fto, "last solve");
        let mut second_history = History::new();
        second_history.add_ms(3000, WcaEvent::Fto, "other session solve");
        let mut model = Model::new();

        model.restore_from_history([first_history, second_history]);

        assert_eq!(model.current_session_index(), 0);
        assert_eq!(model.session_count(), 2);
        let active = model.current_session();
        assert_eq!(active.event, WcaEvent::Fto);
        assert_ne!(active.scramble.as_ref().unwrap().as_str(), "");
        assert_eq!(active.next_scramble.as_ref().unwrap().0, WcaEvent::Fto);
        assert_eq!(active.history.selected_time().unwrap().raw_ms(), 2000);
        let inactive = &model.session_state.sessions[1];
        assert_eq!(inactive.event, WcaEvent::Fto);
        assert!(inactive.scramble.is_none());
        assert!(inactive.next_scramble.is_none());
        assert_eq!(inactive.history.selected_time().unwrap().raw_ms(), 3000);

        // Navigation activates a restored session before its first render/solve.
        crate::handler::update(&mut model, crate::msg::Msg::NextSession);
        assert_eq!(model.current_session_index(), 1);
        assert_ne!(model.scramble(), "");
        assert_eq!(
            model.current_session().next_scramble.as_ref().unwrap().0,
            WcaEvent::Fto
        );
    }
}
