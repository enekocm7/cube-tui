use std::borrow::Cow;
use std::time::Instant;

use btleplug::platform::PeripheralId;

use crate::bluetooth::{BtTimerState, DeviceInfo};
use crate::model::Model;
use crate::model::session::TimerState;

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
    pub const fn show_bluetooth(&self) -> bool {
        self.bluetooth_state.show
    }

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

    pub fn close_bluetooth(&mut self) {
        self.bluetooth_state.show = false;
        self.stop_bluetooth_scan();
        if self.bluetooth_state.screen_state != BluetoothScreenState::Connected {
            self.bluetooth_state.timer_rx = None;
            self.bluetooth_state.connected_device_name = None;
        }
    }

    fn stop_bluetooth_scan(&mut self) {
        self.bluetooth_state.rx = None;
        self.bluetooth_state.status = None;
    }

    pub fn poll_bluetooth(&mut self) {
        if self.bluetooth_state.screen_state != BluetoothScreenState::Searching {
            return;
        }
        let Some(rx) = self.bluetooth_state.rx.take() else {
            return;
        };

        while let Ok(event) = rx.try_recv() {
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
    }

    pub fn bluetooth_devices(&self) -> &[DeviceInfo] {
        &self.bluetooth_state.devices
    }

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

    pub fn bluetooth_status(&self) -> Option<&str> {
        self.bluetooth_state.status.as_deref()
    }

    pub const fn bluetooth_selected_index(&self) -> usize {
        self.bluetooth_state.selected_index
    }

    pub const fn bluetooth_select_up(&mut self) {
        self.bluetooth_state.selected_index = self.bluetooth_state.selected_index.saturating_sub(1);
    }

    pub fn bluetooth_select_down(&mut self) {
        let max_index = self.bluetooth_state.devices.len().saturating_sub(1);
        self.bluetooth_state.selected_index =
            (self.bluetooth_state.selected_index + 1).min(max_index);
    }

    pub fn bluetooth_selected_device(&self) -> Option<&DeviceInfo> {
        self.bluetooth_state
            .devices
            .get(self.bluetooth_state.selected_index)
    }

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

    pub fn poll_bluetooth_timer(&mut self) {
        if let Some(conn_rx) = &self.bluetooth_state.connected_rx
            && conn_rx.try_recv() == Ok(())
        {
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
            return;
        };

        let mut disconnected = false;
        while let Ok(bt_state) = rx.try_recv() {
            match bt_state {
                BtTimerState::Idle | BtTimerState::GetSet | BtTimerState::HandsOn => {
                    self.get_current_session_mut().timer_state =
                        if matches!(bt_state, BtTimerState::Idle) {
                            self.get_current_session_mut().last_time_ms = 0;
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
                    let scramble = self.take_scramble();
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
    }

    pub fn bluetooth_connected(&self) -> bool {
        self.bluetooth_state.screen_state == BluetoothScreenState::Connected
    }

    pub const fn bluetooth_screen_state(&self) -> BluetoothScreenState {
        self.bluetooth_state.screen_state
    }

    pub fn bluetooth_connecting(&self) -> bool {
        self.bluetooth_state.screen_state == BluetoothScreenState::Connecting
    }

    pub fn bluetooth_searching(&self) -> bool {
        self.bluetooth_state.screen_state == BluetoothScreenState::Searching
    }

    pub const fn bluetooth_timer_active(&self) -> bool {
        matches!(
            self.bluetooth_state.screen_state,
            BluetoothScreenState::Connecting | BluetoothScreenState::Connected
        )
    }

    pub fn connected_device_name(&self) -> Option<&str> {
        self.bluetooth_state.connected_device_name.as_deref()
    }

    pub fn connected_device_id(&self) -> Option<PeripheralId> {
        self.bluetooth_state.connected_device_id.clone()
    }

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
