use crate::model::settings::Settings;
use crate::scramble::WcaEvent;
use crate::widgets::history::History;

#[cfg(feature = "bluetooth")]
pub mod bluetooth;
pub mod confirmation;
pub mod detailed_stats;
pub mod details;
pub mod help;
pub mod main_focus;
pub mod mean_details;
pub mod screen;
pub mod session;
pub mod settings;

#[cfg(feature = "bluetooth")]
use bluetooth::BluetoothState;
use confirmation::Confirmation;
use help::HelpState;
use main_focus::{MainFocus, MainStatsSelection};
use screen::Screen;
use session::SessionState;
pub use session::{InspectionState, TimerState};

pub const MAX_SESSIONS: usize = 99;

pub struct Model {
    pub(crate) session_state: SessionState,
    pub(crate) settings: Settings,
    pub(crate) help_state: HelpState,
    pub(crate) screen: Screen,
    pub(crate) confirmation: Option<Confirmation>,
    #[cfg(feature = "bluetooth")]
    pub(crate) bluetooth_state: BluetoothState,
    pub(crate) main_focus: MainFocus,
    pub(crate) main_stats_selection: MainStatsSelection,
}

impl Model {
    pub fn new() -> Self {
        Self {
            session_state: SessionState::new(),
            settings: Settings::default(),
            help_state: HelpState::default(),
            screen: Screen::default(),
            confirmation: None,
            #[cfg(feature = "bluetooth")]
            bluetooth_state: BluetoothState::default(),
            main_focus: MainFocus::History,
            main_stats_selection: MainStatsSelection::default(),
        }
    }

    pub const fn settings(&self) -> &Settings {
        &self.settings
    }

    pub const fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    pub const fn inspection_enabled(&self) -> bool {
        self.settings.inspection()
    }

    pub const fn main_focus_is_stats(&self) -> bool {
        matches!(self.main_focus, MainFocus::Stats)
    }

    pub const fn toggle_main_focus(&mut self) {
        self.main_focus = match self.main_focus {
            MainFocus::History => MainFocus::Stats,
            MainFocus::Stats => MainFocus::History,
        };
    }

    pub const fn main_stats_row(&self) -> usize {
        self.main_stats_selection.row
    }

    pub const fn main_stats_col(&self) -> usize {
        self.main_stats_selection.col
    }

    pub const fn main_stats_select_up(&mut self) {
        self.main_stats_selection.row = self.main_stats_selection.row.saturating_sub(1);
    }

    pub fn main_stats_select_down(&mut self) {
        self.main_stats_selection.row = (self.main_stats_selection.row + 1).min(2);
    }

    pub const fn main_stats_col_left(&mut self) {
        self.main_stats_selection.col = 0;
    }

    pub const fn main_stats_col_right(&mut self) {
        self.main_stats_selection.col = 1;
    }

    pub fn reset_timer(&mut self) {
        self.current_session_mut().reset_timer();
    }

    pub fn start_inspection(&mut self) {
        self.current_session_mut().start_inspection();
    }

    pub fn start_timer(&mut self) {
        self.current_session_mut().start_timer();
    }

    pub fn stop_timer(&mut self) {
        self.current_session_mut().stop_timer();
    }

    pub fn pulse_timer(&mut self) {
        self.current_session_mut().pulse_timer();
    }

    pub const fn toggle_inspection(&mut self) {
        self.settings.set_inspection(!self.settings.inspection());
    }

    pub const fn toggle_zen(&mut self) {
        self.settings.set_zen(!self.settings.zen());
    }

    pub const fn zen_enabled(&self) -> bool {
        self.settings.zen()
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.current_session().elapsed_ms()
    }

    pub fn next_scramble(&mut self) {
        self.current_session_mut().next_scramble();
    }

    pub fn record_solve(&mut self, time_ms: u64) {
        let session = self.current_session_mut();
        let event = session.event;
        let scramble = session
            .scramble
            .take()
            .expect("active session should have a scramble");
        session.last_time_ms = time_ms;
        session.history.add_ms(time_ms, event, scramble);
        session.stop_timer();
    }

    pub fn next_event(&mut self) {
        self.current_session_mut().next_event();
    }

    pub fn prev_event(&mut self) {
        self.current_session_mut().prev_event();
    }

    pub fn timer_state(&self) -> TimerState {
        self.current_session().timer_state
    }

    pub fn set_timer_state(&mut self, timer_state: TimerState) {
        self.current_session_mut().timer_state = timer_state;
    }

    pub fn set_last_time_ms(&mut self, ms: u64) {
        self.current_session_mut().last_time_ms = ms;
    }

    pub fn scramble(&self) -> &str {
        self.current_session()
            .scramble
            .as_ref()
            .expect("active session should have a scramble")
            .as_str()
    }

    pub fn scramble_is_wca(&self) -> bool {
        self.current_session()
            .scramble
            .as_ref()
            .expect("active session should have a scramble")
            .is_wca()
    }

    pub fn event(&self) -> WcaEvent {
        self.current_session().event
    }

    pub fn history(&self) -> &History {
        &self.current_session().history
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.current_session_mut().history
    }
}
