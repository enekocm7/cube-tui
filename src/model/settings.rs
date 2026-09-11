use cube_tui_macros::ColorGetters;
use serde::{Deserialize, Serialize, de::Error};

use crate::model::keybinds::Keybinds;
use crate::persistence;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub timer: TimerSettings,
    #[serde(default)]
    theme: Theme,
    display: DisplaySettings,
    #[serde(default)]
    keybinds: Keybinds,
}

impl Settings {
    /// Returns the minimum width required by the enabled side panels.
    pub const fn minimum_terminal_width(&self) -> u16 {
        self.display.minimum_terminal_width()
    }

    /// Returns the minimum height required by the current display settings.
    pub const fn minimum_terminal_height(&self) -> u16 {
        self.display.minimum_terminal_height()
    }

    /// Sets whether WCA inspection is enabled.
    pub const fn set_inspection(&mut self, inspection: bool) {
        self.timer.inspection = inspection;
    }

    /// Returns whether WCA inspection is enabled.
    pub const fn inspection(&self) -> bool {
        self.timer.inspection
    }

    /// Sets whether zen mode is enabled.
    pub const fn set_zen(&mut self, zen: bool) {
        self.timer.zen = zen;
    }

    /// Returns whether zen mode is enabled.
    pub const fn zen(&self) -> bool {
        self.timer.zen
    }

    /// Returns whether the solve-history panel is visible.
    pub const fn history(&self) -> bool {
        self.display.history
    }

    /// Returns whether the statistics panel is visible.
    pub const fn stats(&self) -> bool {
        self.display.stats
    }

    /// Returns whether the scramble panel is visible.
    pub const fn scramble(&self) -> bool {
        self.display.scramble
    }

    /// Returns the colors of the active theme.
    pub const fn theme(&self) -> &ThemeColors {
        &self.theme.theme
    }

    /// Returns the configured theme filename.
    pub fn theme_name(&self) -> &str {
        self.theme.path.as_str()
    }

    /// Replaces the active theme name and colors.
    pub fn set_theme(&mut self, theme: &Theme) {
        self.theme.clone_from(theme);
    }

    /// Returns the configured keyboard bindings.
    pub const fn keybinds(&self) -> &Keybinds {
        &self.keybinds
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
    /// Enables inspection and disables zen mode by default.
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

impl Theme {
    /// Creates a named theme from an already parsed color palette.
    pub fn new(name: &str, colors: ThemeColors) -> Self {
        Self {
            path: name.to_owned(),
            theme: colors,
        }
    }

    /// Returns the theme's persisted filename.
    pub fn name(&self) -> &str {
        &self.path
    }
}

impl<'de> Deserialize<'de> for Theme {
    /// Loads the named palette while deserializing the theme reference.
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
    /// Loads `default.toml`, falling back to built-in colors.
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
    /// Returns the built-in high-contrast dark palette.
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

    /// Converts the RGB triplet to a Ratatui color.
    pub const fn to_color(self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(self.r, self.g, self.b)
    }

    /// Parses a six-digit `#RRGGBB` color.
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
    /// Formats the color as an uppercase `#RRGGBB` value.
    fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl Serialize for ColorSettings {
    /// Serializes the color as a hexadecimal string.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_hex().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ColorSettings {
    /// Deserializes and validates a hexadecimal color string.
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
    /// Makes the history, scramble, and statistics panels visible.
    fn default() -> Self {
        Self {
            history: true,
            scramble: true,
            stats: true,
        }
    }
}

impl DisplaySettings {
    /// Computes the width occupied by the timer and enabled side panels.
    const fn minimum_terminal_width(self) -> u16 {
        const BASE_WIDTH: u16 = 28;
        const HISTORY_WIDTH: u16 = 24;
        const STATS_WIDTH: u16 = 30;

        BASE_WIDTH
            + if self.history { HISTORY_WIDTH } else { 0 }
            + if self.stats { STATS_WIDTH } else { 0 }
    }

    /// Computes the height required with or without the scramble panel.
    const fn minimum_terminal_height(self) -> u16 {
        if self.scramble { 20 } else { 13 }
    }
}

#[cfg(test)]
mod tests {
    use super::DisplaySettings;

    #[test]
    fn minimum_width_accounts_for_visible_side_panels() {
        let mut display = DisplaySettings::default();
        assert_eq!(display.minimum_terminal_width(), 82);

        display.history = false;
        assert_eq!(display.minimum_terminal_width(), 58);

        display.stats = false;
        assert_eq!(display.minimum_terminal_width(), 28);

        display.history = true;
        assert_eq!(display.minimum_terminal_width(), 52);
    }

    #[test]
    fn minimum_height_accounts_for_scramble() {
        let mut display = DisplaySettings::default();
        assert_eq!(display.minimum_terminal_height(), 20);

        display.scramble = false;
        assert_eq!(display.minimum_terminal_height(), 13);
    }
}
