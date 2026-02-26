use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::scramble::{self, Scramble, WcaEvent};
use crate::widgets::history::{History, Modifier, Time};

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

struct SessionState {
    sessions: Vec<Session>,
    current_session_index: usize,
}

impl SessionState {
    fn new() -> Self {
        Self {
            sessions: vec![Session::new()],
            current_session_index: 0,
        }
    }
}

#[derive(Default)]
struct HelpState {
    show: bool,
    scroll: u16,
    max_scroll: u16,
}

#[derive(Default)]
struct DetailsState {
    show: bool,
    modifier_index: usize,
    return_to_mean_detail: Option<MeanDetailReturnState>,
}

#[derive(Copy, Clone)]
struct MeanDetailReturnState {
    row: usize,
    col: usize,
    selected_index: usize,
}

#[derive(Default)]
struct DetailedStatsState {
    show: bool,
    row: usize,
    col: usize,
    show_mean_detail: bool,
    mean_detail_selected_index: usize,
}

pub struct Model {
    session_state: SessionState,
    settings: Settings,
    help_state: HelpState,
    details_state: DetailsState,
    detailed_stats_state: DetailedStatsState,
}

impl Model {
    pub fn new() -> Self {
        Self {
            session_state: SessionState::new(),
            settings: Settings::default(),
            help_state: HelpState::default(),
            details_state: DetailsState::default(),
            detailed_stats_state: DetailedStatsState::default(),
        }
    }

    pub const fn settings(&self) -> &Settings {
        &self.settings
    }

    pub const fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    pub const fn current_session_index(&self) -> usize {
        self.session_state.current_session_index
    }

    pub const fn session_count(&self) -> usize {
        self.session_state.sessions.len()
    }

    pub const fn is_at_max_sessions(&self) -> bool {
        self.session_state.sessions.len() >= MAX_SESSIONS
    }

    pub fn get_current_session(&self) -> &Session {
        &self.session_state.sessions[self.session_state.current_session_index]
    }

    pub fn get_current_session_mut(&mut self) -> &mut Session {
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

        self.details_state = DetailsState::default();
        self.detailed_stats_state = DetailedStatsState::default();

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

    pub const fn inspection_enabled(&self) -> bool {
        self.settings.inspection
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
        for history in data {
            let mut session = Session::new();
            if let Some(last_time) = history.last() {
                session.event = last_time.event();
                session.scramble = scramble::generate_scramble(session.event);
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
        self.details_state = DetailsState::default();
        self.detailed_stats_state = DetailedStatsState::default();
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
        self.help_state.show
    }

    pub const fn toggle_help(&mut self) {
        self.help_state.show = !self.help_state.show;
        if self.help_state.show {
            self.help_state.scroll = 0;
        }
    }

    pub const fn help_scroll(&self) -> u16 {
        self.help_state.scroll
    }

    pub fn set_help_max_scroll(&mut self, max_scroll: u16) {
        self.help_state.max_scroll = max_scroll;
        self.help_state.scroll = self.help_state.scroll.min(self.help_state.max_scroll);
    }

    pub const fn scroll_help_up(&mut self) {
        self.help_state.scroll = self.help_state.scroll.saturating_sub(1);
    }

    pub fn scroll_help_down(&mut self) {
        self.help_state.scroll = self
            .help_state
            .scroll
            .saturating_add(1)
            .min(self.help_state.max_scroll);
    }

    pub const fn show_details(&self) -> bool {
        self.details_state.show
    }

    pub fn open_details(&mut self) {
        self.details_state.show = true;
        self.details_state.return_to_mean_detail = None;
        self.details_state.modifier_index = match self.history().selected_time().map(Time::modifier)
        {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
    }

    pub const fn close_details(&mut self) {
        self.details_state.show = false;
        self.details_state.return_to_mean_detail = None;
    }

    pub const fn can_return_to_mean_detail(&self) -> bool {
        self.details_state.return_to_mean_detail.is_some()
    }

    pub const fn return_to_mean_detail(&mut self) -> bool {
        let Some(return_state) = self.details_state.return_to_mean_detail.take() else {
            return false;
        };

        self.details_state.show = false;
        self.detailed_stats_state.show = true;
        self.detailed_stats_state.row = return_state.row;
        self.detailed_stats_state.col = return_state.col;
        self.detailed_stats_state.show_mean_detail = true;
        self.detailed_stats_state.mean_detail_selected_index = return_state.selected_index;
        true
    }

    pub fn next_details_modifier(&mut self) {
        self.details_state.modifier_index = (self.details_state.modifier_index + 1).min(1);
    }

    pub const fn prev_details_modifier(&mut self) {
        self.details_state.modifier_index = self.details_state.modifier_index.saturating_sub(1);
    }

    pub const fn selected_details_modifier_index(&self) -> usize {
        self.details_state.modifier_index
    }

    pub const fn selected_details_modifier(&self) -> Modifier {
        if self.details_state.modifier_index == 0 {
            Modifier::PlusTwo
        } else {
            Modifier::DNF
        }
    }

    pub fn details_nav_prev(&mut self) {
        self.history_mut().select_previous();
        self.details_state.modifier_index = match self.history().selected_time().map(Time::modifier)
        {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
    }

    pub fn details_nav_next(&mut self) {
        self.history_mut().select_next();
        self.details_state.modifier_index = match self.history().selected_time().map(Time::modifier)
        {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
    }

    pub const fn show_detailed_stats(&self) -> bool {
        self.detailed_stats_state.show
    }

    pub fn open_detailed_stats(&mut self) {
        if self.history().is_empty() {
            return;
        }
        self.detailed_stats_state.show = true;
        self.detailed_stats_state.row = self.history().len().saturating_sub(1);
        self.detailed_stats_state.col = 0;
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
    }

    pub const fn close_detailed_stats(&mut self) {
        self.detailed_stats_state.show = false;
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
    }

    pub const fn detailed_stats_row(&self) -> usize {
        self.detailed_stats_state.row
    }

    pub const fn detailed_stats_col(&self) -> usize {
        self.detailed_stats_state.col
    }

    pub const fn detailed_stats_select_up(&mut self) {
        self.detailed_stats_state.row = self.detailed_stats_state.row.saturating_sub(1);
    }

    pub fn detailed_stats_select_down(&mut self) {
        let max = self.history().len().saturating_sub(1);
        self.detailed_stats_state.row = (self.detailed_stats_state.row + 1).min(max);
    }

    pub const fn detailed_stats_col_left(&mut self) {
        self.detailed_stats_state.col = 0;
    }

    pub const fn detailed_stats_col_right(&mut self) {
        self.detailed_stats_state.col = 1;
    }

    pub const fn show_mean_detail(&self) -> bool {
        self.detailed_stats_state.show_mean_detail
    }

    pub fn open_mean_detail(&mut self) {
        let row = self.detailed_stats_state.row;
        let col = self.detailed_stats_state.col;
        let has_mean = if col == 0 {
            self.history().mo3_at(row).is_some()
        } else {
            self.history().ao5_at(row).is_some()
        };
        if has_mean {
            self.detailed_stats_state.show_mean_detail = true;
            self.detailed_stats_state.mean_detail_selected_index = 0;
        }
    }

    pub const fn close_mean_detail(&mut self) {
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
    }

    pub fn mean_detail_times_len(&self) -> usize {
        let row = self.detailed_stats_state.row;
        if self.detailed_stats_state.col == 0 {
            self.history().mo3_times_at(row).map_or(0, <[Time]>::len)
        } else {
            self.history().ao5_times_at(row).map_or(0, <[Time]>::len)
        }
    }

    pub const fn mean_detail_selected_index(&self) -> usize {
        self.detailed_stats_state.mean_detail_selected_index
    }

    pub const fn mean_detail_select_up(&mut self) {
        self.detailed_stats_state.mean_detail_selected_index = self
            .detailed_stats_state
            .mean_detail_selected_index
            .saturating_sub(1);
    }

    pub fn mean_detail_select_down(&mut self) {
        let max = self.mean_detail_times_len().saturating_sub(1);
        self.detailed_stats_state.mean_detail_selected_index = self
            .detailed_stats_state
            .mean_detail_selected_index
            .saturating_add(1)
            .min(max);
    }

    pub fn open_details_for_selected_mean_time(&mut self) -> bool {
        let row = self.detailed_stats_state.row;
        let col = self.detailed_stats_state.col;
        let selected_index = self.detailed_stats_state.mean_detail_selected_index;
        let solve_index = if col == 0 {
            if row < 2 {
                return false;
            }
            row.saturating_sub(2).saturating_add(selected_index)
        } else {
            if row < 4 {
                return false;
            }
            row.saturating_sub(4).saturating_add(selected_index)
        };

        if solve_index >= self.history().len() {
            return false;
        }

        self.history_mut().select_index(solve_index);
        self.close_detailed_stats();
        self.open_details();
        self.details_state.return_to_mean_detail = Some(MeanDetailReturnState {
            row,
            col,
            selected_index,
        });
        true
    }
}
