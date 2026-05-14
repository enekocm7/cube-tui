use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Settings {
    timer: TimerSettings,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TimerSettings {
    inspection: bool,
    zen: bool,
}

impl Settings {
    pub const fn set_inspection(&mut self, inspection: bool) {
        self.timer.inspection = inspection;
    }

    pub const fn inspection(&self) -> bool {
        self.timer.inspection
    }

    pub const fn set_zen(&mut self, zen: bool) {
        self.timer.zen = zen;
    }

    pub const fn zen(&self) -> bool {
        self.timer.zen
    }
}
