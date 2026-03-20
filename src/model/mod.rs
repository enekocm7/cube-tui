use std::time::Instant;
#[cfg(feature = "bluetooth")]
use btleplug::platform::PeripheralId;
use crate::bluetooth::{BtTimerState, DeviceInfo};
use crate::scramble::{self, Scramble, WcaEvent};
use crate::widgets::history::{History, Modifier, Time};
use serde::{Deserialize, Serialize};

pub const MAX_SESSIONS: usize = 99;

#[cfg(feature = "bluetooth")]
pub type BluetoothConnection = (
    flume::Sender<BtTimerState>,
    flume::Receiver<()>,
    btleplug::platform::Adapter,
    flume::Sender<()>,
);

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
    return_to_stats: Option<StatsReturnState>,
}

#[derive(Copy, Clone)]
struct MeanDetailReturnState {
    row: usize,
    col: usize,
    selected_index: usize,
}

#[derive(Copy, Clone)]
struct StatsReturnState {
    row: usize,
    col: usize,
}

#[derive(Default)]
struct DetailedStatsState {
    show: bool,
    row: usize,
    col: usize,
    show_mean_detail: bool,
    mean_detail_selected_index: usize,
    opened_from_stats_column: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum MainFocus {
    History,
    Stats,
}

struct MainStatsSelection {
    row: usize,
    col: usize,
}

impl Default for MainStatsSelection {
    fn default() -> Self {
        Self { row: 1, col: 0 }
    }
}

#[cfg(feature = "bluetooth")]
#[derive(Debug)]
pub enum BluetoothEvent {
    Status(String),
    Error(String),
    Device(DeviceInfo),
    Disconnected(DeviceInfo),
    Adapter(btleplug::platform::Adapter),
}

#[cfg(feature = "bluetooth")]
#[derive(Default)]
struct BluetoothState {
    show: bool,
    selected_index: usize,
    devices: Vec<DeviceInfo>,
    status: Option<String>,
    rx: Option<flume::Receiver<BluetoothEvent>>,
    scanning: bool,
    timer_rx: Option<flume::Receiver<BtTimerState>>,
    cancel_tx: Option<flume::Sender<()>>,
    connected_rx: Option<flume::Receiver<()>>,
    adapter: Option<btleplug::platform::Adapter>,
    connected: bool,
    connected_device_name: Option<String>,
    connected_device_id: Option<PeripheralId>,
}

pub struct Model {
    session_state: SessionState,
    settings: Settings,
    help_state: HelpState,
    details_state: DetailsState,
    detailed_stats_state: DetailedStatsState,
    #[cfg(feature = "bluetooth")]
    bluetooth_state: BluetoothState,
    main_focus: MainFocus,
    main_stats_selection: MainStatsSelection,
}

impl Model {
    pub fn new() -> Self {
        Self {
            session_state: SessionState::new(),
            settings: Settings::default(),
            help_state: HelpState::default(),
            details_state: DetailsState::default(),
            detailed_stats_state: DetailedStatsState::default(),
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
        #[cfg(feature = "bluetooth")]
        {
            self.bluetooth_state = BluetoothState::default();
        }
        self.main_focus = MainFocus::History;
        self.main_stats_selection = MainStatsSelection::default();
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

    pub fn open_mean_detail_from_stats(&mut self) -> bool {
        let row = self.main_stats_selection.row;
        let col = self.main_stats_selection.col;

        if row == 0 {
            let solve_index = if col == 0 {
                self.history().len().checked_sub(1)
            } else {
                self.history()
                    .times()
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, t)| t.raw_ms())
                    .map(|(i, _)| i)
            };

            let Some(solve_index) = solve_index else {
                return false;
            };

            self.history_mut().select_index(solve_index);
            self.open_details();
            self.details_state.return_to_stats = Some(StatsReturnState { row, col });
            return true;
        }

        let solve_index = match (row, col) {
            (1, 0) => self.history().latest_mo3_index(),
            (1, 1) => self.history().fastest_mo3_index(),
            (2, 0) => self.history().latest_ao5_index(),
            (2, 1) => self.history().fastest_ao5_index(),
            _ => None,
        };

        let Some(solve_index) = solve_index else {
            return false;
        };

        self.detailed_stats_state.show = true;
        self.detailed_stats_state.row = solve_index;
        self.detailed_stats_state.col = usize::from(row != 1);
        self.detailed_stats_state.show_mean_detail = true;
        self.detailed_stats_state.mean_detail_selected_index = 0;
        self.detailed_stats_state.opened_from_stats_column = true;
        true
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
}

impl Model {
    #[cfg(feature = "bluetooth")]
    pub const fn show_bluetooth(&self) -> bool {
        self.bluetooth_state.show
    }

    #[cfg(feature = "bluetooth")]
    pub fn toggle_bluetooth(&mut self) -> Option<flume::Sender<BluetoothEvent>> {
        self.bluetooth_state.show = !self.bluetooth_state.show;
        if self.bluetooth_state.show {
            self.bluetooth_state.selected_index = 0;
            self.bluetooth_state.devices.clear();
            self.bluetooth_state.status = Some("Starting scan...".to_string());
            let (tx, rx) = flume::unbounded();
            self.bluetooth_state.rx = Some(rx);
            self.bluetooth_state.scanning = true;
            Some(tx)
        } else {
            self.stop_bluetooth_scan();
            None
        }
    }

    #[cfg(feature = "bluetooth")]
    pub fn close_bluetooth(&mut self) {
        self.bluetooth_state.show = false;
        self.stop_bluetooth_scan();
        if !self.bluetooth_state.connected {
            self.bluetooth_state.timer_rx = None;
            self.bluetooth_state.connected_device_name = None;
        }
    }

    #[cfg(feature = "bluetooth")]
    fn stop_bluetooth_scan(&mut self) {
        self.bluetooth_state.rx = None;
        self.bluetooth_state.scanning = false;
        self.bluetooth_state.status = None;
    }

    #[cfg(feature = "bluetooth")]
    pub fn poll_bluetooth(&mut self) {
        let Some(rx) = self.bluetooth_state.rx.take() else {
            return;
        };

        while let Ok(event) = rx.try_recv() {
            match event {
                BluetoothEvent::Status(status) => {
                    self.bluetooth_state.status = Some(status);
                }
                BluetoothEvent::Error(error) => {
                    self.bluetooth_state.scanning = false;
                    if error.contains("No Bluetooth adapters found") {
                        self.bluetooth_state.status =
                            Some("⚠ No Bluetooth adapters found".to_string());
                    } else {
                        self.bluetooth_state.status = Some(format!("Error: {error}"));
                    }
                }
                BluetoothEvent::Device(device) => {
                    self.upsert_bluetooth_device(device);
                    let count = self.bluetooth_state.devices.len();
                    self.bluetooth_state.status =
                        Some(format!("Scanning... ({count} device(s) found)"));
                }
                BluetoothEvent::Adapter(adapter) => {
                    self.bluetooth_state.adapter = Some(adapter);
                }
                BluetoothEvent::Disconnected(device) => {
                    self.remove_bluetooth_device(&device);
                    let count = self.bluetooth_state.devices.len();
                    self.bluetooth_state.status =
                        Some(format!("Scanning... ({count} device(s) found)"));
                }
            }
        }

        self.bluetooth_state.rx = Some(rx);
    }

    #[cfg(feature = "bluetooth")]
    pub fn bluetooth_devices(&self) -> &[DeviceInfo] {
        &self.bluetooth_state.devices
    }

    #[cfg(feature = "bluetooth")]
    fn upsert_bluetooth_device(&mut self, device: DeviceInfo) {
        let existing = self
            .bluetooth_state
            .devices
            .iter_mut()
            .find(|entry| entry.id == device.id);

        if let Some(existing) = existing {
            *existing = device;
            return;
        }
        self.bluetooth_state.devices.push(device);
    }

    #[cfg(feature = "bluetooth")]
    fn remove_bluetooth_device(&mut self, device: &DeviceInfo) {
        if let Some(device_index) = self
            .bluetooth_state
            .devices
            .iter()
            .enumerate()
            .find_map(|(i, dev)| if dev.id == device.id { Some(i) } else { None })
        {
            self.bluetooth_state.devices.remove(device_index);
        }
    }

    #[cfg(feature = "bluetooth")]
    pub fn bluetooth_status(&self) -> Option<&str> {
        self.bluetooth_state.status.as_deref()
    }

    #[cfg(feature = "bluetooth")]
    pub const fn bluetooth_selected_index(&self) -> usize {
        self.bluetooth_state.selected_index
    }

    #[cfg(feature = "bluetooth")]
    pub const fn bluetooth_select_up(&mut self) {
        self.bluetooth_state.selected_index = self.bluetooth_state.selected_index.saturating_sub(1);
    }

    #[cfg(feature = "bluetooth")]
    pub fn bluetooth_select_down(&mut self) {
        let max_index = self.bluetooth_state.devices.len().saturating_sub(1);
        self.bluetooth_state.selected_index =
            (self.bluetooth_state.selected_index + 1).min(max_index);
    }

    #[cfg(feature = "bluetooth")]
    pub fn bluetooth_selected_device(&self) -> Option<&DeviceInfo> {
        self.bluetooth_state
            .devices
            .get(self.bluetooth_state.selected_index)
    }

    #[cfg(feature = "bluetooth")]
    pub fn connect_bluetooth_device(&mut self) -> Option<BluetoothConnection> {
        let adapter = self.bluetooth_state.adapter.clone()?;
        let device = self
            .bluetooth_state
            .devices
            .get(self.bluetooth_state.selected_index)?;
        let device_name = device.name.clone();
        self.bluetooth_state.connected_device_id = Some(device.id.clone());
        let (tx, rx) = flume::unbounded();
        self.bluetooth_state.timer_rx = Some(rx);
        let (cancel_tx, cancel_rx) = flume::bounded(1);
        self.bluetooth_state.cancel_tx = Some(cancel_tx);
        let (conn_tx, conn_rx) = flume::bounded(1);
        self.bluetooth_state.connected_rx = Some(conn_rx);
        self.bluetooth_state.connected = false;
        self.bluetooth_state.connected_device_name = device_name;
        self.bluetooth_state.status = Some("Connecting...".to_string());
        Some((tx, cancel_rx, adapter, conn_tx))
    }

    #[cfg(feature = "bluetooth")]
    pub fn poll_bluetooth_timer(&mut self) {
        if let Some(conn_rx) = &self.bluetooth_state.connected_rx
            && conn_rx.try_recv() == Ok(())
        {
            self.bluetooth_state.connected = true;
            let name = self
                .bluetooth_state
                .connected_device_name
                .as_deref()
                .unwrap_or("device");
            self.bluetooth_state.status = Some(format!("✓ Connected to {name}"));
            self.bluetooth_state.connected_rx = None;
        }

        let Some(rx) = self.bluetooth_state.timer_rx.take() else {
            return;
        };

        let mut disconnected = false;
        while let Ok(bt_state) = rx.try_recv() {
            match bt_state {
                BtTimerState::Idle | BtTimerState::GetSet | BtTimerState::HandsOn => {
                    self.get_current_session_mut().timer_state =
                        if matches!(bt_state, BtTimerState::Idle) {
                            TimerState::Idle
                        } else {
                            TimerState::Pulsed
                        };
                }
                BtTimerState::HandsOff => {
                    self.get_current_session_mut().timer_state = TimerState::Idle;
                }
                BtTimerState::Running => {
                    self.get_current_session_mut().timer_state =
                        TimerState::Running(Instant::now());
                }
                BtTimerState::Finished(time_ms) => {
                    self.get_current_session_mut().last_time_ms = time_ms;
                    let event = self.event();
                    let scramble = self.scramble().as_str().to_string();
                    self.history_mut().add_ms(time_ms, event, scramble);
                    self.get_current_session_mut().timer_state = TimerState::Idle;
                    self.next_scramble();
                    crate::persistence::save(self);
                }
                BtTimerState::Disconnected => {
                    disconnected = true;
                    break;
                }
                BtTimerState::Error(err) => {
                    self.bluetooth_state.status = Some(format!("Error: {err}"));
                    disconnected = true;
                    break;
                }
            }
        }

        if disconnected {
            self.disconnect_bluetooth();
        } else {
            self.bluetooth_state.timer_rx = Some(rx);
        }
    }

    #[cfg(feature = "bluetooth")]
    pub const fn bluetooth_connected(&self) -> bool {
        self.bluetooth_state.connected
    }

    #[cfg(feature = "bluetooth")]
    pub const fn bluetooth_timer_active(&self) -> bool {
        self.bluetooth_state.timer_rx.is_some()
    }

    #[cfg(feature = "bluetooth")]
    pub fn connected_device_name(&self) -> Option<&str> {
        self.bluetooth_state.connected_device_name.as_deref()
    }

    #[cfg(feature = "bluetooth")]
    pub fn connected_device_id(&self) -> Option<PeripheralId> {
        self.bluetooth_state.connected_device_id.clone()
    }

    /// Signals the background connection thread to disconnect and clears state.
    ///
    /// Dropping `cancel_tx` immediately unblocks the `recv_async()` in the
    /// background thread's `tokio::select!`, which then calls `timer::disconnect`
    /// and exits. No runtime or blocking on the calling thread.
    #[cfg(feature = "bluetooth")]
    pub fn disconnect_bluetooth(&mut self) {
        self.bluetooth_state.cancel_tx = None;
        self.bluetooth_state.timer_rx = None;
        self.bluetooth_state.connected_rx = None;
        self.bluetooth_state.connected = false;
        self.bluetooth_state.connected_device_name = None;
        self.bluetooth_state.connected_device_id = None;
    }
}

impl Model {
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
        self.details_state.return_to_stats = None;
        self.details_state.modifier_index = match self.history().selected_time().map(Time::modifier)
        {
            Some(Modifier::DNF) => 1,
            _ => 0,
        };
    }

    pub const fn close_details(&mut self) {
        self.details_state.show = false;
        self.details_state.return_to_mean_detail = None;
        self.details_state.return_to_stats = None;
    }

    pub const fn can_return_to_stats(&self) -> bool {
        self.details_state.return_to_stats.is_some()
    }

    pub const fn return_to_stats_column(&mut self) -> bool {
        let Some(return_state) = self.details_state.return_to_stats.take() else {
            return false;
        };
        self.details_state.show = false;
        self.main_focus = MainFocus::Stats;
        self.main_stats_selection.row = return_state.row;
        self.main_stats_selection.col = return_state.col;
        true
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
        self.detailed_stats_state.opened_from_stats_column = false;
    }

    pub const fn close_detailed_stats(&mut self) {
        self.detailed_stats_state.show = false;
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
        self.detailed_stats_state.opened_from_stats_column = false;
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
            self.detailed_stats_state.opened_from_stats_column = false;
        }
    }

    pub const fn close_mean_detail(&mut self) {
        let opened_from_stats_column = self.detailed_stats_state.opened_from_stats_column;
        self.detailed_stats_state.show_mean_detail = false;
        self.detailed_stats_state.mean_detail_selected_index = 0;
        self.detailed_stats_state.opened_from_stats_column = false;
        if opened_from_stats_column {
            self.detailed_stats_state.show = false;
        }
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
