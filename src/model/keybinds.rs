use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::str::FromStr;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Quit,
    ResetTimer,
    Timer,
    SelectUp,
    SelectDown,
    NavigateLeft,
    NavigateRight,
    ToggleFocus,
    NextEvent,
    PreviousEvent,
    NextSession,
    PreviousSession,
    NewSession,
    DeleteSession,
    NextScramble,
    Help,
    ToggleInspection,
    DetailedStats,
    ThemeSelector,
    DeleteTime,
    Bluetooth,
    DisconnectBluetooth,
    ToggleZen,
    Enter,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyBinding {
    /// Creates a binding from a terminal key code and modifier set.
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Returns whether an input event matches this normalized binding.
    pub fn matches(self, event: KeyEvent) -> bool {
        let event = Self::normalize(event.code, event.modifiers);
        self == event
    }

    /// Canonicalizes shifted letters so configuration and events compare equally.
    fn normalize(mut code: KeyCode, mut modifiers: KeyModifiers) -> Self {
        if let KeyCode::Char(character) = &mut code
            && modifiers.contains(KeyModifiers::SHIFT)
        {
            if character.is_ascii_lowercase() {
                character.make_ascii_uppercase();
            }
            modifiers.remove(KeyModifiers::SHIFT);
        }
        Self { code, modifiers }
    }
}

impl FromStr for KeyBinding {
    type Err = String;

    /// Parses strings such as `Ctrl+Q`, `F5`, or `Space` into a binding.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            return Err("key binding cannot be empty".to_owned());
        }

        let mut modifiers = KeyModifiers::empty();
        let mut parts = value.split('+').peekable();
        let mut key = None;
        while let Some(part) = parts.next() {
            let token = part.trim();
            let is_last = parts.peek().is_none();
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" if !is_last => modifiers.insert(KeyModifiers::CONTROL),
                "alt" if !is_last => modifiers.insert(KeyModifiers::ALT),
                "shift" if !is_last => modifiers.insert(KeyModifiers::SHIFT),
                _ if is_last => key = Some(parse_key_code(token)?),
                _ => return Err(format!("unknown modifier `{token}`")),
            }
        }

        Ok(Self::normalize(
            key.ok_or_else(|| "key binding must include a key".to_owned())?,
            modifiers,
        ))
    }
}

/// Parses the final key token in a configured binding.
fn parse_key_code(value: &str) -> Result<KeyCode, String> {
    let lower = value.to_ascii_lowercase();
    let code = match lower.as_str() {
        "space" => KeyCode::Char(' '),
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        _ if lower.starts_with('f') => {
            let number = lower[1..]
                .parse::<u8>()
                .map_err(|_| format!("unknown key `{value}`"))?;
            if !(1..=24).contains(&number) {
                return Err(format!(
                    "function key must be between F1 and F24, got `{value}`"
                ));
            }
            KeyCode::F(number)
        }
        _ => {
            let mut chars = value.chars();
            let character = chars
                .next()
                .ok_or_else(|| "key cannot be empty".to_owned())?;
            if chars.next().is_some() {
                return Err(format!("unknown key `{value}`"));
            }
            KeyCode::Char(character)
        }
    };
    Ok(code)
}

impl fmt::Display for KeyBinding {
    /// Formats a binding in the same stable notation accepted by the parser.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            formatter.write_str("Ctrl+")?;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            formatter.write_str("Alt+")?;
        }
        match self.code {
            KeyCode::Char(' ') => formatter.write_str("Space"),
            KeyCode::Char(character) => write!(formatter, "{character}"),
            KeyCode::Enter => formatter.write_str("Enter"),
            KeyCode::Esc => formatter.write_str("Esc"),
            KeyCode::Tab => formatter.write_str("Tab"),
            KeyCode::BackTab => formatter.write_str("BackTab"),
            KeyCode::Up => formatter.write_str("Up"),
            KeyCode::Down => formatter.write_str("Down"),
            KeyCode::Left => formatter.write_str("Left"),
            KeyCode::Right => formatter.write_str("Right"),
            KeyCode::Backspace => formatter.write_str("Backspace"),
            KeyCode::Delete => formatter.write_str("Delete"),
            KeyCode::Insert => formatter.write_str("Insert"),
            KeyCode::Home => formatter.write_str("Home"),
            KeyCode::End => formatter.write_str("End"),
            KeyCode::PageUp => formatter.write_str("PageUp"),
            KeyCode::PageDown => formatter.write_str("PageDown"),
            KeyCode::F(number) => write!(formatter, "F{number}"),
            _ => formatter.write_str("Unsupported"),
        }
    }
}

impl Serialize for KeyBinding {
    /// Serializes a binding using its human-readable configuration notation.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KeyBinding {
    /// Parses and validates a binding from its serialized string.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone)]
pub struct Keybinds(BTreeMap<Action, KeyBinding>);

impl Keybinds {
    /// Returns the binding assigned to an action.
    pub fn get(&self, action: Action) -> KeyBinding {
        self.0[&action]
    }

    /// Returns a display label for an action's binding.
    pub fn label(&self, action: Action) -> String {
        self.get(action).to_string()
    }

    /// Finds the action assigned to a terminal key event.
    pub fn action_for(&self, event: KeyEvent) -> Option<Action> {
        self.0
            .iter()
            .find_map(|(action, binding)| binding.matches(event).then_some(*action))
    }

    /// Rejects duplicate bindings and unsafe timer modifiers.
    fn validate(&self) -> Result<(), String> {
        let timer = self.get(Action::Timer);
        if !timer.modifiers.is_empty() {
            return Err(
                "`timer` must use an unmodified key so press/release events remain reliable"
                    .to_owned(),
            );
        }
        let mut bindings = HashSet::new();
        for (action, binding) in &self.0 {
            if !bindings.insert(*binding) {
                return Err(format!(
                    "duplicate key binding `{binding}` (including `{action:?}`)"
                ));
            }
        }
        Ok(())
    }
}

impl Default for Keybinds {
    /// Returns the complete built-in keyboard layout.
    fn default() -> Self {
        use Action::*;
        let none = KeyModifiers::empty();
        let bindings = [
            (Quit, KeyBinding::new(KeyCode::Char('q'), none)),
            (ResetTimer, KeyBinding::new(KeyCode::Char('r'), none)),
            (Timer, KeyBinding::new(KeyCode::Char(' '), none)),
            (SelectUp, KeyBinding::new(KeyCode::Up, none)),
            (SelectDown, KeyBinding::new(KeyCode::Down, none)),
            (NavigateLeft, KeyBinding::new(KeyCode::Left, none)),
            (NavigateRight, KeyBinding::new(KeyCode::Right, none)),
            (ToggleFocus, KeyBinding::new(KeyCode::Tab, none)),
            (NextEvent, KeyBinding::new(KeyCode::Char('e'), none)),
            (PreviousEvent, KeyBinding::new(KeyCode::Char('E'), none)),
            (NextSession, KeyBinding::new(KeyCode::Char(']'), none)),
            (PreviousSession, KeyBinding::new(KeyCode::Char('['), none)),
            (NewSession, KeyBinding::new(KeyCode::Char('s'), none)),
            (DeleteSession, KeyBinding::new(KeyCode::Char('S'), none)),
            (NextScramble, KeyBinding::new(KeyCode::Char('n'), none)),
            (Help, KeyBinding::new(KeyCode::Char('?'), none)),
            (ToggleInspection, KeyBinding::new(KeyCode::Char('i'), none)),
            (DetailedStats, KeyBinding::new(KeyCode::Char('a'), none)),
            (ThemeSelector, KeyBinding::new(KeyCode::Char('t'), none)),
            (DeleteTime, KeyBinding::new(KeyCode::Char('d'), none)),
            (Bluetooth, KeyBinding::new(KeyCode::Char('b'), none)),
            (
                DisconnectBluetooth,
                KeyBinding::new(KeyCode::Char('x'), none),
            ),
            (ToggleZen, KeyBinding::new(KeyCode::Char('z'), none)),
            (Enter, KeyBinding::new(KeyCode::Enter, none)),
            (Back, KeyBinding::new(KeyCode::Esc, none)),
        ];
        Self(bindings.into_iter().collect())
    }
}

impl Serialize for Keybinds {
    /// Serializes the action-to-binding map.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Keybinds {
    /// Merges configured bindings with defaults and validates the result.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let overrides = BTreeMap::<Action, KeyBinding>::deserialize(deserializer)?;
        let mut keybinds = Self::default();
        keybinds.0.extend(overrides);
        keybinds.validate().map_err(D::Error::custom)?;
        Ok(keybinds)
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{Action, KeyBinding, Keybinds};

    #[test]
    fn partial_configuration_keeps_other_defaults() {
        let keybinds: Keybinds = toml::from_str("next_scramble = \"Ctrl+n\"").unwrap();

        assert_eq!(keybinds.label(Action::NextScramble), "Ctrl+n");
        assert_eq!(keybinds.label(Action::Quit), "q");
    }

    #[test]
    fn duplicate_binding_is_rejected() {
        let error = toml::from_str::<Keybinds>("next_scramble = \"q\"").unwrap_err();

        assert!(error.to_string().contains("duplicate key binding `q`"));
    }

    #[test]
    fn modified_timer_binding_is_rejected() {
        let error = toml::from_str::<Keybinds>("timer = \"Ctrl+Space\"").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("timer` must use an unmodified key")
        );
    }

    #[test]
    fn shifted_letters_are_normalized() {
        let binding: KeyBinding = "Shift+e".parse().unwrap();
        let event = KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT);

        assert_eq!(binding.to_string(), "E");
        assert!(binding.matches(event));
    }

    #[test]
    fn unsupported_and_unknown_keys_are_rejected() {
        assert!("F25".parse::<KeyBinding>().is_err());
        assert!("Ctrl+NotAKey".parse::<KeyBinding>().is_err());
    }
}
