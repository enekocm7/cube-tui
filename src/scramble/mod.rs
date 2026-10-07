use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

pub mod visualization;
mod wca;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WcaEvent {
    Cube2x2,
    Cube3x3,
    Cube4x4,
    Cube5x5,
    Cube6x6,
    Cube7x7,
    Megaminx,
    Pyraminx,
    Fto,
    Skewb,
    Square1,
    Clock,
}

impl WcaEvent {
    pub const ALL: [Self; 12] = [
        Self::Cube2x2,
        Self::Cube3x3,
        Self::Cube4x4,
        Self::Cube5x5,
        Self::Cube6x6,
        Self::Cube7x7,
        Self::Megaminx,
        Self::Pyraminx,
        Self::Fto,
        Self::Skewb,
        Self::Square1,
        Self::Clock,
    ];

    /// Returns the human-readable puzzle name used by the UI.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cube2x2 => "2x2x2",
            Self::Cube3x3 => "3x3x3",
            Self::Cube4x4 => "4x4x4",
            Self::Cube5x5 => "5x5x5",
            Self::Cube6x6 => "6x6x6",
            Self::Cube7x7 => "7x7x7",
            Self::Megaminx => "Megaminx",
            Self::Pyraminx => "Pyraminx",
            Self::Fto => "FTO",
            Self::Skewb => "Skewb",
            Self::Square1 => "Square-1",
            Self::Clock => "Clock",
        }
    }

    /// Maps an event to its stable position in the navigation cycle.
    const fn as_index(self) -> usize {
        match self {
            Self::Cube2x2 => 0,
            Self::Cube3x3 => 1,
            Self::Cube4x4 => 2,
            Self::Cube5x5 => 3,
            Self::Cube6x6 => 4,
            Self::Cube7x7 => 5,
            Self::Megaminx => 6,
            Self::Pyraminx => 7,
            Self::Fto => 8,
            Self::Skewb => 9,
            Self::Square1 => 10,
            Self::Clock => 11,
        }
    }

    /// Maps a navigation-cycle position back to an event.
    const fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Cube2x2,
            2 => Self::Cube4x4,
            3 => Self::Cube5x5,
            4 => Self::Cube6x6,
            5 => Self::Cube7x7,
            6 => Self::Megaminx,
            7 => Self::Pyraminx,
            8 => Self::Fto,
            9 => Self::Skewb,
            10 => Self::Square1,
            11 => Self::Clock,
            _ => Self::Cube3x3,
        }
    }

    /// Returns the next event, wrapping after Clock.
    pub const fn next(self) -> Self {
        let index = self.as_index();
        let next_index = (index + 1) % 12;
        Self::from_index(next_index)
    }

    /// Returns the previous event, wrapping before 2×2.
    pub const fn prev(self) -> Self {
        let index = self.as_index();
        let prev_index = if index == 0 { 11 } else { index - 1 };
        Self::from_index(prev_index)
    }
}

pub struct Scramble {
    text: Cow<'static, str>,
    pub warning: Option<String>,
}

impl Scramble {
    /// Creates a scramble from its notation.
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: text.into(),
            warning: None,
        }
    }

    /// Returns the scramble notation as a string slice.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for Scramble {
    /// Writes the scramble notation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<Scramble> for Cow<'static, str> {
    /// Consumes a scramble and returns its owned or borrowed notation.
    fn from(scramble: Scramble) -> Self {
        scramble.text
    }
}

/// Generates an official WCA random-state scramble for `event`.
pub fn generate_scramble(event: WcaEvent) -> Scramble {
    Scramble::new(wca::get_wca_scramble(event))
}

/// Infers the most likely puzzle event from scramble notation.
pub fn classify_event(scramble: &str) -> WcaEvent {
    let text = scramble.trim();
    if text.is_empty() {
        return WcaEvent::Cube3x3;
    }

    if text.contains('(') || text.contains('/') {
        return WcaEvent::Square1;
    }

    if text.contains("R++")
        || text.contains("R--")
        || text.contains("D++")
        || text.contains("D--")
        || text.contains('\n')
    {
        return WcaEvent::Megaminx;
    }

    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.iter().any(|token| is_clock_token(token)) {
        return WcaEvent::Clock;
    }

    let move_count = tokens.len();
    let has_tip = tokens
        .iter()
        .any(|token| token.chars().next().is_some_and(|_| false));
    if has_tip {
        return WcaEvent::Pyraminx;
    }

    let bases: Vec<&str> = tokens.iter().map(|t| base_move(t)).collect();
    if bases.iter().all(|b| matches!(*b, "R" | "L" | "U" | "B")) {
        return if move_count <= 10 {
            WcaEvent::Skewb
        } else {
            WcaEvent::Pyraminx
        };
    }

    let has_three_wide = bases
        .iter()
        .any(|b| matches!(*b, "3Rw" | "3Lw" | "3Uw" | "3Dw" | "3Fw" | "3Bw"));
    if has_three_wide {
        return if move_count >= 90 {
            WcaEvent::Cube7x7
        } else {
            WcaEvent::Cube6x6
        };
    }

    let has_wide = bases
        .iter()
        .any(|b| matches!(*b, "Rw" | "Lw" | "Uw" | "Dw" | "Fw" | "Bw"));
    if has_wide {
        return if move_count >= 50 {
            WcaEvent::Cube5x5
        } else {
            WcaEvent::Cube4x4
        };
    }

    if bases.iter().all(|b| matches!(*b, "R" | "U" | "F")) {
        return WcaEvent::Cube2x2;
    }

    WcaEvent::Cube3x3
}

/// Removes a move's amount or direction suffix for classification.
fn base_move(token: &str) -> &str {
    token
        .strip_suffix('2')
        .unwrap_or_else(|| token.strip_suffix('\'').map_or(token, |stripped| stripped))
}

/// Returns whether a token resembles a clock dial turn.
fn is_clock_token(token: &str) -> bool {
    const POSITIONS: [&str; 9] = ["UR", "DR", "DL", "UL", "U", "R", "D", "L", "ALL"];
    for pos in POSITIONS {
        if let Some(rest) = token.strip_prefix(pos) {
            if rest.len() < 2 {
                continue;
            }
            let mut chars = rest.chars();
            let sign = chars.next().unwrap_or('+');
            if sign != '+' && sign != '-' {
                continue;
            }
            if chars.all(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{Scramble, WcaEvent, generate_scramble};

    #[test]
    fn scrambles_are_non_empty() {
        let events = [
            WcaEvent::Cube2x2,
            WcaEvent::Cube3x3,
            WcaEvent::Cube4x4,
            WcaEvent::Cube5x5,
            WcaEvent::Cube6x6,
            WcaEvent::Cube7x7,
            WcaEvent::Megaminx,
            WcaEvent::Pyraminx,
            WcaEvent::Fto,
            WcaEvent::Skewb,
            WcaEvent::Square1,
            WcaEvent::Clock,
        ];

        for event in events {
            let scramble = generate_scramble(event);
            assert!(
                !scramble.as_str().is_empty(),
                "{event:?} scramble was empty"
            );
        }
    }

    #[test]
    fn cube_scramble_lengths() {
        // (event, min/max move count)
        let cases: [(WcaEvent, usize, usize); 6] = [
            (WcaEvent::Cube2x2, 4, 14),
            (WcaEvent::Cube3x3, 4, 25),
            (WcaEvent::Cube4x4, 30, 55),
            (WcaEvent::Cube5x5, 40, 75),
            (WcaEvent::Cube6x6, 50, 100),
            (WcaEvent::Cube7x7, 60, 120),
        ];

        for _ in 0..10 {
            for (event, min, max) in cases {
                let scramble = generate_scramble(event);
                let count = scramble.as_str().split_whitespace().count();
                assert!(
                    (min..=max).contains(&count),
                    "{event:?} length {count} outside {min}-{max}"
                );
            }
        }
    }

    #[test]
    fn cube_3x3_uses_valid_moves() {
        let valid_bases = ["R", "L", "U", "D", "F", "B"];
        let valid_modifiers = ["", "'", "2"];

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Cube3x3);
            for token in scramble.as_str().split_whitespace() {
                let base = token.trim_end_matches(['\'', '2']);
                let modifier = &token[base.len()..];

                assert!(valid_bases.contains(&base), "Invalid move base: {base}");
                assert!(
                    valid_modifiers.contains(&modifier),
                    "Invalid modifier: {modifier}"
                );
            }
        }
    }

    #[test]
    fn cube_6x6_includes_wide_moves() {
        let mut found_3_wide = false;

        for _ in 0..20 {
            let scramble = generate_scramble(WcaEvent::Cube6x6);
            if scramble.as_str().contains("3Rw")
                || scramble.as_str().contains("3Lw")
                || scramble.as_str().contains("3Uw")
                || scramble.as_str().contains("3Dw")
                || scramble.as_str().contains("3Fw")
                || scramble.as_str().contains("3Bw")
            {
                found_3_wide = true;
                break;
            }
        }

        assert!(
            found_3_wide,
            "6x6 scrambles should include 3-layer wide moves"
        );
    }

    #[test]
    fn megaminx_uses_valid_moves() {
        let valid_moves = ["R++", "R--", "D++", "D--", "U", "U'"];

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Megaminx);
            for token in scramble.as_str().split_whitespace() {
                assert!(
                    valid_moves.contains(&token),
                    "Invalid megaminx move: {token}"
                );
            }
        }
    }

    #[test]
    fn pyraminx_base_length() {
        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Pyraminx);
            let count = scramble.as_str().split_whitespace().count();
            assert!(
                count >= 11,
                "Pyraminx should have at least 11 moves, got {count}"
            );
        }
    }

    #[test]
    fn skewb_length() {
        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Skewb);
            let count = scramble.as_str().split_whitespace().count();
            assert!(
                (4..=20).contains(&count),
                "Skewb length {count} outside 4-20"
            );
        }
    }

    #[test]
    fn square1_format() {
        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Square1);
            let text = scramble.as_str();

            // Should contain parentheses and slashes
            assert!(text.contains('('), "Square-1 should have parentheses");
            assert!(text.contains('/'), "Square-1 should have slashes");

            // Random-state Square-1 scrambles vary in length
            let token_count = text.split_whitespace().count();
            assert!(
                (4..=30).contains(&token_count),
                "Square-1 token count {token_count} outside 4-30"
            );
        }
    }

    #[test]
    fn clock_format() {
        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Clock);
            let text = scramble.as_str();

            // Should contain + or - for amounts
            assert!(
                text.contains('+') || text.contains('-'),
                "Clock should have +/- amounts"
            );

            // Two sections separated by a y2 rotation, ending in bare pin moves
            assert!(text.contains("y2"), "Clock should contain y2");
            let sections: Vec<&str> = text.split("y2").collect();
            assert_eq!(sections.len(), 2, "Clock should have 2 y2 sections");
            assert!(
                !sections[0].trim().is_empty() && !sections[1].trim().is_empty(),
                "Clock sections should not be empty"
            );
        }
    }

    #[test]
    fn scramble_display() {
        let scramble = Scramble::new("R U R' U'".to_string());
        assert_eq!(scramble.to_string(), "R U R' U'");
        assert_eq!(scramble.as_str(), "R U R' U'");
    }

    #[test]
    fn wca_event_name() {
        assert_eq!(WcaEvent::Cube3x3.name(), "3x3x3");
        assert_eq!(WcaEvent::Megaminx.name(), "Megaminx");
        assert_eq!(WcaEvent::Square1.name(), "Square-1");
    }

    #[test]
    fn wca_event_next_prev() {
        let events = [
            WcaEvent::Cube2x2,
            WcaEvent::Cube3x3,
            WcaEvent::Cube4x4,
            WcaEvent::Cube5x5,
            WcaEvent::Cube6x6,
            WcaEvent::Cube7x7,
            WcaEvent::Megaminx,
            WcaEvent::Pyraminx,
            WcaEvent::Fto,
            WcaEvent::Skewb,
            WcaEvent::Square1,
            WcaEvent::Clock,
        ];

        for (index, event) in events.iter().copied().enumerate() {
            let next = events[(index + 1) % events.len()];
            let prev = events[(index + events.len() - 1) % events.len()];
            assert_eq!(event.next(), next, "unexpected next event for {event:?}");
            assert_eq!(
                event.prev(),
                prev,
                "unexpected previous event for {event:?}"
            );
        }
    }

    #[test]
    fn fto_scramble_uses_valid_moves_and_length() {
        let valid_bases = ["U", "R", "F", "L", "B", "BL", "D", "BR"];
        let valid_modifiers = ["", "'"];

        for _ in 0..20 {
            let scramble = generate_scramble(WcaEvent::Fto);

            let tokens: Vec<&str> = scramble.as_str().split_whitespace().collect();
            // Random-state scrambles vary in length.
            let lengths = 10..50;
            assert!(
                lengths.contains(&tokens.len()),
                "FTO length {} outside {lengths:?}",
                tokens.len()
            );

            for token in tokens {
                let base = token.trim_end_matches('\'');
                let modifier = &token[base.len()..];
                assert!(valid_bases.contains(&base), "invalid FTO move: {base}");
                assert!(
                    valid_modifiers.contains(&modifier),
                    "invalid FTO modifier: {modifier}"
                );
            }
        }
    }

    #[test]
    fn cube_2x2_uses_valid_moves() {
        let valid_bases = ["R", "U", "F"];
        let valid_modifiers = ["", "'", "2"];

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Cube2x2);
            for token in scramble.as_str().split_whitespace() {
                let base = token.trim_end_matches(['\'', '2']);
                let modifier = &token[base.len()..];

                assert!(valid_bases.contains(&base), "Invalid 2x2 move base: {base}");
                assert!(
                    valid_modifiers.contains(&modifier),
                    "Invalid modifier: {modifier}"
                );
            }
        }
    }

    #[test]
    fn cube_4x4_uses_valid_moves() {
        let valid_bases = [
            "R", "L", "U", "D", "F", "B", "Rw", "Lw", "Uw", "Dw", "Fw", "Bw",
        ];
        let valid_modifiers = ["", "'", "2"];

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Cube4x4);
            for token in scramble.as_str().split_whitespace() {
                let base = token.trim_end_matches(['\'', '2']);
                let modifier = &token[base.len()..];

                assert!(valid_bases.contains(&base), "Invalid 4x4 move base: {base}");
                assert!(
                    valid_modifiers.contains(&modifier),
                    "Invalid modifier: {modifier}"
                );
            }
        }
    }

    #[test]
    fn cube_7x7_uses_valid_moves() {
        let valid_bases = [
            "R", "L", "U", "D", "F", "B", "Rw", "Lw", "Uw", "Dw", "Fw", "Bw", "3Rw", "3Lw", "3Uw",
            "3Dw", "3Fw", "3Bw",
        ];
        let valid_modifiers = ["", "'", "2"];

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Cube7x7);
            for token in scramble.as_str().split_whitespace() {
                let base = token.trim_end_matches(['\'', '2']);
                let modifier = &token[base.len()..];

                assert!(valid_bases.contains(&base), "Invalid 7x7 move base: {base}");
                assert!(
                    valid_modifiers.contains(&modifier),
                    "Invalid modifier: {modifier}"
                );
            }
        }
    }

    #[test]
    fn pyraminx_uses_valid_moves() {
        let valid_bases = ["R", "L", "U", "B", "r", "l", "u", "b"];
        let valid_modifiers = ["", "'"];

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Pyraminx);
            for token in scramble.as_str().split_whitespace() {
                let base = token.trim_end_matches('\'');
                let modifier = &token[base.len()..];

                assert!(valid_bases.contains(&base), "Invalid pyraminx move: {base}");
                assert!(
                    valid_modifiers.contains(&modifier),
                    "Invalid pyraminx modifier: {modifier}"
                );
            }
        }
    }

    #[test]
    fn skewb_uses_valid_moves() {
        let valid_bases = ["R", "L", "U", "B"];
        let valid_modifiers = ["", "'"];

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Skewb);
            for token in scramble.as_str().split_whitespace() {
                let base = token.trim_end_matches('\'');
                let modifier = &token[base.len()..];

                assert!(valid_bases.contains(&base), "Invalid skewb move: {base}");
                assert!(
                    valid_modifiers.contains(&modifier),
                    "Invalid skewb modifier: {modifier}"
                );
            }
        }
    }

    #[test]
    fn all_event_names_unique() {
        let events = [
            WcaEvent::Cube2x2,
            WcaEvent::Cube3x3,
            WcaEvent::Cube4x4,
            WcaEvent::Cube5x5,
            WcaEvent::Cube6x6,
            WcaEvent::Cube7x7,
            WcaEvent::Megaminx,
            WcaEvent::Pyraminx,
            WcaEvent::Fto,
            WcaEvent::Skewb,
            WcaEvent::Square1,
            WcaEvent::Clock,
        ];

        let names: Vec<&str> = events.iter().map(|e| e.name()).collect();
        let mut unique_names = names.clone();
        unique_names.sort_unstable();
        unique_names.dedup();

        assert_eq!(
            names.len(),
            unique_names.len(),
            "Event names should be unique"
        );
    }

    #[test]
    fn event_cycle_complete() {
        let start = WcaEvent::Cube2x2;
        let mut current = start.next();
        let mut count = 1;

        while current != start && count < 20 {
            current = current.next();
            count += 1;
        }

        assert_eq!(count, 12, "Should cycle through all 12 events");
    }

    #[test]
    fn megaminx_scramble_length() {
        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Megaminx);
            let count = scramble.as_str().split_whitespace().count();
            assert_eq!(
                count, 77,
                "Megaminx should have 77 moves (7 rows × 11), got {count}"
            );
        }
    }

    #[test]
    fn square1_move_values_in_range() {
        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Square1);

            for part in scramble.as_str().split_whitespace() {
                if part.starts_with('(') && part.ends_with(')') {
                    let inner = &part[1..part.len() - 1];
                    let nums: Vec<&str> = inner.split(',').collect();
                    assert_eq!(nums.len(), 2, "Square-1 move should have 2 values");

                    for num_str in nums {
                        let num: i8 = num_str.parse().expect("Should parse as number");
                        assert!(
                            (-6..=6).contains(&num),
                            "Square-1 value {num} outside -6..=6"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn clock_positions_valid() {
        const POSITIONS: [&str; 9] = ["UR", "DR", "DL", "UL", "U", "R", "D", "L", "ALL"];

        // Longest first so e.g. "UR" is matched before "U"
        let mut prefixes: Vec<&str> = POSITIONS.to_vec();
        prefixes.sort_by_key(|pos| std::cmp::Reverse(pos.len()));

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Clock);

            for token in scramble.as_str().split_whitespace() {
                if token == "y2" {
                    continue;
                }

                // Primed pin move, e.g. "UR'"
                if let Some(pin) = token.strip_suffix('\'') {
                    assert!(prefixes.contains(&pin), "Invalid clock pin move: {token}");
                    continue;
                }

                let Some(rest) = prefixes.iter().find_map(|pos| token.strip_prefix(pos)) else {
                    panic!("Invalid clock position in token: {token}");
                };

                if rest.is_empty() {
                    continue;
                }

                // Amounts have magnitude 0-6 with a direction sign
                let magnitude = parse_clock_amount(rest)
                    .unwrap_or_else(|| panic!("Invalid clock amount in token: {token}"));
                assert!(
                    (0..=6).contains(&magnitude),
                    "Clock magnitude {magnitude} out of range in token: {token}"
                );
            }
        }
    }

    /// Parses a "3+"/"3-" clock amount into its magnitude.
    fn parse_clock_amount(rest: &str) -> Option<i8> {
        rest.strip_suffix(['+', '-'])?.parse().ok()
    }

    #[test]
    fn scramble_length_bounds() {
        // Random-state scrambles vary within competition bounds.
        let cases: [(WcaEvent, usize, usize); 8] = [
            (WcaEvent::Cube2x2, 4, 14),
            (WcaEvent::Cube3x3, 4, 25),
            (WcaEvent::Cube4x4, 30, 55),
            (WcaEvent::Cube5x5, 40, 75),
            (WcaEvent::Cube6x6, 50, 100),
            (WcaEvent::Cube7x7, 60, 120),
            (WcaEvent::Megaminx, 77, 77),
            (WcaEvent::Skewb, 4, 20),
        ];

        for (event, min, max) in cases {
            for _ in 0..5 {
                let scramble = generate_scramble(event);
                let count = scramble.as_str().split_whitespace().count();
                assert!(
                    (min..=max).contains(&count),
                    "{event:?} length {count} outside {min}-{max}"
                );
            }
        }
    }
}
