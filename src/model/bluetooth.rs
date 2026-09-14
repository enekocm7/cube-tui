use std::borrow::Cow;
use std::time::Instant;

use btleplug::platform::PeripheralId;

use crate::bluetooth::{BtTimerState, DeviceInfo};
use crate::model::Model;
use crate::model::session::TimerState;
use crate::widgets::history::Modifier;

pub type BluetoothConnection = (
    flume::Sender<BtTimerState>,
    btleplug::platform::Adapter,
    flume::Sender<()>,
);

#[derive(Debug)]
pub enum BluetoothEvent {
    Status(Cow<'static, str>),
    Error(Cow<'static, str>),
    Device(DeviceInfo),
    Adapter(btleplug::platform::Adapter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BluetoothScreenState {
    #[default]
    Searching,
    Connecting,
    Connected,
}

#[derive(Default)]
pub struct BluetoothState {
    pub show: bool,
    pub screen_state: BluetoothScreenState,
    pub selected_index: usize,
    pub devices: Vec<DeviceInfo>,
    pub status: Option<Cow<'static, str>>,
    pub rx: Option<flume::Receiver<BluetoothEvent>>,
    pub timer_rx: Option<flume::Receiver<BtTimerState>>,
    pub connected_rx: Option<flume::Receiver<()>>,
    pub adapter: Option<btleplug::platform::Adapter>,
    pub connected_device_name: Option<String>,
    pub connected_device_id: Option<PeripheralId>,
}

impl Model {
    /// Returns whether Bluetooth still requires periodic channel polling.
    ///
    /// A disconnected receiver remains active while it contains buffered
    /// events, ensuring final status and timer messages are not lost.
    pub fn bluetooth_needs_poll(&self) -> bool {
        let state = &self.bluetooth_state;
        let scanning = self.show_bluetooth()
            && self.bluetooth_searching()
            && state.rx.as_ref().is_some_and(receiver_active);
        let timer = self.bluetooth_timer_active()
            && (state.timer_rx.as_ref().is_some_and(receiver_active)
                || state.connected_rx.as_ref().is_some_and(receiver_active));
        scanning || timer
    }

    /// Returns whether the Bluetooth device panel is visible.
    pub const fn show_bluetooth(&self) -> bool {
        self.bluetooth_state.show
    }

    /// Opens or closes Bluetooth discovery and returns the new scanner sender.
    pub fn toggle_bluetooth(&mut self) -> Option<flume::Sender<BluetoothEvent>> {
        self.bluetooth_state.show = !self.bluetooth_state.show;
        if self.bluetooth_state.show {
            if self.bluetooth_state.screen_state == BluetoothScreenState::Connected {
                self.sync_connected_device_list();
                let name = self
                    .bluetooth_state
                    .connected_device_name
                    .as_deref()
                    .unwrap_or("device");
                self.bluetooth_state.status = Some(Cow::Owned(format!("✓ Connected to {name}")));
                self.bluetooth_state.rx = None;
                return None;
            }

            self.bluetooth_state.selected_index = 0;
            self.bluetooth_state.devices.clear();
            self.bluetooth_state.status = Some(Cow::Borrowed("Starting scan..."));
            let (tx, rx) = flume::unbounded();
            self.bluetooth_state.rx = Some(rx);
            self.bluetooth_state.screen_state = BluetoothScreenState::Searching;
            Some(tx)
        } else {
            self.stop_bluetooth_scan();
            None
        }
    }

    /// Closes Bluetooth UI state and cancels outstanding scanner work.
    pub fn close_bluetooth(&mut self) {
        self.bluetooth_state.show = false;
        self.stop_bluetooth_scan();
        if self.bluetooth_state.screen_state != BluetoothScreenState::Connected {
            self.bluetooth_state.timer_rx = None;
            self.bluetooth_state.connected_device_name = None;
        }
    }

    /// Drops the scanner channel and clears its transient state.
    fn stop_bluetooth_scan(&mut self) {
        self.bluetooth_state.rx = None;
        self.bluetooth_state.status = None;
    }

    /// Drains pending scanner events and reports whether model state changed.
    ///
    /// The return value lets the event loop poll an active scan without
    /// redrawing unchanged frames.
    pub fn poll_bluetooth(&mut self) -> bool {
        if self.bluetooth_state.screen_state != BluetoothScreenState::Searching {
            return false;
        }
        let Some(rx) = self.bluetooth_state.rx.take() else {
            return false;
        };

        let mut changed = false;
        while let Ok(event) = rx.try_recv() {
            changed = true;
            match event {
                BluetoothEvent::Status(status) => {
                    self.bluetooth_state.status = Some(status);
                }
                BluetoothEvent::Error(error) => {
                    if error.contains("No Bluetooth adapters found") {
                        self.bluetooth_state.status =
                            Some(Cow::Borrowed("⚠ No Bluetooth adapters found"));
                    } else {
                        self.bluetooth_state.status = Some(Cow::Owned(format!("Error: {error}")));
                    }
                }
                BluetoothEvent::Device(device) => {
                    self.upsert_bluetooth_device(device);
                    let count = self.bluetooth_state.devices.len();
                    self.bluetooth_state.status =
                        Some(Cow::Owned(format!("Scanning... ({count} device(s) found)")));
                }
                BluetoothEvent::Adapter(adapter) => {
                    self.bluetooth_state.adapter = Some(adapter);
                }
            }
        }

        self.bluetooth_state.rx = Some(rx);
        changed
    }

    /// Returns discovered Bluetooth timer devices in display order.
    pub fn bluetooth_devices(&self) -> &[DeviceInfo] {
        &self.bluetooth_state.devices
    }

    /// Inserts a discovered device or refreshes its existing list entry.
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

    /// Ensures the connected timer remains represented in the device list.
    fn sync_connected_device_list(&mut self) {
        self.bluetooth_state.devices = self
            .bluetooth_state
            .connected_device_id
            .as_ref()
            .map(|id| DeviceInfo {
                id: id.clone(),
                name: self.bluetooth_state.connected_device_name.clone(),
            })
            .into_iter()
            .collect();
        self.bluetooth_state.selected_index = 0;
    }

    /// Returns the most recent Bluetooth status or error message.
    pub fn bluetooth_status(&self) -> Option<&str> {
        self.bluetooth_state.status.as_deref()
    }

    /// Returns the selected Bluetooth device index.
    pub const fn bluetooth_selected_index(&self) -> usize {
        self.bluetooth_state.selected_index
    }

    /// Moves the Bluetooth device selection up one row.
    pub const fn bluetooth_select_up(&mut self) {
        self.bluetooth_state.selected_index = self.bluetooth_state.selected_index.saturating_sub(1);
    }

    /// Moves the Bluetooth device selection down within the discovered list.
    pub fn bluetooth_select_down(&mut self) {
        let max_index = self.bluetooth_state.devices.len().saturating_sub(1);
        self.bluetooth_state.selected_index =
            (self.bluetooth_state.selected_index + 1).min(max_index);
    }

    /// Returns the currently selected discovered device.
    pub fn bluetooth_selected_device(&self) -> Option<&DeviceInfo> {
        self.bluetooth_state
            .devices
            .get(self.bluetooth_state.selected_index)
    }

    /// Creates connection state for the selected device when possible.
    pub fn connect_bluetooth_device(&mut self) -> Option<BluetoothConnection> {
        if self.bluetooth_state.screen_state != BluetoothScreenState::Searching {
            return None;
        }
        let adapter = self.bluetooth_state.adapter.clone()?;
        let device = self
            .bluetooth_state
            .devices
            .get(self.bluetooth_state.selected_index)?;
        let device_name = device.name.clone();
        self.bluetooth_state.connected_device_id = Some(device.id.clone());
        let (tx, rx) = flume::unbounded();
        self.bluetooth_state.timer_rx = Some(rx);
        let (conn_tx, conn_rx) = flume::bounded(1);
        self.bluetooth_state.connected_rx = Some(conn_rx);
        self.bluetooth_state.connected_device_name = device_name;
        self.bluetooth_state.screen_state = BluetoothScreenState::Connecting;
        self.bluetooth_state.status = Some(Cow::Borrowed("Connecting..."));
        self.bluetooth_state.rx = None;
        Some((tx, adapter, conn_tx))
    }

    /// Drains connection and timer events and reports whether state changed.
    ///
    /// Finished solves are recorded and persisted here. Disconnect and error
    /// events also tear down the active connection before returning.
    pub fn poll_bluetooth_timer(&mut self) -> bool {
        let mut changed = false;
        if let Some(conn_rx) = &self.bluetooth_state.connected_rx
            && conn_rx.try_recv() == Ok(())
        {
            changed = true;
            self.bluetooth_state.screen_state = BluetoothScreenState::Connected;
            self.sync_connected_device_list();
            let name = self
                .bluetooth_state
                .connected_device_name
                .as_deref()
                .unwrap_or("device");
            self.bluetooth_state.status = Some(Cow::Owned(format!("✓ Connected to {name}")));
            self.bluetooth_state.connected_rx = None;
        }

        let Some(rx) = self.bluetooth_state.timer_rx.take() else {
            return changed;
        };

        let mut disconnected = false;
        while let Ok(bt_state) = rx.try_recv() {
            changed = true;
            match bt_state {
                BtTimerState::Idle | BtTimerState::GetSet | BtTimerState::HandsOn => {
                    self.current_session_mut().timer_state =
                        if matches!(bt_state, BtTimerState::Idle) {
                            self.current_session_mut().last_time_ms = 0;
                            self.current_session_mut().last_modifier = Modifier::None;
                            TimerState::Idle
                        } else {
                            TimerState::Pulsed
                        };
                }
                BtTimerState::HandsOff => {
                    self.current_session_mut().timer_state = TimerState::Idle;
                }
                BtTimerState::Running => {
                    self.current_session_mut().timer_state = TimerState::Running {
                        time: Instant::now(),
                        inspection_modifier: Modifier::None,
                    };
                }
                BtTimerState::Finished(time_ms) => {
                    self.record_solve(time_ms);
                    self.next_scramble();
                    crate::persistence::save(self);
                }
                BtTimerState::Disconnected => {
                    disconnected = true;
                    break;
                }
                BtTimerState::Error(err) => {
                    self.bluetooth_state.status = Some(Cow::Owned(format!("Error: {err}")));
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
        changed
    }

    /// Returns whether a Bluetooth timer is currently connected.
    pub fn bluetooth_connected(&self) -> bool {
        self.bluetooth_state.screen_state == BluetoothScreenState::Connected
    }

    /// Returns the high-level state displayed by the Bluetooth panel.
    pub const fn bluetooth_screen_state(&self) -> BluetoothScreenState {
        self.bluetooth_state.screen_state
    }

    /// Returns whether a Bluetooth connection attempt is in progress.
    pub fn bluetooth_connecting(&self) -> bool {
        self.bluetooth_state.screen_state == BluetoothScreenState::Connecting
    }

    /// Returns whether Bluetooth discovery is in progress.
    pub fn bluetooth_searching(&self) -> bool {
        self.bluetooth_state.screen_state == BluetoothScreenState::Searching
    }

    /// Returns whether a connected timer is actively timing or readying a solve.
    pub const fn bluetooth_timer_active(&self) -> bool {
        matches!(
            self.bluetooth_state.screen_state,
            BluetoothScreenState::Connecting | BluetoothScreenState::Connected
        )
    }

    /// Returns the connected timer's display name.
    pub fn connected_device_name(&self) -> Option<&str> {
        self.bluetooth_state.connected_device_name.as_deref()
    }

    /// Returns the connected timer's platform peripheral identifier.
    pub fn connected_device_id(&self) -> Option<PeripheralId> {
        self.bluetooth_state.connected_device_id.clone()
    }

    /// Tears down the active connection and returns its adapter and device ID.
    pub fn disconnect_bluetooth(
        &mut self,
    ) -> Option<(
        flume::Sender<BluetoothEvent>,
        flume::Receiver<BluetoothEvent>,
        btleplug::platform::Adapter,
    )> {
        self.bluetooth_state.timer_rx = None;
        self.bluetooth_state.connected_rx = None;
        self.bluetooth_state.connected_device_name = None;
        self.bluetooth_state.connected_device_id = None;
        if self.bluetooth_state.show {
            self.bluetooth_state.screen_state = BluetoothScreenState::Searching;
            self.bluetooth_state.selected_index = 0;
            self.bluetooth_state.devices.clear();
            self.bluetooth_state.status = Some(Cow::Borrowed("Starting scan..."));
            let (tx, rx) = flume::unbounded();
            self.bluetooth_state.rx = Some(rx.clone());
            let adapter = self.bluetooth_state.adapter.clone()?;
            Some((tx, rx, adapter))
        } else {
            None
        }
    }
}

/// Returns whether a receiver can still yield an event.
///
/// Buffered messages keep a disconnected channel active until it is drained.
fn receiver_active<T>(receiver: &flume::Receiver<T>) -> bool {
    !receiver.is_disconnected() || !receiver.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::update;
    use crate::msg::Msg;

    #[test]
    fn scan_waits_for_events_without_requesting_unchanged_frames() {
        let mut model = Model::new();
        let tx = model.toggle_bluetooth().unwrap();
        assert!(model.bluetooth_needs_poll());
        assert!(!update(&mut model, Msg::Tick));

        tx.send(BluetoothEvent::Status(Cow::Borrowed("Scanning")))
            .unwrap();
        assert!(update(&mut model, Msg::Tick));
        assert_eq!(model.bluetooth_status(), Some("Scanning"));
        assert!(!update(&mut model, Msg::Tick));
        assert!(model.bluetooth_needs_poll());

        model.close_bluetooth();
        assert!(!model.bluetooth_needs_poll());
    }

    #[test]
    fn finished_scan_delivers_buffered_events_before_polling_stops() {
        let mut model = Model::new();
        let tx = model.toggle_bluetooth().unwrap();
        tx.send(BluetoothEvent::Error(Cow::Borrowed(
            "No Bluetooth adapters found",
        )))
        .unwrap();
        drop(tx);

        assert!(model.bluetooth_needs_poll());
        assert!(update(&mut model, Msg::Tick));
        assert_eq!(
            model.bluetooth_status(),
            Some("⚠ No Bluetooth adapters found")
        );
        assert!(!model.bluetooth_needs_poll());
    }

    #[test]
    fn connection_and_timer_events_redraw_when_bluetooth_panel_is_closed() {
        let mut model = Model::new();
        let (timer_tx, timer_rx) = flume::unbounded();
        let (connected_tx, connected_rx) = flume::bounded(1);
        model.bluetooth_state.screen_state = BluetoothScreenState::Connecting;
        model.bluetooth_state.timer_rx = Some(timer_rx);
        model.bluetooth_state.connected_rx = Some(connected_rx);

        assert!(!model.show_bluetooth());
        assert!(model.bluetooth_needs_poll());
        assert!(!update(&mut model, Msg::Tick));

        connected_tx.send(()).unwrap();
        assert!(update(&mut model, Msg::Tick));
        assert!(model.bluetooth_connected());
        assert!(model.bluetooth_needs_poll());

        timer_tx.send(BtTimerState::Running).unwrap();
        assert!(update(&mut model, Msg::Tick));
        assert!(matches!(model.timer_state(), TimerState::Running { .. }));
        assert!(!update(&mut model, Msg::Tick));
    }
}
