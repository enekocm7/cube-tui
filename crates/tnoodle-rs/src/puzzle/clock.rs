//! Rubik's Clock (`ClockPuzzle`).

use std::fmt::Write as _;

use crate::java::{self, RandomSource, array_hash};
use crate::scrambles::{ColorScheme, Puzzle, PuzzleState, PuzzleStateAndGenerator};
use crate::svg::{Color, Dimension, Element, Svg, Transform};

const TURNS: [&str; 9] = ["UR", "DR", "DL", "UL", "U", "R", "D", "L", "ALL"];
const STROKE_WIDTH: i32 = 2;
const FACE_STROKE_WIDTH: i32 = 1;
const RADIUS: i32 = 70;
const CLOCK_RADIUS: i32 = 14;
const CLOCK_OUTER_RADIUS: i32 = 21;
const POINT_RADIUS: i32 = CLOCK_RADIUS.midpoint(CLOCK_OUTER_RADIUS);
const TICK_MARK_RADIUS: i32 = 1;
const TOP_TICK_MARK_RADIUS: i32 = 2;
const ARROW_HEIGHT: i32 = 10;
const ARROW_RADIUS: i32 = 2;
const PIN_RADIUS: i32 = 4;
const GAP: i32 = 5;

/// Which dials each pin configuration turns: the first 9 values are the front dials, the
/// last 9 the back dials (which turn the other way).
const MOVES: [[i32; 18]; 9] = [
    [0, 1, 1, 0, 1, 1, 0, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0], // UR
    [0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, -1, 0, 0], // DR
    [0, 0, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1], // DL
    [1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, -1, 0, 0, 0, 0, 0, 0], // UL
    [1, 1, 1, 1, 1, 1, 0, 0, 0, -1, 0, -1, 0, 0, 0, 0, 0, 0], // U
    [0, 1, 1, 0, 1, 1, 0, 1, 1, -1, 0, 0, 0, 0, 0, -1, 0, 0], // R
    [0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, -1, 0, -1], // D
    [1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 0, -1, 0, 0, 0, 0, 0, -1], // L
    [1, 1, 1, 1, 1, 1, 1, 1, 1, -1, 0, -1, 0, 0, 0, -1, 0, -1], // ALL
];

static DEFAULT_COLOR_SCHEME: [(&str, Color); 12] = [
    ("Front", Color::BLUE_DEEP),
    ("FrontClock", Color::BLUE_BRIGHT),
    ("FrontTopClock", Color::YELLOW_SUNFLOWER),
    ("FrontHand", Color::BLUE_DEEP),
    ("FrontHandBorder", Color::BLUE_DEEP),
    ("FrontPin", Color::BLUE_ICE),
    ("Back", Color::BLUE_BRIGHT),
    ("BackClock", Color::BLUE_DEEP),
    ("BackTopClock", Color::ORANGE_BRONZE),
    ("BackHand", Color::BLUE_BRIGHT),
    ("BackHandBorder", Color::BLUE_BRIGHT),
    ("BackPin", Color::BLUE_ASPHALT),
];

/// Rubik's Clock. Scrambles are WCA style pin sequences such as `UR5- DR1- ... y2 U3- ...`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClockPuzzle;

impl ClockPuzzle {
    /// Creates the puzzle.
    pub const fn new() -> Self {
        Self
    }
}

fn append_turns(scramble: &mut String, turns: &[&str], r: &mut dyn RandomSource) {
    for turn in turns {
        let amount = r.next_int_bounded(12) - 5;
        let sign = if amount >= 0 { '+' } else { '-' };
        let _ = write!(scramble, "{turn}{}{sign} ", amount.abs());
    }
}

impl Puzzle for ClockPuzzle {
    type State = ClockState;

    fn short_name(&self) -> &str {
        "clock"
    }

    fn long_name(&self) -> &str {
        "Clock"
    }

    fn wca_min_scramble_distance(&self) -> i32 {
        2
    }

    fn default_color_scheme_entries(&self) -> &'static [(&'static str, Color)] {
        &DEFAULT_COLOR_SCHEME
    }

    fn preferred_size(&self) -> Dimension {
        Dimension::new(4 * (RADIUS + GAP), 2 * (RADIUS + GAP))
    }

    fn solved_state(&self) -> ClockState {
        ClockState {
            posit: [0; 18],
            right_side_up: true,
        }
    }

    fn random_move_count(&self) -> i32 {
        19
    }

    fn generate_random_moves(
        &self,
        r: &mut dyn RandomSource,
    ) -> PuzzleStateAndGenerator<ClockState> {
        let mut scramble = String::new();
        append_turns(&mut scramble, &TURNS, r);
        scramble.push_str("y2 ");
        append_turns(&mut scramble, &TURNS[4..], r);
        let generator = scramble.trim().to_owned();
        let state = self
            .solved_state()
            .apply_algorithm(&generator)
            .expect("generated clock scrambles are valid");
        PuzzleStateAndGenerator { state, generator }
    }
}

/// The positions of the 18 dials (front then back, 0..12) and which side faces the viewer.
///
/// Like TNoodle, equality only considers the dials, not the side facing the viewer, so `y2`
/// on a solved clock behaves like a rotation.
#[derive(Debug, Clone)]
pub struct ClockState {
    posit: [i32; 18],
    right_side_up: bool,
}

impl PartialEq for ClockState {
    fn eq(&self, other: &Self) -> bool {
        self.posit == other.posit
    }
}

impl Eq for ClockState {}

impl std::hash::Hash for ClockState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.posit.hash(state);
    }
}

impl ClockState {
    /// The dial positions.
    pub fn positions(&self) -> [i32; 18] {
        self.posit
    }

    fn draw_background(&self, g: &mut Svg, scheme: &ColorScheme) {
        let color_string = if self.right_side_up {
            ["Front", "Back"]
        } else {
            ["Back", "Front"]
        };
        let get = |key: &str| scheme.get(key).copied();

        for s in 0..2 {
            let t = Transform::translation(
                f64::from((s * 2 + 1) * (RADIUS + GAP)),
                f64::from(RADIUS + GAP),
            );
            let corners = [-2 * CLOCK_OUTER_RADIUS, 2 * CLOCK_OUTER_RADIUS];

            // Draw the puzzle.
            for cx in corners {
                for cy in corners {
                    let mut c = Element::circle(
                        f64::from(cx),
                        f64::from(cy),
                        f64::from(CLOCK_OUTER_RADIUS),
                    );
                    c.set_transform(Some(&t));
                    c.set_stroke(Some(Color::BLACK));
                    g.append_child(c);
                }
            }

            let mut outer = Element::circle(0.0, 0.0, f64::from(RADIUS));
            outer.set_transform(Some(&t));
            outer.set_stroke(Some(Color::BLACK));
            outer.set_fill(get(color_string[s as usize]));
            g.append_child(outer);

            for cx in corners {
                for cy in corners {
                    // Don't clobber part of the thick outer border.
                    let inner_radius =
                        f64::from(CLOCK_OUTER_RADIUS as f32 - STROKE_WIDTH as f32 / 2.0);
                    let mut c = Element::circle(f64::from(cx), f64::from(cy), inner_radius);
                    c.set_transform(Some(&t));
                    c.set_fill(get(color_string[s as usize]));
                    g.append_child(c);
                }
            }

            // Draw the clocks.
            for i in -1..=1 {
                for j in -1..=1 {
                    let mut t_copy = t;
                    t_copy.translate(
                        f64::from(2 * i * CLOCK_OUTER_RADIUS),
                        f64::from(2 * j * CLOCK_OUTER_RADIUS),
                    );

                    let mut face = Element::circle(0.0, 0.0, f64::from(CLOCK_RADIUS));
                    face.set_stroke_style(FACE_STROKE_WIDTH, 10, "round");
                    face.set_stroke(Some(Color::BLACK));
                    face.set_fill(get(&format!("{}Clock", color_string[s as usize])));
                    face.set_transform(Some(&t_copy));
                    g.append_child(face);

                    for k in 0..12 {
                        let radius = if k == 0 {
                            TOP_TICK_MARK_RADIUS
                        } else {
                            TICK_MARK_RADIUS
                        };
                        let mut tick =
                            Element::circle(0.0, f64::from(-POINT_RADIUS), f64::from(radius));
                        let top = if k == 0 { "Top" } else { "" };
                        tick.set_fill(get(&format!("{}{top}Clock", color_string[s as usize])));
                        tick.rotate(f64::from(30 * k).to_radians());
                        tick.transform(&t_copy);
                        g.append_child(tick);
                    }
                }
            }
        }
    }

    fn draw_clock(&self, g: &mut Svg, clock: usize, position: i32, scheme: &ColorScheme) {
        let arrow_angle = std::f64::consts::FRAC_PI_2
            - java::acos(f64::from(ARROW_RADIUS) / f64::from(ARROW_HEIGHT));
        let mut t = Transform::IDENTITY;
        t.rotate(f64::from(position * 30).to_radians());
        let side_prefix = if (clock < 9) ^ self.right_side_up {
            "Back"
        } else {
            "Front"
        };
        let mut clock = clock as i32;
        if clock < 9 {
            t.translate(f64::from(RADIUS + GAP), f64::from(RADIUS + GAP));
        } else {
            t.translate(f64::from(3 * (RADIUS + GAP)), f64::from(RADIUS + GAP));
            clock -= 9;
        }
        t.translate(
            f64::from(2 * ((clock % 3) - 1) * CLOCK_OUTER_RADIUS),
            f64::from(2 * ((clock / 3) - 1) * CLOCK_OUTER_RADIUS),
        );

        let get = |suffix: &str| scheme.get(&format!("{side_prefix}{suffix}")).copied();
        let radius = f64::from(ARROW_RADIUS);
        let mut arrow = Element::path();
        arrow.move_to(0.0, 0.0);
        arrow.line_to(
            radius * java::cos(arrow_angle),
            -radius * java::sin(arrow_angle),
        );
        arrow.line_to(0.0, f64::from(-ARROW_HEIGHT));
        arrow.line_to(
            -radius * java::cos(arrow_angle),
            -radius * java::sin(arrow_angle),
        );
        arrow.close_path();
        arrow.set_stroke(get("HandBorder"));
        arrow.set_transform(Some(&t));
        let mut arrow_fill = arrow.java_copy();
        g.append_child(arrow);

        let mut hand_base = Element::circle(0.0, 0.0, radius);
        hand_base.set_stroke(get("HandBorder"));
        hand_base.set_transform(Some(&t));
        let mut hand_base_fill = hand_base.java_copy();
        g.append_child(hand_base);

        arrow_fill.set_fill(get("Hand"));
        arrow_fill.set_stroke(None);
        arrow_fill.set_transform(Some(&t));
        g.append_child(arrow_fill);

        hand_base_fill.set_fill(get("Hand"));
        hand_base_fill.set_stroke(None);
        hand_base_fill.set_transform(Some(&t));
        g.append_child(hand_base_fill);
    }

    fn draw_pins(&self, g: &mut Svg, scheme: &ColorScheme) {
        let pin_color = |front: bool| {
            scheme
                .get(if front { "FrontPin" } else { "BackPin" })
                .copied()
        };
        let mut t = Transform::IDENTITY;
        t.translate(f64::from(RADIUS + GAP), f64::from(RADIUS + GAP));
        for side in 0..2 {
            if side == 1 {
                t.translate(f64::from(2 * (RADIUS + GAP)), 0.0);
            }
            // The left face shows the back pins when the front is up, and vice versa.
            let color = pin_color((side == 1) == self.right_side_up);
            for i in [-1, 1] {
                for j in [-1, 1] {
                    let mut tt = t;
                    tt.translate(
                        f64::from(j * CLOCK_OUTER_RADIUS),
                        f64::from(i * CLOCK_OUTER_RADIUS),
                    );
                    let mut pin = Element::circle(0.0, 0.0, f64::from(PIN_RADIUS));
                    pin.set_transform(Some(&tt));
                    pin.set_fill(color);
                    g.append_child(pin);
                }
            }
        }
    }
}

impl PuzzleState for ClockState {
    fn successors_by_name(&self) -> Vec<(String, Self)> {
        let mut successors = Vec::with_capacity(TURNS.len() * 12 + 1);
        for (turn, name) in TURNS.iter().enumerate() {
            for rot in 0..12 {
                let mut posit = [0; 18];
                for (p, v) in posit.iter_mut().enumerate() {
                    *v = (self.posit[p] + rot * MOVES[turn][p] + 12) % 12;
                }
                let mv = if rot < 7 {
                    format!("{name}{rot}+")
                } else {
                    format!("{name}{}-", 12 - rot)
                };
                successors.push((
                    mv,
                    Self {
                        posit,
                        right_side_up: self.right_side_up,
                    },
                ));
            }
        }
        let mut posit = [0; 18];
        posit[9..].copy_from_slice(&self.posit[..9]);
        posit[..9].copy_from_slice(&self.posit[9..]);
        successors.push((
            "y2".to_owned(),
            Self {
                posit,
                right_side_up: !self.right_side_up,
            },
        ));
        successors
    }

    fn java_hash_code(&self) -> i32 {
        array_hash(self.posit)
    }

    fn solved(&self) -> Self {
        ClockPuzzle.solved_state()
    }

    fn draw(&self, scheme: &ColorScheme) -> Svg {
        let mut svg = Svg::new(ClockPuzzle.preferred_size());
        svg.root_mut().set_stroke_style(STROKE_WIDTH, 10, "round");
        self.draw_background(&mut svg, scheme);
        for (i, &p) in self.posit.iter().enumerate() {
            self.draw_clock(&mut svg, i, p, scheme);
        }
        self.draw_pins(&mut svg, scheme);
        svg
    }
}
