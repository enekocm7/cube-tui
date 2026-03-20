#[cfg(feature = "bluetooth")]
pub mod timer;

#[cfg(feature = "bluetooth")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BtTimerState {
    Disconnected,
    GetSet,
    HandsOff,
    Running,
    Idle,
    HandsOn,
    Finished(u64),
    Error(String),
}

#[cfg(feature = "bluetooth")]
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: btleplug::platform::PeripheralId,
    pub name: Option<String>,
    pub rssi: Option<i16>,
    pub disconnected: bool,
}
