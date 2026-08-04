use cube_tui_macros::ColorGetters;
use serde::{Deserialize, Serialize, de::Error};

use crate::persistence;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub timer: TimerSettings,
    #[serde(default)]
    theme: Theme,
    display: DisplaySettings,
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

    pub const fn history(&self) -> bool {
        self.display.history
    }

    pub const fn stats(&self) -> bool {
        self.display.stats
    }

    pub const fn scramble(&self) -> bool {
        self.display.scramble
    }

    pub const fn theme(&self) -> &ThemeColors {
        &self.theme.theme
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimerSettings {
    #[serde(default)]
    inspection: bool,
    #[serde(default)]
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

#[derive(Debug, Clone, Serialize)]
pub struct Theme {
    path: String,
    #[serde(skip)]
    theme: ThemeColors,
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ThemeDeserializer {
            path: Option<String>,
        }

        let path = ThemeDeserializer::deserialize(deserializer)?
            .path
            .filter(|p| !p.trim().is_empty())
            .unwrap_or("default.toml".to_owned());
        let theme = persistence::load_theme(&path).unwrap_or_default();
        Ok(Self { path, theme })
    }
}

impl Default for Theme {
    fn default() -> Self {
        let theme = persistence::load_theme("default.toml").unwrap_or_default();
        Self {
            path: "default.toml".to_owned(),
            theme,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ColorGetters)]
pub struct ThemeColors {
    background: ColorSettings,
    border: ColorSettings,
    scramble: ColorSettings,
    selection: ColorSettings,
    selection_text: ColorSettings,
    text: ColorSettings,
}

impl Default for ThemeColors {
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

    pub const fn to_color(self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(self.r, self.g, self.b)
    }

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

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct DisplaySettings {
    #[serde(default)]
    history: bool,
    #[serde(default)]
    scramble: bool,
    #[serde(default)]
    stats: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            history: true,
            scramble: true,
            stats: true,
        }
    }
}
