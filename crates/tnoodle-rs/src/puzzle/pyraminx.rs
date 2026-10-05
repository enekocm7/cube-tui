//! The Pyraminx (`PyraminxPuzzle`).

use std::f64::consts::PI;

use super::megaminx::line_intersection;
use super::pyraminx_solver::{self, PyraminxSolver, PyraminxSolverState};
use super::search_random::SearchRandom;
use crate::java::{self, RandomSource, deep_hash_2d};
use crate::scrambles::{ColorScheme, Puzzle, PuzzleState, PuzzleStateAndGenerator};
use crate::svg::{Color, Dimension, Element, PathCommand, Svg};

const MIN_SCRAMBLE_LENGTH: i32 = 11;
const SCRAMBLE_LENGTH_INCLUDES_TIPS: bool = true;
const PIECE_SIZE: i32 = 30;
const GAP: i32 = 5;

static DEFAULT_COLOR_SCHEME: [(&str, Color); 4] = [
    ("F", Color::GREEN),
    ("D", Color::YELLOW),
    ("L", Color::RED),
    ("R", Color::BLUE),
];

/// The Pyraminx puzzle.
#[derive(Debug, Clone, Copy, Default)]
pub struct PyraminxPuzzle {
    search_random: SearchRandom,
}

impl PyraminxPuzzle {
    /// Creates the puzzle.
    pub const fn new() -> Self {
        Self {
            search_random: SearchRandom::Entropy,
        }
    }

    /// Makes the solver's move order deterministic: every solve starts from
    /// `java.util.Random(seed)` instead of fresh entropy (TNoodle uses `new Random()`).
    #[must_use]
    pub const fn with_search_seed(mut self, seed: i64) -> Self {
        self.search_random = SearchRandom::Seeded(seed);
        self
    }
}

impl Puzzle for PyraminxPuzzle {
    type State = PyraminxState;

    fn short_name(&self) -> &str {
        "pyram"
    }

    fn long_name(&self) -> &str {
        "Pyraminx"
    }

    fn wca_min_scramble_distance(&self) -> i32 {
        6
    }

    fn default_color_scheme_entries(&self) -> &'static [(&'static str, Color)] {
        &DEFAULT_COLOR_SCHEME
    }

    fn preferred_size(&self) -> Dimension {
        Dimension::new(
            2 * 3 * PIECE_SIZE + 4 * GAP,
            (2.0 * 1.5 * 3_f64.sqrt() * f64::from(PIECE_SIZE) + f64::from(3 * GAP)) as i32,
        )
    }

    fn solved_state(&self) -> PyraminxState {
        let mut image = [[0; 9]; 4];
        for (i, face) in image.iter_mut().enumerate() {
            face.fill(i as u8);
        }
        PyraminxState {
            image,
            search_random: self.search_random,
        }
    }

    fn random_move_count(&self) -> i32 {
        15
    }

    fn generate_random_moves(
        &self,
        r: &mut dyn RandomSource,
    ) -> PuzzleStateAndGenerator<PyraminxState> {
        let solver = PyraminxSolver::new();
        let state = solver.random_state(r);
        let scramble = solver
            .generate_exactly(
                state,
                MIN_SCRAMBLE_LENGTH,
                false,
                &mut self.search_random.source(),
            )
            .expect("every pyraminx state can be generated in 11 moves");
        let state = self
            .solved_state()
            .apply_algorithm(&scramble)
            .expect("generated pyraminx scrambles are valid");
        PuzzleStateAndGenerator {
            state,
            generator: scramble,
        }
    }
}

/// The stickers of a Pyraminx: 4 faces of 9 stickers.
///
/// ```text
///                                     U
///               ____  ____  ____              ____  ____  ____
///              \    /\    /\    /     /\     \    /\    /\    /
///               \0 /1 \2 /4 \3 /     /0 \     \0 /1 \2 /4 \3 /
///                \/____\/____\/     /____\     \/____\/____\/
///                 \    /\    /     /\    /\     \    /\    /
///         face 2   \8 /7 \5 /     /8 \1 /2 \     \8 /7 \5 / face 3
///                   \/____\/     /____\/____\     \/____\/
///                    \    /     /\    /\    /\     \    /
///                     \6 /     /6 \7 /5 \4 /3 \     \6 /
///                      \/     /____\/____\/____\     \/
///                                   face 0
///                         L    ____  ____  ____    R
///                             \    /\    /\    /
///                              \0 /1 \2 /4 \3 /
///                               \/____\/____\/
///                                \    /\    /
///                                 \8 /7 \5 /
///                          face 1  \/____\/
///                                   \    /
///                                    \6 /
///                                     \/
///                                     B
/// ```
#[derive(Debug, Clone)]
pub struct PyraminxState {
    image: [[u8; 9]; 4],
    search_random: SearchRandom,
}

impl PartialEq for PyraminxState {
    fn eq(&self, other: &Self) -> bool {
        self.image == other.image
    }
}

impl Eq for PyraminxState {}

impl std::hash::Hash for PyraminxState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.image.hash(state);
    }
}

fn swap3(image: &mut [[u8; 9]; 4], a: (usize, usize), b: (usize, usize), c: (usize, usize)) {
    let temp = image[a.0][a.1];
    image[a.0][a.1] = image[b.0][b.1];
    image[b.0][b.1] = image[c.0][c.1];
    image[c.0][c.1] = temp;
}

fn turn_once(s: usize, image: &mut [[u8; 9]; 4]) {
    match s {
        0 => {
            swap3(image, (0, 8), (3, 8), (2, 2));
            swap3(image, (0, 1), (3, 1), (2, 4));
            swap3(image, (0, 2), (3, 2), (2, 5));
        }
        1 => {
            swap3(image, (2, 8), (1, 2), (0, 8));
            swap3(image, (2, 7), (1, 1), (0, 7));
            swap3(image, (2, 5), (1, 8), (0, 5));
        }
        2 => {
            swap3(image, (3, 8), (0, 5), (1, 5));
            swap3(image, (3, 7), (0, 4), (1, 4));
            swap3(image, (3, 5), (0, 2), (1, 2));
        }
        3 => {
            swap3(image, (1, 8), (2, 2), (3, 5));
            swap3(image, (1, 7), (2, 1), (3, 4));
            swap3(image, (1, 5), (2, 8), (3, 2));
        }
        _ => {}
    }
    turn_tip_once(s, image);
}

fn turn_tip_once(s: usize, image: &mut [[u8; 9]; 4]) {
    match s {
        0 => swap3(image, (0, 0), (3, 0), (2, 3)),
        1 => swap3(image, (0, 6), (2, 6), (1, 0)),
        2 => swap3(image, (0, 3), (1, 3), (3, 6)),
        3 => swap3(image, (1, 6), (2, 0), (3, 3)),
        _ => {}
    }
}

impl PyraminxState {
    /// The equivalent solver state.
    pub fn to_solver_state(&self) -> PyraminxSolverState {
        let image = &self.image;
        // Colour values chosen so that the sum identifies each edge; the primary facelet of
        // each edge is the one on the lowest numbered face.
        let stickers_to_edges = [
            [image[0][5], image[1][2]],
            [image[0][8], image[2][5]],
            [image[1][8], image[2][8]],
            [image[0][2], image[3][8]],
            [image[1][5], image[3][5]],
            [image[2][2], image[3][2]],
        ];
        let color_to_value = [0, 1, 2, 4];
        let mut edges = [0; 6];
        for (e, s) in edges.iter_mut().zip(stickers_to_edges) {
            *e = color_to_value[s[0] as usize] + color_to_value[s[1] as usize] - 1;
            if s[0] > s[1] {
                *e += 8;
            }
        }

        let stickers_to_corners = [
            [image[0][1], image[2][4], image[3][1]],
            [image[0][7], image[1][1], image[2][7]],
            [image[0][4], image[3][7], image[1][4]],
            [image[1][7], image[3][4], image[2][1]],
        ];
        let mut corners = [0; 4];
        for (c, s) in corners.iter_mut().zip(stickers_to_corners) {
            if s[0] < s[1] && s[0] < s[2] {
                *c = 0;
            }
            if s[1] < s[0] && s[1] < s[2] {
                *c = 1;
            }
            if s[2] < s[1] && s[2] < s[0] {
                *c = 2;
            }
        }

        // The tips use the same numbering; they are compared with their corner's colours.
        let stickers_to_tips = [
            [image[0][0], image[2][3], image[3][0]],
            [image[0][6], image[1][0], image[2][6]],
            [image[0][3], image[3][6], image[1][3]],
            [image[1][6], image[3][3], image[2][0]],
        ];
        let mut tips = [0; 4];
        for (i, tip) in tips.iter_mut().enumerate() {
            let corner_primary_color = stickers_to_corners[i][0];
            let mut turns = 0;
            while stickers_to_tips[i][turns] != corner_primary_color {
                turns += 1;
            }
            *tip = turns as i32;
        }

        PyraminxSolverState {
            edge_perm: pyraminx_solver::pack_edge_perm(&edges),
            edge_orient: pyraminx_solver::pack_edge_orient(&edges),
            corner_orient: pyraminx_solver::pack_corner_orient(&corners),
            tips: pyraminx_solver::pack_corner_orient(&tips),
        }
    }

    fn draw_triangle(
        svg: &mut Svg,
        x: f64,
        y: f64,
        up: bool,
        state: &[u8; 9],
        scheme: &[Option<Color>; 4],
    ) {
        let mut p = triangle(up, PIECE_SIZE);
        p.translate_path(x, y);

        let mut xpoints = [0.0; 3];
        let mut ypoints = [0.0; 3];
        for (ch, cmd) in p.path_commands().iter().take(3).enumerate() {
            if let PathCommand::MoveTo(px, py) | PathCommand::LineTo(px, py) = *cmd {
                xpoints[ch] = px;
                ypoints[ch] = py;
            }
        }

        let mut xs = [0.0; 6];
        let mut ys = [0.0; 6];
        for i in 0..3 {
            xs[i] = 1.0 / 3.0 * xpoints[(i + 1) % 3] + 2.0 / 3.0 * xpoints[i];
            ys[i] = 1.0 / 3.0 * ypoints[(i + 1) % 3] + 2.0 / 3.0 * ypoints[i];
            xs[i + 3] = 2.0 / 3.0 * xpoints[(i + 1) % 3] + 1.0 / 3.0 * xpoints[i];
            ys[i + 3] = 2.0 / 3.0 * ypoints[(i + 1) % 3] + 1.0 / 3.0 * ypoints[i];
        }

        let mut ps: Vec<Element> = (0..9).map(|_| Element::path()).collect();
        let center = line_intersection(xs[0], ys[0], xs[4], ys[4], xs[2], ys[2], xs[3], ys[3]);
        for i in 0..3 {
            let a = &mut ps[3 * i];
            a.move_to(xpoints[i], ypoints[i]);
            a.line_to(xs[i], ys[i]);
            a.line_to(xs[3 + (2 + i) % 3], ys[3 + (2 + i) % 3]);
            a.close_path();

            let b = &mut ps[3 * i + 1];
            b.move_to(xs[i], ys[i]);
            b.line_to(xs[3 + (i + 2) % 3], ys[3 + (i + 2) % 3]);
            b.line_to(center.x, center.y);
            b.close_path();

            let c = &mut ps[3 * i + 2];
            c.move_to(xs[i], ys[i]);
            c.line_to(xs[i + 3], ys[i + 3]);
            c.line_to(center.x, center.y);
            c.close_path();
        }

        for (i, mut sticker) in ps.into_iter().enumerate() {
            sticker.set_fill(scheme[state[i] as usize]);
            sticker.set_stroke(Some(Color::BLACK));
            svg.append_child(sticker);
        }
    }
}

fn triangle(pointup: bool, piece_size: i32) -> Element {
    let rad = (3_f64.sqrt() * f64::from(piece_size)) as i32;
    let mut angs = [7.0 / 6.0, 11.0 / 6.0, 0.5];
    for a in &mut angs {
        if pointup {
            *a += 1.0 / 3.0;
        }
        *a *= PI;
    }
    let rad = f64::from(rad);
    let mut p = Element::path();
    p.move_to(rad * java::cos(angs[0]), rad * java::sin(angs[0]));
    for &a in &angs[1..] {
        p.line_to(rad * java::cos(a), rad * java::sin(a));
    }
    p.close_path();
    p
}

impl PuzzleState for PyraminxState {
    fn successors_by_name(&self) -> Vec<(String, Self)> {
        let mut successors = Vec::with_capacity(16);
        for (axis, face) in "ulrb".chars().enumerate() {
            for tip in [true, false] {
                let face = if tip { face } else { face.to_ascii_uppercase() };
                for dir in 1..=2 {
                    let turn = if dir == 2 {
                        format!("{face}'")
                    } else {
                        face.to_string()
                    };
                    let mut image = self.image;
                    for _ in 0..dir {
                        if tip {
                            turn_tip_once(axis, &mut image);
                        } else {
                            turn_once(axis, &mut image);
                        }
                    }
                    successors.push((
                        turn,
                        Self {
                            image,
                            search_random: self.search_random,
                        },
                    ));
                }
            }
        }
        successors
    }

    fn java_hash_code(&self) -> i32 {
        deep_hash_2d(self.image.as_flattened(), 9)
    }

    fn solved(&self) -> Self {
        PyraminxPuzzle {
            search_random: self.search_random,
        }
        .solved_state()
    }

    fn solve_in(&self, n: i32) -> Option<String> {
        PyraminxSolver::new().solve_in(
            self.to_solver_state(),
            n,
            SCRAMBLE_LENGTH_INCLUDES_TIPS,
            &mut self.search_random.source(),
        )
    }

    fn draw(&self, scheme: &ColorScheme) -> Svg {
        let mut svg = Svg::new(PyraminxPuzzle::new().preferred_size());
        svg.root_mut().set_stroke_style(2, 10, "round");
        let colors = ["F", "D", "L", "R"].map(|f| scheme.get(f).copied());
        let piece = f64::from(PIECE_SIZE);
        let gap = f64::from(GAP);
        let sqrt3 = 3_f64.sqrt();
        Self::draw_triangle(
            &mut svg,
            2.0 * gap + 3.0 * piece,
            gap + sqrt3 * piece,
            true,
            &self.image[0],
            &colors,
        );
        Self::draw_triangle(
            &mut svg,
            2.0 * gap + 3.0 * piece,
            2.0 * gap + 2.0 * sqrt3 * piece,
            false,
            &self.image[1],
            &colors,
        );
        Self::draw_triangle(
            &mut svg,
            gap + 1.5 * piece,
            gap + sqrt3 / 2.0 * piece,
            false,
            &self.image[2],
            &colors,
        );
        Self::draw_triangle(
            &mut svg,
            3.0 * gap + 4.5 * piece,
            gap + sqrt3 / 2.0 * piece,
            false,
            &self.image[3],
            &colors,
        );
        svg
    }
}
