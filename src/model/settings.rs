use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Settings {
    timer: TimerSettings,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TimerSettings {
    inspection: bool,
}

impl Settings {
    pub const fn set_inspection(&mut self, inspection: bool) {
        self.timer.inspection = inspection;
    }

    pub const fn inspection(&self) -> bool {
        self.timer.inspection
    }
}
