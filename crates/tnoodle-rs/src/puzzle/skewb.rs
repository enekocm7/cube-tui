//! The Skewb (`SkewbPuzzle`), in WCA fixed corner notation.

use super::search_random::SearchRandom;
use super::skewb_solver::{self, FREE_CORNER_PERM, SkewbSolver, SkewbSolverState};
use crate::java::{RandomSource, deep_hash_2d, double_to_string};
use crate::scrambles::{ColorScheme, Puzzle, PuzzleState, PuzzleStateAndGenerator};
use crate::svg::{Color, Dimension, Element, Svg, Transform};

const MIN_SCRAMBLE_LENGTH: i32 = 11;
const PIECE_SIZE: i32 = 30;
const GAP: i32 = 3;

static DEFAULT_COLOR_SCHEME: [(&str, Color); 6] = [
    ("U", Color::WHITE),
    ("R", Color::BLUE),
    ("F", Color::RED),
    ("D", Color::YELLOW),
    ("L", Color::GREEN),
    ("B", Color::ORANGE),
];

fn sq3d2() -> f64 {
    3_f64.sqrt() / 2.0
}

/// The Skewb puzzle.
#[derive(Debug, Clone, Copy, Default)]
pub struct SkewbPuzzle {
    search_random: SearchRandom,
}

impl SkewbPuzzle {
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

impl Puzzle for SkewbPuzzle {
    type State = SkewbState;

    fn short_name(&self) -> &str {
        "skewb"
    }

    fn long_name(&self) -> &str {
        "Skewb"
    }

    fn wca_min_scramble_distance(&self) -> i32 {
        7
    }

    fn default_color_scheme_entries(&self) -> &'static [(&'static str, Color)] {
        &DEFAULT_COLOR_SCHEME
    }

    fn preferred_size(&self) -> Dimension {
        Dimension::new(
            (f64::from(3 * GAP + 8 * PIECE_SIZE + 1) * sq3d2()).ceil() as i32,
            f64::from(2 * GAP + 6 * PIECE_SIZE + 1).ceil() as i32,
        )
    }

    fn solved_state(&self) -> SkewbState {
        let mut image = [[0; 5]; 6];
        for (i, face) in image.iter_mut().enumerate() {
            face.fill(i as u8);
        }
        SkewbState {
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
    ) -> PuzzleStateAndGenerator<SkewbState> {
        let solver = SkewbSolver::new();
        let state = solver.random_state(r);
        let scramble = solver
            .generate_exactly(state, MIN_SCRAMBLE_LENGTH, &mut self.search_random.source())
            .expect("every skewb state can be generated in 11 moves");
        let state = self
            .solved_state()
            .apply_algorithm(&scramble)
            .expect("generated skewb scrambles are valid");
        PuzzleStateAndGenerator {
            state,
            generator: scramble,
        }
    }
}

/// The stickers of a Skewb: 6 faces (`U R F D L B`) of a center and 4 corners.
///
/// ```text
///           +---------+
///           | 1     2 |
///       U > |   0-0   |
///           | 3     4 |
/// +---------+---------+---------+---------+
/// | 1     2 | 1     2 | 1     2 | 1     2 |
/// |   4-0   |   2-0   |   1-0   |   5-0   |
/// | 3     4 | 3     4 | 3     4 | 3     4 |
/// +---------+---------+---------+---------+
///      ^    | 1     2 |
///      FL   |   3-0   |
///           | 3     4 |
///           +---------+
/// ```
#[derive(Debug, Clone)]
pub struct SkewbState {
    image: [[u8; 5]; 6],
    search_random: SearchRandom,
}

impl PartialEq for SkewbState {
    fn eq(&self, other: &Self) -> bool {
        self.image == other.image
    }
}

impl Eq for SkewbState {}

impl std::hash::Hash for SkewbState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.image.hash(state);
    }
}

const ORIENTATION_U: u8 = 0;
const ORIENTATION_D: u8 = 3;
/// Maps our faces to the face colours of the Jaap solver.
const COLOR_TO_SOLVER: [i32; 6] = [0, 3, 1, 5, 2, 4];
/// How the stickering changes under a z2 rotation.
const Z2: [usize; 6] = [3, 5, 4, 0, 2, 1];
const FIXED_CORNER_COORDS: [[[usize; 2]; 3]; 4] = [
    [[0, 4], [1, 1], [2, 2]], // U-FR-BR
    [[0, 1], [4, 1], [5, 2]], // U-FL-BL
    [[3, 1], [4, 4], [2, 3]], // D-FL-FR
    [[3, 4], [1, 4], [5, 3]], // D-BL-BR
];
const FREE_CORNER_COORDS: [[[usize; 2]; 3]; 4] = [
    [[0, 3], [2, 1], [4, 2]], // U-FR-FL (Front)
    [[0, 2], [5, 1], [1, 2]], // U-BL-BR (Back)
    [[3, 2], [2, 4], [1, 3]], // D-FR-BR (Right-Down)
    [[3, 3], [5, 4], [4, 3]], // D-FL-BL (Left-Down)
];
const CENTER_COORDS: [[usize; 2]; 6] = [[0, 0], [2, 0], [4, 0], [1, 0], [5, 0], [3, 0]];
/// Maps the sum of a free corner's solver colours to its permutation index.
const FREE_PERM_MAP: [i32; 12] = [-1, -1, -1, 0, -1, -1, -1, 1, -1, 2, -1, 3];

fn swap3(image: &mut [[u8; 5]; 6], a: (usize, usize), b: (usize, usize), c: (usize, usize)) {
    let temp = image[a.0][a.1];
    image[a.0][a.1] = image[b.0][b.1];
    image[b.0][b.1] = image[c.0][c.1];
    image[c.0][c.1] = temp;
}

/// Turns a corner (0 = R, 1 = U, 2 = L, 3 = B) `pow` times.
fn turn(axis: usize, pow: usize, image: &mut [[u8; 5]; 6]) {
    for _ in 0..pow {
        match axis {
            0 => {
                swap3(image, (2, 0), (3, 0), (1, 0));
                swap3(image, (2, 4), (3, 2), (1, 3));
                swap3(image, (2, 2), (3, 1), (1, 4));
                swap3(image, (2, 3), (3, 4), (1, 1));
                swap3(image, (4, 4), (5, 3), (0, 4));
            }
            1 => {
                swap3(image, (0, 0), (1, 0), (5, 0));
                swap3(image, (0, 2), (1, 2), (5, 1));
                swap3(image, (0, 4), (1, 4), (5, 2));
                swap3(image, (0, 1), (1, 1), (5, 3));
                swap3(image, (4, 1), (2, 2), (3, 4));
            }
            2 => {
                swap3(image, (4, 0), (5, 0), (3, 0));
                swap3(image, (4, 3), (5, 4), (3, 3));
                swap3(image, (4, 1), (5, 3), (3, 1));
                swap3(image, (4, 4), (5, 2), (3, 4));
                swap3(image, (2, 3), (0, 1), (1, 4));
            }
            3 => {
                swap3(image, (1, 0), (3, 0), (5, 0));
                swap3(image, (1, 4), (3, 4), (5, 3));
                swap3(image, (1, 3), (3, 3), (5, 1));
                swap3(image, (1, 2), (3, 2), (5, 4));
                swap3(image, (0, 2), (2, 4), (4, 3));
            }
            _ => {}
        }
    }
}

fn orientation_of(image: &[[u8; 5]; 6], coords: &[[usize; 2]; 3]) -> usize {
    let mut orient = 0;
    while image[coords[orient][0]][coords[orient][1]] != ORIENTATION_U
        && image[coords[orient][0]][coords[orient][1]] != ORIENTATION_D
    {
        orient += 1;
    }
    orient
}

/// The placement of each face in the drawing.
fn face_transforms() -> [Transform; 6] {
    let p = f64::from(PIECE_SIZE);
    let g = f64::from(GAP);
    let s = sq3d2();
    // `pieceSize / 2` is an integer division in Java.
    let half = f64::from(PIECE_SIZE / 2);
    [
        Transform::new(p * s, -half, p * s, half, (p * 4.0 + g * 1.5) * s, p),
        Transform::new(p * s, -half, 0.0, p, (p * 7.0 + g * 3.0) * s, p * 1.5),
        Transform::new(
            p * s,
            -half,
            0.0,
            p,
            (p * 5.0 + g * 2.0) * s,
            p * 2.5 + 0.5 * g,
        ),
        Transform::new(
            0.0,
            p,
            -p * s,
            -half,
            (p * 3.0 + g * 1.0) * s,
            p * 4.5 + 1.5 * g,
        ),
        Transform::new(
            p * s,
            half,
            0.0,
            p,
            (p * 3.0 + g * 1.0) * s,
            p * 2.5 + 0.5 * g,
        ),
        Transform::new(p * s, half, 0.0, p, p * s, p * 1.5),
    ]
}

/// The stickers of a face as paths in a square with corners `(±1, ±1)`.
fn face_paths() -> [Element; 5] {
    let mut p: [Element; 5] = std::array::from_fn(|_| {
        let mut path = Element::path();
        // A tiny stroke width compensates for the face transform scaling the stroke.
        path.set_attribute(
            "stroke-width",
            format!("{}px", double_to_string(1.0 / f64::from(PIECE_SIZE))),
        );
        path
    });
    let shapes: [&[(f64, f64)]; 5] = [
        &[(-1.0, 0.0), (0.0, 1.0), (1.0, 0.0), (0.0, -1.0)],
        &[(-1.0, 0.0), (-1.0, -1.0), (0.0, -1.0)],
        &[(0.0, -1.0), (1.0, -1.0), (1.0, 0.0)],
        &[(-1.0, 0.0), (-1.0, 1.0), (0.0, 1.0)],
        &[(0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
    ];
    for (path, shape) in p.iter_mut().zip(shapes) {
        path.move_to(shape[0].0, shape[0].1);
        for &(x, y) in &shape[1..] {
            path.line_to(x, y);
        }
        path.close_path();
    }
    p
}

impl SkewbState {
    /// The equivalent solver state, in Jaap's notation.
    pub fn to_solver_state(&self) -> SkewbSolverState {
        let image = &self.image;
        // In fixed corner notation the U-F-R corner is fixed, while the Jaap solver fixes
        // the corners in the other orbit. Every B move shifts the orientation sum of that
        // orbit, so the whole puzzle has to be rotated accordingly.
        let fcn_b_orbit_orientation: usize = FIXED_CORNER_COORDS
            .iter()
            .map(|c| orientation_of(image, c))
            .sum();

        // A z2 rotation swaps the two corner orbits.
        let mut jaap_correction = Z2;
        let cycle = |s: &mut [usize; 6], a: usize, b: usize, c: usize| {
            let t = s[a];
            s[a] = s[b];
            s[b] = s[c];
            s[c] = t;
        };
        for _ in 0..fcn_b_orbit_orientation % 3 {
            cycle(&mut jaap_correction, 0, 4, 2);
            cycle(&mut jaap_correction, 1, 5, 3);
        }
        let jc = |f: usize, s: usize| jaap_correction[image[f][s] as usize] as u8;
        let jaap_image: [[u8; 5]; 6] = [
            [jc(3, 0), jc(3, 2), jc(3, 4), jc(3, 1), jc(3, 3)],
            [jc(5, 0), jc(5, 4), jc(5, 3), jc(5, 2), jc(5, 1)],
            [jc(4, 0), jc(4, 4), jc(4, 3), jc(4, 2), jc(4, 1)],
            [jc(0, 0), jc(0, 3), jc(0, 1), jc(0, 4), jc(0, 2)],
            [jc(2, 0), jc(2, 4), jc(2, 3), jc(2, 2), jc(2, 1)],
            [jc(1, 0), jc(1, 4), jc(1, 3), jc(1, 2), jc(1, 1)],
        ];

        let mut center_perm = [0; 6];
        for (c, coords) in center_perm.iter_mut().zip(CENTER_COORDS) {
            *c = COLOR_TO_SOLVER[jaap_image[coords[0]][coords[1]] as usize];
        }
        let mut fixed_twist = [0; 4];
        for (t, coords) in fixed_twist.iter_mut().zip(&FIXED_CORNER_COORDS) {
            *t = orientation_of(&jaap_image, coords) as i32;
        }
        let mut free_perm = [0; 4];
        let mut free_twist = [0; 4];
        for i in 0..4 {
            let coords = &FREE_CORNER_COORDS[i];
            let sum: i32 = coords
                .iter()
                .map(|c| COLOR_TO_SOLVER[jaap_image[c[0]][c[1]] as usize])
                .sum();
            free_perm[i] = FREE_PERM_MAP[sum as usize];
            free_twist[i] = orientation_of(&jaap_image, coords) as i32;
        }
        SkewbSolverState {
            perm: skewb_solver::pack_center_perm(&center_perm) * FREE_CORNER_PERM
                + skewb_solver::pack_corner_perm(&free_perm),
            twst: skewb_solver::pack_corner_orient(&free_twist, &fixed_twist),
        }
    }
}

impl PuzzleState for SkewbState {
    fn successors_by_name(&self) -> Vec<(String, Self)> {
        let mut successors = Vec::with_capacity(8);
        for (axis, face) in "RULB".chars().enumerate() {
            for pow in 1..=2 {
                let turn_name = if pow == 2 {
                    format!("{face}'")
                } else {
                    face.to_string()
                };
                let mut image = self.image;
                turn(axis, pow, &mut image);
                successors.push((
                    turn_name,
                    Self {
                        image,
                        search_random: self.search_random,
                    },
                ));
            }
        }
        successors
    }

    fn java_hash_code(&self) -> i32 {
        deep_hash_2d(self.image.as_flattened(), 5)
    }

    fn solved(&self) -> Self {
        SkewbPuzzle {
            search_random: self.search_random,
        }
        .solved_state()
    }

    fn solve_in(&self, n: i32) -> Option<String> {
        SkewbSolver::new().solve_in(self.to_solver_state(), n, &mut self.search_random.source())
    }

    fn draw(&self, scheme: &ColorScheme) -> Svg {
        let mut svg = Svg::new(SkewbPuzzle::new().preferred_size());
        let colors = ["U", "R", "F", "D", "L", "B"].map(|f| scheme.get(f).copied());
        let positions = face_transforms();
        for (face, position) in positions.iter().enumerate() {
            for (i, mut p) in face_paths().into_iter().enumerate() {
                p.transform(position);
                p.set_fill(colors[self.image[face][i] as usize]);
                p.set_stroke(Some(Color::BLACK));
                svg.append_child(p);
            }
        }
        svg
    }
}
