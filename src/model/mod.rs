use crate::scramble::WcaEvent;
use crate::widgets::history::{History, Modifier, Time};
use crate::{model::settings::Settings, widgets::theme_selector::ThemeSelector};

#[cfg(feature = "bluetooth")]
pub mod bluetooth;
pub mod confirmation;
pub mod detailed_stats;
pub mod details;
pub mod help;
pub mod keybinds;
pub mod main_focus;
pub mod mean_details;
pub mod screen;
pub mod session;
pub mod settings;
pub mod theme_selector;

#[cfg(feature = "bluetooth")]
use bluetooth::BluetoothState;
use confirmation::Confirmation;
use help::HelpState;
use main_focus::{MainFocus, MainStatsSelection};
use screen::Screen;
use session::SessionState;
pub use session::TimerState;

pub const MAX_SESSIONS: usize = 99;

pub struct Model {
    pub(crate) session_state: SessionState,
    pub(crate) settings: Settings,
    pub(crate) help_state: HelpState,
    pub(crate) screen: Screen,
    pub(crate) theme_selector: Option<ThemeSelector>,
    pub(crate) confirmation: Option<Confirmation>,
    #[cfg(feature = "bluetooth")]
    pub(crate) bluetooth_state: BluetoothState,
    pub(crate) main_focus: MainFocus,
    pub(crate) main_stats_selection: MainStatsSelection,
}

impl Model {
    /// Creates a model with one fresh session and default UI settings.
    pub fn new() -> Self {
        Self {
            session_state: SessionState::new(),
            settings: Settings::default(),
            help_state: HelpState::default(),
            screen: Screen::default(),
            theme_selector: None,
            confirmation: None,
            #[cfg(feature = "bluetooth")]
            bluetooth_state: BluetoothState::default(),
            main_focus: MainFocus::History,
            main_stats_selection: MainStatsSelection::default(),
        }
    }

    /// Returns the active application settings.
    pub const fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Replaces the application settings used by the model.
    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    /// Returns whether WCA inspection is enabled.
    pub const fn inspection_enabled(&self) -> bool {
        self.settings.inspection()
    }

    /// Returns whether keyboard focus is on the statistics pane.
    pub const fn main_focus_is_stats(&self) -> bool {
        matches!(self.main_focus, MainFocus::Stats)
    }

    /// Switches keyboard focus between history and statistics.
    pub const fn toggle_main_focus(&mut self) {
        self.main_focus = match self.main_focus {
            MainFocus::History => MainFocus::Stats,
            MainFocus::Stats => MainFocus::History,
        };
    }

    /// Returns the selected statistics row.
    pub const fn main_stats_row(&self) -> usize {
        self.main_stats_selection.row
    }

    /// Returns the selected statistics column.
    pub const fn main_stats_col(&self) -> usize {
        self.main_stats_selection.col
    }

    /// Moves the statistics selection up by one row.
    pub const fn main_stats_select_up(&mut self) {
        self.main_stats_selection.row = self.main_stats_selection.row.saturating_sub(1);
    }

    /// Moves the statistics selection down, stopping at the last row.
    pub fn main_stats_select_down(&mut self) {
        self.main_stats_selection.row = (self.main_stats_selection.row + 1).min(5);
    }

    /// Selects the left statistics column.
    pub const fn main_stats_col_left(&mut self) {
        self.main_stats_selection.col = 0;
    }

    /// Selects the right statistics column.
    pub const fn main_stats_col_right(&mut self) {
        self.main_stats_selection.col = 1;
    }

    /// Resets the active session timer and its displayed time.
    pub fn reset_timer(&mut self) {
        self.current_session_mut().reset_timer();
    }

    /// Starts inspection for the active session.
    pub fn start_inspection(&mut self) {
        self.current_session_mut().start_inspection();
    }

    /// Starts timing a solve, or immediately records an expired inspection as DNF.
    ///
    /// Returns `true` when timing started and `false` when the attempt instead
    /// finished as a zero-duration DNF.
    pub fn start_timer(&mut self) -> bool {
        let inspection_limit_ms = self.settings.inspection_limit();
        let modifier = self
            .current_session()
            .inspection_modifier(inspection_limit_ms);
        if modifier == Modifier::DNF {
            self.finish_inspection_dnf();
            false
        } else {
            self.current_session_mut().start_timer(modifier);
            true
        }
    }

    /// Stops the active session timer without recording a solve.
    pub fn stop_timer(&mut self) {
        self.current_session_mut().stop_timer();
    }

    /// Marks a running inspection as having received its initial key press.
    pub fn pulse_timer(&mut self) {
        self.current_session_mut().pulse_timer();
    }

    /// Enables or disables WCA inspection.
    pub const fn toggle_inspection(&mut self) {
        self.settings.set_inspection(!self.settings.inspection());
    }

    /// Enables or disables the distraction-free timer view.
    pub const fn toggle_zen(&mut self) {
        self.settings.set_zen(!self.settings.zen());
    }

    /// Returns whether the distraction-free timer view is enabled.
    pub const fn zen_enabled(&self) -> bool {
        self.settings.zen()
    }

    /// Returns the active timer's elapsed or last-recorded duration.
    pub fn elapsed_ms(&self) -> u64 {
        self.current_session().elapsed_ms()
    }

    /// Returns the modifier attached to the duration currently shown while idle.
    pub fn displayed_modifier(&self) -> Modifier {
        self.current_session().last_modifier
    }

    /// Advances the active session to a newly generated scramble.
    pub fn next_scramble(&mut self) {
        self.current_session_mut().next_scramble();
    }

    /// Records a completed solve and consumes its displayed scramble.
    pub fn record_solve(&mut self, time_ms: u64) {
        let modifier = match self.current_session().timer_state {
            TimerState::Running {
                inspection_modifier,
                ..
            } => inspection_modifier,
            TimerState::Idle | TimerState::Pulsed | TimerState::Inspection { .. } => Modifier::None,
        };
        self.record_solve_with_modifier(time_ms, modifier);
    }

    /// Finishes an inspection that reached the DNF threshold.
    pub(crate) fn finish_expired_inspection(&mut self) -> bool {
        let inspection_limit_ms = self.settings.inspection_limit();
        if self
            .current_session()
            .inspection_modifier(inspection_limit_ms)
            != Modifier::DNF
        {
            return false;
        }

        self.finish_inspection_dnf();
        true
    }

    /// Records the zero-duration DNF used when inspection expires.
    fn finish_inspection_dnf(&mut self) {
        self.record_solve_with_modifier(0, Modifier::DNF);
        self.next_scramble();
    }

    /// Records a completed solve with an explicit modifier.
    fn record_solve_with_modifier(&mut self, time_ms: u64, modifier: Modifier) {
        let session = self.current_session_mut();
        let event = session.event;
        let scramble = session
            .scramble
            .take()
            .expect("active session should have a scramble");
        session.last_time_ms = time_ms;
        session.last_modifier = modifier;
        session
            .history
            .add(Time::new_with_modifier(time_ms, event, scramble, modifier));
        session.stop_timer();
    }

    /// Selects the next puzzle event and generates its scramble.
    pub fn next_event(&mut self) {
        self.current_session_mut().next_event();
    }

    /// Selects the previous puzzle event and generates its scramble.
    pub fn prev_event(&mut self) {
        self.current_session_mut().prev_event();
    }

    /// Returns the active session's timer state.
    pub fn timer_state(&self) -> TimerState {
        self.current_session().timer_state
    }

    /// Returns a mutable reference of the session's timer state
    pub fn timer_state_mut(&mut self) -> &mut TimerState {
        &mut self.current_session_mut().timer_state
    }

    /// Replaces the active session's timer state.
    pub fn set_timer_state(&mut self, timer_state: TimerState) {
        self.current_session_mut().timer_state = timer_state;
    }

    /// Replaces the time displayed while the timer is idle.
    pub fn set_last_time_ms(&mut self, ms: u64) {
        let session = self.current_session_mut();
        session.last_time_ms = ms;
        session.last_modifier = Modifier::None;
    }

    /// Returns the active session's current scramble text.
    ///
    /// # Panics
    /// Panics if the active session has not yet been prepared with a scramble.
    pub fn scramble(&self) -> &str {
        self.current_session()
            .scramble
            .as_ref()
            .expect("active session should have a scramble")
            .as_str()
    }

    /// Returns whether the current scramble came from the official WCA generator.
    ///
    /// # Panics
    /// Panics if the active session has not yet been prepared with a scramble.
    pub fn scramble_is_wca(&self) -> bool {
        self.current_session()
            .scramble
            .as_ref()
            .expect("active session should have a scramble")
            .is_wca()
    }

    /// Returns the active session's puzzle event.
    pub fn event(&self) -> WcaEvent {
        self.current_session().event
    }

    /// Returns the active session's solve history.
    pub fn history(&self) -> &History {
        &self.current_session().history
    }

    /// Returns mutable access to the active session's solve history.
    pub fn history_mut(&mut self) -> &mut History {
        &mut self.current_session_mut().history
    }
}
