use crate::bluetooth::{BtTimerState, DeviceInfo};
use btleplug::platform::PeripheralId;

pub type BluetoothConnection = (
    flume::Sender<BtTimerState>,
    flume::Receiver<()>,
    btleplug::platform::Adapter,
    flume::Sender<()>,
);

#[derive(Debug)]
pub enum BluetoothEvent {
    Status(String),
    Error(String),
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
    pub status: Option<String>,
    pub rx: Option<flume::Receiver<BluetoothEvent>>,
    pub timer_rx: Option<flume::Receiver<BtTimerState>>,
    pub cancel_tx: Option<flume::Sender<()>>,
    pub connected_rx: Option<flume::Receiver<()>>,
    pub adapter: Option<btleplug::platform::Adapter>,
    pub connected_device_name: Option<String>,
    pub connected_device_id: Option<PeripheralId>,
}
