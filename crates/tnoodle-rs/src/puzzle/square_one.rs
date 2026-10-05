//! The Square-1 (`SquareOnePuzzle`).

use crate::java::{self, RandomSource, array_hash};
use crate::scrambles::{
    AlgorithmBuilder, ColorScheme, MergingMode, Puzzle, PuzzleState, PuzzleStateAndGenerator,
};
use crate::sq12phase::{self, FullCube};
use crate::svg::{Color, Dimension, Element, Svg, Transform};

const RADIUS: i32 = 32;
const MULTIPLIER: f64 = 1.4;

static DEFAULT_COLOR_SCHEME: [(&str, Color); 6] = [
    ("B", Color::ORANGE),
    ("D", Color::WHITE),
    ("F", Color::RED),
    ("L", Color::BLUE),
    ("R", Color::GREEN),
    ("U", Color::YELLOW),
];

fn radius_multiplier() -> f64 {
    2_f64.sqrt() * java::cos(15_f64.to_radians())
}

fn width(radius: i32) -> i32 {
    (2.0 * radius_multiplier() * MULTIPLIER * f64::from(radius)) as i32
}

fn height(radius: i32) -> i32 {
    (4.0 * radius_multiplier() * MULTIPLIER * f64::from(radius)) as i32
}

/// The Square-1 puzzle.
#[derive(Debug, Clone, Copy, Default)]
pub struct SquareOnePuzzle;

impl SquareOnePuzzle {
    /// Creates the puzzle.
    pub const fn new() -> Self {
        Self
    }
}

impl Puzzle for SquareOnePuzzle {
    type State = SquareOneState;

    fn short_name(&self) -> &str {
        "sq1"
    }

    fn long_name(&self) -> &str {
        "Square-1"
    }

    fn wca_min_scramble_distance(&self) -> i32 {
        11
    }

    fn default_color_scheme_entries(&self) -> &'static [(&'static str, Color)] {
        &DEFAULT_COLOR_SCHEME
    }

    fn preferred_size(&self) -> Dimension {
        Dimension::new(width(RADIUS), height(RADIUS))
    }

    fn solved_state(&self) -> SquareOneState {
        SquareOneState {
            slice_solved: true,
            pieces: [
                0, 0, 1, 2, 2, 3, 4, 4, 5, 6, 6, 7, 8, 9, 9, 10, 11, 11, 12, 13, 13, 14, 15, 15,
            ],
        }
    }

    fn random_move_count(&self) -> i32 {
        40
    }

    fn generate_random_moves(
        &self,
        r: &mut dyn RandomSource,
    ) -> PuzzleStateAndGenerator<SquareOneState> {
        let random_state = FullCube::random(r);
        let scramble = sq12phase::Search::new()
            .solution(&random_state, sq12phase::INVERSE_SOLUTION)
            .expect("every square-1 state has a solution")
            .trim()
            .to_owned();
        let state = self
            .solved_state()
            .apply_algorithm(&scramble)
            .expect("generated square-1 scrambles are valid");
        PuzzleStateAndGenerator {
            state,
            generator: scramble,
        }
    }
}

/// The 24 slots of a Square-1 (12 per layer, a corner fills two) and whether the middle
/// layer is solved. Pieces 0..8 are in the top layer when solved; even pieces below 8 and
/// odd pieces from 8 are corners.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SquareOneState {
    slice_solved: bool,
    pieces: [u8; 24],
}

/// The slashability cost of a turn: the total amount of turning.
fn slashability_cost(mv: &str) -> Option<i32> {
    parse_turn(mv).map(|(top, bottom)| top.abs() + bottom.abs())
}

/// Parses `(top,bottom)` for `top, bottom` in `-5..=6`, not both zero.
fn parse_turn(mv: &str) -> Option<(i32, i32)> {
    let inner = mv.strip_prefix('(')?.strip_suffix(')')?;
    let (top, bottom) = inner.split_once(',')?;
    let (top, bottom): (i32, i32) = (top.parse().ok()?, bottom.parse().ok()?);
    let valid = (-5..=6).contains(&top) && (-5..=6).contains(&bottom) && (top, bottom) != (0, 0);
    // Reject non-canonical spellings such as `(+1,0)` or `(01,0)`.
    (valid && mv == format!("({top},{bottom})")).then_some((top, bottom))
}

impl SquareOneState {
    /// The pieces in the 24 slots.
    pub fn pieces(&self) -> [u8; 24] {
        self.pieces
    }

    /// Whether the middle layer is solved.
    pub fn is_slice_solved(&self) -> bool {
        self.slice_solved
    }

    /// The equivalent sq12phase cube.
    pub fn to_full_cube(&self) -> FullCube {
        const MAP1: [i32; 16] = [3, 2, 1, 0, 7, 6, 5, 4, 0xa, 0xb, 8, 9, 0xe, 0xf, 0xc, 0xd];
        const MAP2: [usize; 24] = [
            5, 4, 3, 2, 1, 0, 11, 10, 9, 8, 7, 6, 17, 16, 15, 14, 13, 12, 23, 22, 21, 20, 19, 18,
        ];
        let mut f = FullCube::new();
        for i in 0..24 {
            f.set_piece(MAP2[i], MAP1[self.pieces[i] as usize]);
        }
        f.set_piece(24, i32::from(!self.slice_solved));
        f
    }

    fn do_slash(&self) -> [u8; 24] {
        let mut p = self.pieces;
        for i in 0..6 {
            p.swap(i + 12, i + 6);
        }
        p
    }

    fn can_slash(&self) -> bool {
        let p = &self.pieces;
        p[0] != p[11] && p[6] != p[5] && p[12] != p[23] && p[12 + 6] != p[(12 + 6) - 1]
    }

    fn rotate_top_and_bottom(&self, top: i32, bottom: i32) -> [u8; 24] {
        let top = (-top).rem_euclid(12) as usize;
        let bottom = (-bottom).rem_euclid(12) as usize;
        let mut p = self.pieces;
        p[..12].rotate_left(top);
        p[12..].rotate_left(bottom);
        p
    }

    /// Solves a state that cannot do a `/` by first turning the layers as little as
    /// possible, hoping that the setup move cancels with the solution.
    fn solve_with_slashability_in(
        &self,
        n: i32,
        setup: &str,
        pre_setup: &Self,
        lower_threshold: i32,
    ) -> Option<String> {
        if !self.can_slash() || n < lower_threshold {
            return None;
        }
        // Don't search n - 1 moves: the solution may cancel with the setup move.
        let next_best = self.solve_in(n)?;
        let mut ab =
            AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, pre_setup.clone());
        ab.append_move(setup).expect("the setup move is valid");
        ab.append_algorithm(&next_best)
            .expect("sq12phase solutions are valid");
        if ab.total_cost() > n {
            // The setup move did not cancel with the solution; try a shorter one.
            return self.solve_with_slashability_in(n - 1, setup, pre_setup, lower_threshold);
        }
        Some(ab.state_and_generator().generator)
    }

    fn draw_face(
        svg: &mut Svg,
        transform: &mut Transform,
        face: &[u8],
        x: f64,
        y: f64,
        colors: &[Option<Color>; 6],
    ) {
        let mut ch = 0;
        while ch < 12 {
            if ch < 11 && face[ch] == face[ch + 1] {
                ch += 1;
            }
            draw_piece(svg, transform, face[ch], x, y, colors);
            ch += 1;
        }
    }
}

fn is_corner_piece(piece: u8) -> bool {
    (piece + u8::from(piece > 7)).is_multiple_of(2)
}

fn piece_colors(piece: u8, scheme: &[Option<Color>; 6]) -> Vec<Option<Color>> {
    let up = piece <= 7;
    let top = if up { scheme[4] } else { scheme[5] };
    if is_corner_piece(piece) {
        let piece = if up { piece } else { 15 - piece } as usize;
        let mut a = scheme[(piece / 2 + 3) % 4];
        let mut b = scheme[piece / 2];
        if !up {
            // Mirrored for the bottom layer.
            std::mem::swap(&mut a, &mut b);
        }
        vec![top, a, b] // ordered counter-clockwise
    } else {
        let piece = if up { piece } else { 14 - piece } as usize;
        vec![top, scheme[piece / 2]]
    }
}

fn wedge_poly(x: f64, y: f64, radius: i32) -> Vec<Element> {
    let r = f64::from(radius);
    let mut p = Element::path();
    p.move_to(0.0, 0.0);
    p.line_to(r, 0.0);
    let tempx = 3_f64.sqrt() * r / 2.0;
    let tempy = r / 2.0;
    p.line_to(tempx, tempy);
    p.close_path();
    p.translate_path(x, y);

    let mut side = Element::path();
    side.move_to(r, 0.0);
    side.line_to(MULTIPLIER * r, 0.0);
    side.line_to(MULTIPLIER * tempx, MULTIPLIER * tempy);
    side.line_to(tempx, tempy);
    side.close_path();
    side.translate_path(x, y);
    vec![p, side]
}

fn corner_poly(x: f64, y: f64, radius: i32) -> Vec<Element> {
    let r = f64::from(radius);
    let mut p = Element::path();
    p.move_to(0.0, 0.0);
    p.line_to(r, 0.0);
    let tempx = r * (1.0 + java::cos(75_f64.to_radians()) / 2_f64.sqrt());
    let tempy = r * java::sin(75_f64.to_radians()) / 2_f64.sqrt();
    p.line_to(tempx, tempy);
    let temp_x = r / 2.0;
    let temp_y = 3_f64.sqrt() * r / 2.0;
    p.line_to(temp_x, temp_y);
    p.close_path();
    p.translate_path(x, y);

    let mut side1 = Element::path();
    side1.move_to(r, 0.0);
    side1.line_to(MULTIPLIER * r, 0.0);
    side1.line_to(MULTIPLIER * tempx, MULTIPLIER * tempy);
    side1.line_to(tempx, tempy);
    side1.close_path();
    side1.translate_path(x, y);

    let mut side2 = Element::path();
    side2.move_to(MULTIPLIER * tempx, MULTIPLIER * tempy);
    side2.line_to(tempx, tempy);
    side2.line_to(temp_x, temp_y);
    side2.line_to(MULTIPLIER * temp_x, MULTIPLIER * temp_y);
    side2.close_path();
    side2.translate_path(x, y);
    vec![p, side1, side2]
}

fn draw_piece(
    svg: &mut Svg,
    transform: &mut Transform,
    piece: u8,
    x: f64,
    y: f64,
    scheme: &[Option<Color>; 6],
) {
    let corner = is_corner_piece(piece);
    let degree = 30 * if corner { 2 } else { 1 };
    let mut polys = if corner {
        corner_poly(x, y, RADIUS)
    } else {
        wedge_poly(x, y, RADIUS)
    };
    let colors = piece_colors(piece, scheme);
    for ch in (0..colors.len()).rev() {
        let mut p = std::mem::replace(&mut polys[ch], Element::path());
        p.set_fill(colors[ch]);
        p.set_stroke(Some(Color::BLACK));
        p.set_transform(Some(transform));
        svg.append_child(p);
    }
    transform.rotate_around(f64::from(degree).to_radians(), x, y);
}

impl PuzzleState for SquareOneState {
    fn successors_by_name(&self) -> Vec<(String, Self)> {
        let mut successors = Vec::with_capacity(144);
        for top in -5..=6 {
            for bottom in -5..=6 {
                if top == 0 && bottom == 0 {
                    // No use doing nothing.
                    continue;
                }
                successors.push((
                    format!("({top},{bottom})"),
                    Self {
                        slice_solved: self.slice_solved,
                        pieces: self.rotate_top_and_bottom(top, bottom),
                    },
                ));
            }
        }
        if self.can_slash() {
            successors.push((
                "/".to_owned(),
                Self {
                    slice_solved: !self.slice_solved,
                    pieces: self.do_slash(),
                },
            ));
        }
        successors
    }

    /// Only the turns that leave the puzzle able to do a `/`.
    fn scramble_successors(&self) -> Vec<(String, Self)> {
        self.successors_by_name()
            .into_iter()
            .filter(|(_, state)| state.can_slash())
            .collect()
    }

    /// Every turn and slash counts as one move (WCA regulation 12c4).
    fn move_cost(&self, _mv: &str) -> i32 {
        1
    }

    fn java_hash_code(&self) -> i32 {
        array_hash(self.pieces.iter().map(|&p| i32::from(p))) ^ i32::from(self.slice_solved)
    }

    fn solved(&self) -> Self {
        SquareOnePuzzle.solved_state()
    }

    fn solve_in(&self, n: i32) -> Option<String> {
        // sq12phase can neither represent nor solve states that cannot do a `/`.
        if !self.can_slash() {
            // Prefer the least invasive setup move: for (-1,0), (1,0) over (4,0) or (5,0).
            let mut best: Option<(String, Self)> = None;
            let mut current_min = i32::MAX;
            for (mv, state) in self.scramble_successors() {
                if let Some(cost) = slashability_cost(&mv)
                    && cost < current_min
                {
                    current_min = cost;
                    best = Some((mv, state));
                }
            }
            let (setup, slashable) = best?;
            return slashable.solve_with_slashability_in(n, &setup, self, n - 1);
        }
        sq12phase::Search::new()
            .solution_opt(&self.to_full_cube(), n, 0)
            .map(|s| s.trim().to_owned())
    }

    fn draw(&self, scheme: &ColorScheme) -> Svg {
        let mut svg = Svg::new(SquareOnePuzzle.preferred_size());
        svg.root_mut().set_stroke_style(2, 10, "round");
        let colors = ["L", "B", "R", "F", "U", "D"].map(|f| scheme.get(f).copied());

        let dim = SquareOnePuzzle.preferred_size();
        let (w, h) = (f64::from(dim.width), f64::from(dim.height));
        let r = f64::from(RADIUS);
        let half_square_width = (r * radius_multiplier() * MULTIPLIER) / 2_f64.sqrt();
        let edge_width = 2.0 * r * MULTIPLIER * java::sin(15_f64.to_radians());
        let corner_width = half_square_width - edge_width / 2.0;
        let mid_y = h / 2.0 - r * (MULTIPLIER - 1.0) / 2.0;
        let mid_height = r * (MULTIPLIER - 1.0);
        let mut left_mid =
            Element::rectangle(w / 2.0 - half_square_width, mid_y, corner_width, mid_height);
        left_mid.set_fill(colors[3]); // front
        let right_mid = if self.slice_solved {
            let mut rect = Element::rectangle(
                w / 2.0 - half_square_width,
                mid_y,
                2.0 * corner_width + edge_width,
                mid_height,
            );
            rect.set_fill(colors[3]); // front
            rect
        } else {
            let mut rect = Element::rectangle(
                w / 2.0 - half_square_width,
                mid_y,
                corner_width + edge_width,
                mid_height,
            );
            rect.set_fill(colors[1]); // back
            rect
        };
        let mut right_outline = right_mid.java_copy();
        svg.append_child(right_mid);
        let mut left_outline = left_mid.java_copy();
        svg.append_child(left_mid); // clobbers part of the right one
        right_outline.set_stroke(Some(Color::BLACK));
        right_outline.set_fill(None);
        left_outline.set_stroke(Some(Color::BLACK));
        left_outline.set_fill(None);
        svg.append_child(right_outline);
        svg.append_child(left_outline);

        let x = w / 2.0;
        let mut y = h / 4.0;
        let mut transform = Transform::rotation_around(f64::from(90 + 15).to_radians(), x, y);
        Self::draw_face(&mut svg, &mut transform, &self.pieces[..12], x, y, &colors);
        y *= 3.0;
        let mut transform = Transform::rotation_around(f64::from(-90 - 15).to_radians(), x, y);
        Self::draw_face(&mut svg, &mut transform, &self.pieces[12..], x, y, &colors);
        svg
    }
}

impl std::fmt::Display for SquareOneState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pieces: Vec<String> = self.pieces.iter().map(ToString::to_string).collect();
        write!(
            f,
            "sliceSolved: {} [{}]",
            self.slice_solved,
            pieces.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_turns() {
        assert_eq!(parse_turn("(1,0)"), Some((1, 0)));
        assert_eq!(parse_turn("(-5,6)"), Some((-5, 6)));
        assert_eq!(parse_turn("(0,0)"), None);
        assert_eq!(parse_turn("(7,0)"), None);
        assert_eq!(parse_turn("(+1,0)"), None);
        assert_eq!(parse_turn("/"), None);
        assert_eq!(slashability_cost("(-3,2)"), Some(5));
    }

    #[test]
    fn displays_like_java() {
        let s = SquareOnePuzzle.solved_state();
        assert!(s.to_string().starts_with("sliceSolved: true [0, 0, 1, 2"));
        assert!(s.is_slice_solved());
        assert_eq!(s.pieces()[23], 15);
    }
}
