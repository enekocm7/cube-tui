use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Settings {
    pub timer: TimerSettings,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TimerSettings {
    pub inspection: bool,
}

impl Settings {
    pub const fn set_inspection(&mut self, inspection: bool) {
        self.timer.inspection = inspection;
    }
}
