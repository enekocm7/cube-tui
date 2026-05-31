use serde::{Deserialize, Serialize, de::Error};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Settings {
    timer: TimerSettings,
    #[serde(default)]
    theme: ThemeSettings,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimerSettings {
    inspection: bool,
    zen: bool,
}

impl Default for TimerSettings {
    fn default() -> Self {
        Self {
            inspection: true,
            zen: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThemeSettings {
    background: ColorSettings,
    border: ColorSettings,
    scramble: ColorSettings,
    selection: ColorSettings,
    selection_text: ColorSettings,
    text: ColorSettings,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            background: ColorSettings::BLACK,
            border: ColorSettings::WHITE,
            scramble: ColorSettings::WHITE,
            selection: ColorSettings::BLUE,
            selection_text: ColorSettings::BLACK,
            text: ColorSettings::WHITE,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ColorSettings {
    r: u8,
    g: u8,
    b: u8,
}

impl ColorSettings {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLUE: Self = Self {
        r: 51,
        g: 153,
        b: 255,
    };

    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#')?;
        if s.len() != 6 {
            return None;
        }
        Some(Self {
            r: u8::from_str_radix(&s[0..2], 16).ok()?,
            g: u8::from_str_radix(&s[2..4], 16).ok()?,
            b: u8::from_str_radix(&s[4..6], 16).ok()?,
        })
    }
    fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl Serialize for ColorSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_hex().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ColorSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).ok_or_else(|| Error::custom(format!("Invalid hex color: {s}")))
    }
}
