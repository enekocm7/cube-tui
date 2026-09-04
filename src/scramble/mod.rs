use rand::RngExt;
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

#[cfg(feature = "wca-scrambles")]
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
            1 | _ => Self::Cube3x3,
        }
    }

    pub const fn next(self) -> Self {
        let index = self.as_index();
        let next_index = (index + 1) % 12;
        Self::from_index(next_index)
    }

    pub const fn prev(self) -> Self {
        let index = self.as_index();
        let prev_index = if index == 0 { 11 } else { index - 1 };
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
    Br,
    Bl,
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
            Self::R
            | Self::L
            | Self::Rw
            | Self::Lw
            | Self::ThreeRw
            | Self::ThreeLw
            | Self::Br
            | Self::Bl => 0,
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
            Self::Bl => "Bl",
            Self::Br => "Br",
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
    text: Cow<'static, str>,
    wca: bool,
}

impl Scramble {
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: text.into(),
            wca: false,
        }
    }

    pub fn new_wca(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: text.into(),
            wca: true,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub const fn is_wca(&self) -> bool {
        self.wca
    }
}

impl fmt::Display for Scramble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<Scramble> for Cow<'static, str> {
    fn from(scramble: Scramble) -> Self {
        scramble.text
    }
}

pub fn generate_scramble(event: WcaEvent) -> Scramble {
    //Temporary fix until the official WCA scrambler supports FTO event
    if event == WcaEvent::Fto {
        return Scramble::new(random_scramble(event));
    }

    #[cfg(feature = "wca-scrambles")]
    if let Some(text) = wca::get_wca_scramble(event) {
        return Scramble::new_wca(text);
    }

    let text = random_scramble(event);

    Scramble::new(text)
}

fn random_scramble(event: WcaEvent) -> String {
    match event {
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
        WcaEvent::Fto => fto_scramble(rand::random_range(25..30)),
    }
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
fn fto_moves() -> Vec<Move> {
    vec![
        Move::R,
        Move::L,
        Move::B,
        Move::D,
        Move::F,
        Move::Br,
        Move::Bl,
    ]
}

fn cube_modifiers() -> Vec<Modifier> {
    vec![Modifier::None, Modifier::Prime, Modifier::Double]
}

fn pyraminx_modifiers() -> Vec<Modifier> {
    vec![Modifier::None, Modifier::Prime]
}

fn fto_modifiers() -> Vec<Modifier> {
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

fn fto_scramble(length: usize) -> String {
    let moves = fto_moves();
    let modifiers = fto_modifiers();
    cube_scramble(length, &moves, &modifiers)
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
        // (event, built-in exact length, WCA min/max move count)
        let cases: [(WcaEvent, usize, usize, usize); 6] = [
            (WcaEvent::Cube2x2, 10, 4, 14),
            (WcaEvent::Cube3x3, 20, 4, 25),
            (WcaEvent::Cube4x4, 40, 30, 55),
            (WcaEvent::Cube5x5, 60, 40, 75),
            (WcaEvent::Cube6x6, 80, 50, 100),
            (WcaEvent::Cube7x7, 100, 60, 120),
        ];

        for _ in 0..10 {
            for (event, internal_len, wca_min, wca_max) in cases {
                let scramble = generate_scramble(event);
                let count = scramble.as_str().split_whitespace().count();
                if scramble.is_wca() {
                    assert!(
                        (wca_min..=wca_max).contains(&count),
                        "{event:?} WCA length {count} outside {wca_min}-{wca_max}"
                    );
                } else {
                    assert_eq!(
                        count, internal_len,
                        "{event:?} should have {internal_len} moves"
                    );
                }
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
            if scramble.is_wca() {
                assert!(
                    (4..=20).contains(&count),
                    "WCA skewb length {count} outside 4-20"
                );
            } else {
                assert_eq!(count, 9, "Skewb should have 9 moves, got {count}");
            }
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

            let slash_count = text.matches('/').count();
            if scramble.is_wca() {
                // WCA Square-1 scrambles vary in length
                let token_count = text.split_whitespace().count();
                assert!(
                    (4..=30).contains(&token_count),
                    "WCA Square-1 token count {token_count} outside 4-30"
                );
            } else {
                // Built-in generator emits exactly 15 twist/slash pairs
                assert_eq!(
                    slash_count, 15,
                    "Square-1 should have 15 slashes, got {slash_count}"
                );
            }
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

            if scramble.is_wca() {
                // WCA clock: two sections separated by a y2 rotation,
                // ending in bare pin moves
                assert!(text.contains("y2"), "WCA clock should contain y2");
                let sections: Vec<&str> = text.split("y2").collect();
                assert_eq!(sections.len(), 2, "WCA clock should have 2 y2 sections");
                assert!(
                    !sections[0].trim().is_empty() && !sections[1].trim().is_empty(),
                    "WCA clock sections should not be empty"
                );
            } else {
                // Built-in clock ends with y2
                assert!(text.ends_with("y2"), "Clock should end with y2");
            }
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
        let valid_bases = ["R", "L", "B", "D", "F", "Br", "Bl"];
        let valid_modifiers = ["", "'"];

        for _ in 0..20 {
            let scramble = generate_scramble(WcaEvent::Fto);
            assert!(
                !scramble.is_wca(),
                "FTO currently uses the built-in generator"
            );

            let tokens: Vec<&str> = scramble.as_str().split_whitespace().collect();
            assert!(
                (25..30).contains(&tokens.len()),
                "FTO length {} outside 25-29",
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
                if scramble.is_wca() {
                    // WCA scrambles legitimately contain consecutive same-axis moves
                    continue;
                }
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
                        // WCA Square-1 coordinates span -6..6, built-in uses -5..6
                        let range = if scramble.is_wca() { -6..=6 } else { -5..=6 };
                        assert!(
                            range.contains(&num),
                            "Square-1 value {num} outside {range:?}"
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

                // Primed pin move (WCA-only), e.g. "UR'"
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

                let (magnitude, negative) = parse_clock_amount(rest)
                    .unwrap_or_else(|| panic!("Invalid clock amount in token: {token}"));
                if scramble.is_wca() {
                    // WCA amounts have magnitude 0-6 with a direction sign
                    assert!(
                        (0..=6).contains(&magnitude),
                        "Clock magnitude {magnitude} out of range in token: {token}"
                    );
                } else {
                    // Built-in amounts span -5 to 6
                    let amount = if negative { -magnitude } else { magnitude };
                    assert!(
                        (-5..=6).contains(&amount),
                        "Clock amount {amount} out of range in token: {token}"
                    );
                }
            }
        }
    }

    /// Parses "+3"/"-3" (built-in) or "3+"/"3-" (WCA) into (magnitude, negative).
    fn parse_clock_amount(rest: &str) -> Option<(i8, bool)> {
        if let Some(digits) = rest.strip_prefix('+') {
            return Some((digits.parse::<i8>().ok()?, false));
        }
        if let Some(digits) = rest.strip_prefix('-') {
            return Some((digits.parse::<i8>().ok()?, true));
        }
        if let Some(digits) = rest.strip_suffix('+') {
            return Some((digits.parse::<i8>().ok()?, false));
        }
        let digits = rest.strip_suffix('-')?;
        Some((digits.parse::<i8>().ok()?, true))
    }

    #[test]
    fn scramble_deterministic_length() {
        // The built-in generator produces a fixed length per event;
        // WCA scrambles vary within competition bounds.
        let cases: [(WcaEvent, usize, usize, usize); 8] = [
            (WcaEvent::Cube2x2, 10, 4, 14),
            (WcaEvent::Cube3x3, 20, 4, 25),
            (WcaEvent::Cube4x4, 40, 30, 55),
            (WcaEvent::Cube5x5, 60, 40, 75),
            (WcaEvent::Cube6x6, 80, 50, 100),
            (WcaEvent::Cube7x7, 100, 60, 120),
            (WcaEvent::Megaminx, 77, 77, 77),
            (WcaEvent::Skewb, 9, 4, 20),
        ];

        for (event, internal_len, wca_min, wca_max) in cases {
            for _ in 0..5 {
                let scramble = generate_scramble(event);
                let count = scramble.as_str().split_whitespace().count();
                if scramble.is_wca() {
                    assert!(
                        (wca_min..=wca_max).contains(&count),
                        "{event:?} WCA length {count} outside {wca_min}-{wca_max}"
                    );
                } else {
                    assert_eq!(count, internal_len, "{event:?} length mismatch");
                }
            }
        }
    }
}
