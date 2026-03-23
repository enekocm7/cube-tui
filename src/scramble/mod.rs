use rand::RngExt;
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::fmt;

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
    Skewb,
    Square1,
    Clock,
}

impl WcaEvent {
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
            Self::Skewb => "Skewb",
            Self::Square1 => "Square-1",
            Self::Clock => "Clock",
        }
    }

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
            Self::Skewb => 8,
            Self::Square1 => 9,
            Self::Clock => 10,
        }
    }

    const fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Cube2x2,
            1 => Self::Cube3x3,
            2 => Self::Cube4x4,
            3 => Self::Cube5x5,
            4 => Self::Cube6x6,
            5 => Self::Cube7x7,
            6 => Self::Megaminx,
            7 => Self::Pyraminx,
            8 => Self::Skewb,
            9 => Self::Square1,
            _ => Self::Clock,
        }
    }

    pub const fn next(self) -> Self {
        let index = self.as_index();
        let next_index = (index + 1) % 11;
        Self::from_index(next_index)
    }

    pub const fn prev(self) -> Self {
        let index = self.as_index();
        let prev_index = if index == 0 { 10 } else { index - 1 };
        Self::from_index(prev_index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    R,
    L,
    U,
    D,
    F,
    B,
    Rw,
    Lw,
    Uw,
    Dw,
    Fw,
    Bw,
    ThreeRw,
    ThreeLw,
    ThreeUw,
    ThreeDw,
    ThreeFw,
    ThreeBw,
    RDoublePlus,
    RDoubleMinus,
    DDoublePlus,
    DDoubleMinus,
    SmallR,
    SmallL,
    SmallU,
    SmallB,
}

impl Move {
    pub const fn axis(self) -> u8 {
        match self {
            Self::R | Self::L | Self::Rw | Self::Lw | Self::ThreeRw | Self::ThreeLw => 0,
            Self::U | Self::D | Self::Uw | Self::Dw | Self::ThreeUw | Self::ThreeDw => 1,
            Self::F | Self::B | Self::Fw | Self::Bw | Self::ThreeFw | Self::ThreeBw => 2,
            Self::RDoublePlus | Self::RDoubleMinus => 3,
            Self::DDoublePlus | Self::DDoubleMinus => 4,
            Self::SmallR | Self::SmallL | Self::SmallU | Self::SmallB => 5,
        }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::R => "R",
            Self::L => "L",
            Self::U => "U",
            Self::D => "D",
            Self::F => "F",
            Self::B => "B",
            Self::Rw => "Rw",
            Self::Lw => "Lw",
            Self::Uw => "Uw",
            Self::Dw => "Dw",
            Self::Fw => "Fw",
            Self::Bw => "Bw",
            Self::ThreeRw => "3Rw",
            Self::ThreeLw => "3Lw",
            Self::ThreeUw => "3Uw",
            Self::ThreeDw => "3Dw",
            Self::ThreeFw => "3Fw",
            Self::ThreeBw => "3Bw",
            Self::RDoublePlus => "R++",
            Self::RDoubleMinus => "R--",
            Self::DDoublePlus => "D++",
            Self::DDoubleMinus => "D--",
            Self::SmallR => "r",
            Self::SmallL => "l",
            Self::SmallU => "u",
            Self::SmallB => "b",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    None,
    Prime,
    Double,
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::None => "",
            Self::Prime => "'",
            Self::Double => "2",
        };
        f.write_str(s)
    }
}

pub struct Scramble {
    text: String,
}

impl Scramble {
    pub const fn new(text: String) -> Self {
        Self { text }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for Scramble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

pub fn generate_scramble(event: WcaEvent) -> Scramble {
    let text = match event {
        WcaEvent::Cube2x2 => cube_scramble(10, &cube_2x2_moves(), &cube_modifiers()),
        WcaEvent::Cube3x3 => cube_scramble(20, &cube_3x3_moves(), &cube_modifiers()),
        WcaEvent::Cube4x4 => cube_scramble(40, &cube_4x4_moves(), &cube_modifiers()),
        WcaEvent::Cube5x5 => cube_scramble(60, &cube_5x5_moves(), &cube_modifiers()),
        WcaEvent::Cube6x6 => cube_scramble(80, &cube_6x6_moves(), &cube_modifiers()),
        WcaEvent::Cube7x7 => cube_scramble(100, &cube_7x7_moves(), &cube_modifiers()),
        WcaEvent::Megaminx => megaminx_scramble(),
        WcaEvent::Pyraminx => pyraminx_scramble(11),
        WcaEvent::Skewb => skewb_scramble(9),
        WcaEvent::Square1 => square1_scramble(15),
        WcaEvent::Clock => clock_scramble(14),
    };

    Scramble::new(text)
}

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

fn base_move(token: &str) -> &str {
    token
        .strip_suffix('2')
        .unwrap_or_else(|| token.strip_suffix('\'').map_or(token, |stripped| stripped))
}

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

fn cube_2x2_moves() -> Vec<Move> {
    vec![Move::R, Move::U, Move::F]
}

fn cube_3x3_moves() -> Vec<Move> {
    vec![Move::R, Move::L, Move::U, Move::D, Move::F, Move::B]
}

fn cube_4x4_moves() -> Vec<Move> {
    vec![
        Move::R,
        Move::L,
        Move::U,
        Move::D,
        Move::F,
        Move::B,
        Move::Rw,
        Move::Lw,
        Move::Uw,
        Move::Dw,
        Move::Fw,
        Move::Bw,
    ]
}

fn cube_5x5_moves() -> Vec<Move> {
    vec![
        Move::R,
        Move::L,
        Move::U,
        Move::D,
        Move::F,
        Move::B,
        Move::Rw,
        Move::Lw,
        Move::Uw,
        Move::Dw,
        Move::Fw,
        Move::Bw,
    ]
}

fn cube_6x6_moves() -> Vec<Move> {
    vec![
        Move::R,
        Move::L,
        Move::U,
        Move::D,
        Move::F,
        Move::B,
        Move::Rw,
        Move::Lw,
        Move::Uw,
        Move::Dw,
        Move::Fw,
        Move::Bw,
        Move::ThreeRw,
        Move::ThreeLw,
        Move::ThreeUw,
        Move::ThreeDw,
        Move::ThreeFw,
        Move::ThreeBw,
    ]
}

fn cube_7x7_moves() -> Vec<Move> {
    cube_6x6_moves()
}

fn cube_modifiers() -> Vec<Modifier> {
    vec![Modifier::None, Modifier::Prime, Modifier::Double]
}

fn pyraminx_modifiers() -> Vec<Modifier> {
    vec![Modifier::None, Modifier::Prime]
}

fn cube_scramble(length: usize, moves: &[Move], modifiers: &[Modifier]) -> String {
    let mut rng = rand::rng();
    let mut last_move: Option<Move> = None;
    let mut last_axis: Option<u8> = None;
    let mut parts = Vec::with_capacity(length);

    while parts.len() < length {
        let mv = *moves
            .choose(&mut rng)
            .expect("moves list should not be empty");
        if Some(mv) == last_move || Some(mv.axis()) == last_axis {
            continue;
        }

        let modifier = *modifiers.choose(&mut rng).unwrap_or(&Modifier::None);
        parts.push(format!("{mv}{modifier}"));
        last_move = Some(mv);
        last_axis = Some(mv.axis());
    }

    parts.join(" ")
}

fn megaminx_scramble() -> String {
    let mut rng = rand::rng();
    let r_moves = [Move::RDoublePlus, Move::RDoubleMinus];
    let d_moves = [Move::DDoublePlus, Move::DDoubleMinus];
    let u_modifiers = [Modifier::None, Modifier::Prime];

    let mut rows = Vec::with_capacity(7);
    for _ in 0..7 {
        let mut parts = Vec::with_capacity(11);
        for _ in 0..5 {
            let r = r_moves.choose(&mut rng).unwrap_or(&Move::RDoublePlus);
            let d = d_moves.choose(&mut rng).unwrap_or(&Move::DDoublePlus);
            parts.push(r.to_string());
            parts.push(d.to_string());
        }
        let u_mod = u_modifiers.choose(&mut rng).unwrap_or(&Modifier::None);
        parts.push(format!("U{u_mod} "));
        rows.push(parts.join(" "));
    }
    rows.join("\n")
}

fn simple_scramble(length: usize, moves: &[Move], modifiers: &[Modifier]) -> String {
    let mut rng = rand::rng();
    let mut parts = Vec::with_capacity(length);

    for _ in 0..length {
        let mv = moves
            .choose(&mut rng)
            .expect("moves list should not be empty");
        let modifier = modifiers.choose(&mut rng).unwrap_or(&Modifier::None);
        parts.push(format!("{mv}{modifier}"));
    }

    parts.join(" ")
}

fn pyraminx_scramble(length: usize) -> String {
    let mut rng = rand::rng();
    let moves = [Move::R, Move::L, Move::U, Move::B];
    let modifiers = pyraminx_modifiers();

    let mut base = simple_scramble(length, &moves, &modifiers);

    let tips = [Move::SmallR, Move::SmallL, Move::SmallU, Move::SmallB];
    let mut tip_parts = Vec::new();
    for tip in tips {
        if rng.random_bool(0.5) {
            let modifier = modifiers.choose(&mut rng).unwrap_or(&Modifier::None);
            tip_parts.push(format!("{tip}{modifier}"));
        }
    }

    if !tip_parts.is_empty() {
        base.push(' ');
        base.push_str(&tip_parts.join(" "));
    }

    base
}

fn skewb_scramble(length: usize) -> String {
    let moves = [Move::R, Move::L, Move::U, Move::B];
    simple_scramble(length, &moves, &pyraminx_modifiers())
}

fn square1_scramble(length: usize) -> String {
    let mut rng = rand::rng();
    let mut parts = Vec::with_capacity(length * 2);
    for _ in 0..length {
        let (a, b) = loop {
            let a = rng.random_range(-5..=6);
            let b = rng.random_range(-5..=6);
            if a != 0 || b != 0 {
                break (a, b);
            }
        };
        parts.push(format!("({a},{b})"));
        parts.push("/".to_string());
    }
    parts.join(" ")
}

fn clock_scramble(length: usize) -> String {
    let mut rng = rand::rng();
    let positions = ["UR", "DR", "DL", "UL", "U", "R", "D", "L", "ALL"];
    let mut parts = Vec::with_capacity(length + 2);
    for _ in 0..length {
        let pos = positions
            .choose(&mut rng)
            .expect("positions list should not be empty");
        let amount: i8 = rng.random_range(-5..=6);
        parts.push(format!("{pos}{amount:+}"));
    }
    parts.push("y2".to_string());
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::{Modifier, Move, Scramble, WcaEvent, generate_scramble};

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
        for _ in 0..10 {
            let s2 = generate_scramble(WcaEvent::Cube2x2);
            let s3 = generate_scramble(WcaEvent::Cube3x3);
            let s4 = generate_scramble(WcaEvent::Cube4x4);
            let s5 = generate_scramble(WcaEvent::Cube5x5);
            let s6 = generate_scramble(WcaEvent::Cube6x6);
            let s7 = generate_scramble(WcaEvent::Cube7x7);

            assert_eq!(
                s2.as_str().split_whitespace().count(),
                10,
                "2x2 should have 10 moves"
            );
            assert_eq!(
                s3.as_str().split_whitespace().count(),
                20,
                "3x3 should have 20 moves"
            );
            assert_eq!(
                s4.as_str().split_whitespace().count(),
                40,
                "4x4 should have 40 moves"
            );
            assert_eq!(
                s5.as_str().split_whitespace().count(),
                60,
                "5x5 should have 60 moves"
            );
            assert_eq!(
                s6.as_str().split_whitespace().count(),
                80,
                "6x6 should have 80 moves"
            );
            assert_eq!(
                s7.as_str().split_whitespace().count(),
                100,
                "7x7 should have 100 moves"
            );
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
            assert_eq!(count, 9, "Skewb should have 9 moves, got {count}");
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

            // Count slashes - should be 15
            let slash_count = text.matches('/').count();
            assert_eq!(
                slash_count, 15,
                "Square-1 should have 15 slashes, got {slash_count}"
            );
        }
    }

    #[test]
    fn clock_format() {
        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Clock);
            let text = scramble.as_str();

            // Should end with y2
            assert!(text.ends_with("y2"), "Clock should end with y2");

            // Should contain + or - for amounts
            assert!(
                text.contains('+') || text.contains('-'),
                "Clock should have +/- amounts"
            );
        }
    }

    #[test]
    fn move_display() {
        assert_eq!(Move::R.to_string(), "R");
        assert_eq!(Move::Rw.to_string(), "Rw");
        assert_eq!(Move::ThreeRw.to_string(), "3Rw");
        assert_eq!(Move::RDoublePlus.to_string(), "R++");
        assert_eq!(Move::SmallR.to_string(), "r");
    }

    #[test]
    fn modifier_display() {
        assert_eq!(Modifier::None.to_string(), "");
        assert_eq!(Modifier::Prime.to_string(), "'");
        assert_eq!(Modifier::Double.to_string(), "2");
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
        assert_eq!(WcaEvent::Cube2x2.next(), WcaEvent::Cube3x3);
        assert_eq!(WcaEvent::Clock.next(), WcaEvent::Cube2x2);
        assert_eq!(WcaEvent::Cube2x2.prev(), WcaEvent::Clock);
        assert_eq!(WcaEvent::Cube3x3.prev(), WcaEvent::Cube2x2);
    }

    #[test]
    fn move_axis() {
        // Same axis moves
        assert_eq!(Move::R.axis(), Move::L.axis());
        assert_eq!(Move::U.axis(), Move::D.axis());
        assert_eq!(Move::F.axis(), Move::B.axis());

        // Wide moves same axis as base
        assert_eq!(Move::R.axis(), Move::Rw.axis());
        assert_eq!(Move::Rw.axis(), Move::ThreeRw.axis());

        // Different axes
        assert_ne!(Move::R.axis(), Move::U.axis());
        assert_ne!(Move::U.axis(), Move::F.axis());
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
    fn cube_scrambles_no_consecutive_same_axis() {
        fn parse_axis(token: &str) -> u8 {
            let base = token.trim_end_matches(['\'', '2']);
            match base {
                "R" | "L" | "Rw" | "Lw" | "3Rw" | "3Lw" => 0,
                "U" | "D" | "Uw" | "Dw" | "3Uw" | "3Dw" => 1,
                "F" | "B" | "Fw" | "Bw" | "3Fw" | "3Bw" => 2,
                _ => 255, // Unknown
            }
        }

        let cube_events = [
            WcaEvent::Cube2x2,
            WcaEvent::Cube3x3,
            WcaEvent::Cube4x4,
            WcaEvent::Cube5x5,
            WcaEvent::Cube6x6,
            WcaEvent::Cube7x7,
        ];

        for event in cube_events {
            for _ in 0..5 {
                let scramble = generate_scramble(event);
                let tokens: Vec<&str> = scramble.as_str().split_whitespace().collect();

                for i in 1..tokens.len() {
                    let prev_axis = parse_axis(tokens[i - 1]);
                    let curr_axis = parse_axis(tokens[i]);

                    assert_ne!(
                        prev_axis,
                        curr_axis,
                        "{:?}: consecutive same-axis moves {} and {}",
                        event,
                        tokens[i - 1],
                        tokens[i]
                    );
                }
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

        assert_eq!(count, 11, "Should cycle through all 11 events");
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
                        assert!((-5..=6).contains(&num), "Square-1 value {num} out of range");
                    }
                }
            }
        }
    }

    #[test]
    fn clock_positions_valid() {
        let valid_positions = ["UR", "DR", "DL", "UL", "U", "R", "D", "L", "ALL"];

        for _ in 0..10 {
            let scramble = generate_scramble(WcaEvent::Clock);
            let tokens: Vec<&str> = scramble.as_str().split_whitespace().collect();

            // All but the last should be position+amount
            for token in &tokens[..tokens.len() - 1] {
                // Find where the number starts (+ or - or digit)
                let pos_end = token.find(|c: char| c == '+' || c == '-' || c.is_ascii_digit());
                if let Some(idx) = pos_end {
                    let pos = &token[..idx];
                    assert!(
                        valid_positions.contains(&pos),
                        "Invalid clock position: {pos}"
                    );
                }
            }

            // Last should be y2
            assert_eq!(*tokens.last().unwrap(), "y2", "Clock should end with y2");
        }
    }

    #[test]
    fn scramble_deterministic_length() {
        // The same event should always produce the same length scrambles
        let lengths: Vec<(WcaEvent, usize)> = vec![
            (WcaEvent::Cube2x2, 10),
            (WcaEvent::Cube3x3, 20),
            (WcaEvent::Cube4x4, 40),
            (WcaEvent::Cube5x5, 60),
            (WcaEvent::Cube6x6, 80),
            (WcaEvent::Cube7x7, 100),
            (WcaEvent::Megaminx, 77),
            (WcaEvent::Skewb, 9),
        ];

        for (event, expected_len) in lengths {
            for _ in 0..5 {
                let scramble = generate_scramble(event);
                let count = scramble.as_str().split_whitespace().count();
                assert_eq!(count, expected_len, "{event:?} length mismatch");
            }
        }
    }
}
