//! The Face Turning Octahedron (`FaceTurningOctahedronPuzzle`).

use crate::fto3phase::{FtoCubie, Search};
use crate::java::{RandomSource, deep_hash_2d, java_hash_order};
use crate::scrambles::per_thread::PerThread;
use crate::scrambles::{ColorScheme, Puzzle, PuzzleState, PuzzleStateAndGenerator};
use crate::svg::{Color, Dimension, Element, Svg};

static DEFAULT_COLOR_SCHEME: [(&str, Color); 8] = [
    ("B", Color::BLUE),
    ("D", Color::YELLOW),
    ("F", Color::GREEN),
    ("L", Color::PURPLE),
    ("R", Color::RED),
    ("U", Color::WHITE),
    ("BL", Color::ORANGE),
    ("BR", Color::GRAY),
];

const STICKER_SIZE: i32 = 30;
const CENTER_GAP_SIZE: i32 = 12;
const FACE_GAP_SIZE: i32 = 3;
const MARGIN: i32 = 5;

const MOVE_NAMES: [&str; 16] = [
    "U", "R", "F", "L", "B", "BL", "D", "BR", "U'", "R'", "F'", "L'", "B'", "BL'", "D'", "BR'",
];

/// The Face Turning Octahedron puzzle.
#[derive(Debug, Default)]
pub struct FtoPuzzle {
    searchers: PerThread<Search>,
}

impl FtoPuzzle {
    /// Creates the puzzle.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Puzzle for FtoPuzzle {
    type State = FtoState;

    fn short_name(&self) -> &str {
        "fto"
    }

    fn long_name(&self) -> &str {
        "Face Turning Octahedron"
    }

    fn wca_min_scramble_distance(&self) -> i32 {
        2
    }

    fn default_color_scheme_entries(&self) -> &'static [(&'static str, Color)] {
        &DEFAULT_COLOR_SCHEME
    }

    fn preferred_size(&self) -> Dimension {
        Dimension::new(
            3 * 2 * STICKER_SIZE + CENTER_GAP_SIZE + 2 * MARGIN,
            3 * STICKER_SIZE + 2 * MARGIN,
        )
    }

    fn solved_state(&self) -> FtoState {
        let mut image = [[0; 9]; 8];
        for (face, stickers) in image.iter_mut().enumerate() {
            stickers.fill(face as u8);
        }
        FtoState { image }
    }

    /// 34 moves make a decently random random-turn scramble (not used for WCA scrambles).
    fn random_move_count(&self) -> i32 {
        34
    }

    fn generate_random_moves(&self, r: &mut dyn RandomSource) -> PuzzleStateAndGenerator<FtoState> {
        let random_state = FtoCubie::random(r);
        let scramble = self
            .searchers
            .with(Search::new, |s| s.solution(&random_state))
            .trim()
            .to_owned();
        let state = self
            .solved_state()
            .apply_algorithm(&scramble)
            .expect("generated FTO scrambles are valid");
        PuzzleStateAndGenerator {
            state,
            generator: scramble,
        }
    }
}

/// The stickers of an FTO: 8 faces of 9 triangles.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FtoState {
    image: [[u8; 9]; 8],
}

type Point = [f64; 2];

fn rotate_point(p: Point, center: Point) -> Point {
    let (x, y) = (p[0], p[1]);
    let (h, k) = (center[0], center[1]);
    [h - (y - k), k + (x - h)]
}

fn translate_point_right(p: Point) -> Point {
    [
        p[0] + f64::from(STICKER_SIZE * 3) + f64::from(CENTER_GAP_SIZE),
        p[1],
    ]
}

/// Barycentric interpolation of the triangle `a b c` with weights `i/3` and `j/3`.
fn face_point(a: Point, b: Point, c: Point, i: i32, j: i32) -> Point {
    let b_weight = f64::from(i) / 3.0;
    let c_weight = f64::from(j) / 3.0;
    let a_weight = 1.0 - b_weight - c_weight;
    [
        a_weight * a[0] + b_weight * b[0] + c_weight * c[0],
        a_weight * a[1] + b_weight * b[1] + c_weight * c[1],
    ]
}

fn sticker(a: Point, b: Point, c: Point, color: Option<Color>) -> Element {
    let mut p = Element::path();
    p.set_stroke(Some(Color::BLACK));
    p.set_fill(color);
    p.move_to(a[0], a[1]);
    p.line_to(b[0], b[1]);
    p.line_to(c[0], c[1]);
    p.close_path();
    p
}

impl FtoState {
    /// The colour (as a face index in [`MOVE_NAMES`] order) of a sticker.
    pub fn sticker(&self, face: usize, sticker: usize) -> u8 {
        self.image[face][sticker]
    }

    fn three_cycle(&mut self, f1: usize, s1: usize, f2: usize, s2: usize, f3: usize, s3: usize) {
        let temp = self.image[f3][s3];
        self.image[f3][s3] = self.image[f2][s2];
        self.image[f2][s2] = self.image[f1][s1];
        self.image[f1][s1] = temp;
    }

    fn turn(&mut self, side: usize, dir: usize) {
        for _ in 0..dir % 3 {
            self.turn_once(side);
        }
    }

    fn turn_once(&mut self, side: usize) {
        const CYCLES: [[[usize; 6]; 9]; 8] = [
            // U
            [
                [0, 0, 0, 5, 0, 2],
                [0, 6, 0, 7, 0, 3],
                [0, 1, 0, 8, 0, 4],
                [2, 1, 5, 8, 7, 4],
                [1, 1, 3, 4, 4, 8],
                [1, 8, 3, 1, 4, 4],
                [1, 6, 3, 3, 4, 7],
                [1, 0, 3, 2, 4, 5],
                [1, 5, 3, 0, 4, 2],
            ],
            // R
            [
                [1, 0, 1, 5, 1, 2],
                [1, 3, 1, 6, 1, 7],
                [1, 1, 1, 8, 1, 4],
                [2, 6, 0, 3, 7, 7],
                [2, 1, 0, 4, 7, 8],
                [0, 1, 7, 4, 2, 8],
                [0, 0, 7, 2, 2, 5],
                [2, 0, 0, 2, 7, 5],
                [3, 1, 4, 8, 6, 4],
            ],
            // F
            [
                [2, 2, 2, 0, 2, 5],
                [2, 3, 2, 6, 2, 7],
                [2, 1, 2, 8, 2, 4],
                [3, 6, 1, 3, 6, 7],
                [3, 0, 1, 2, 6, 5],
                [3, 1, 1, 4, 6, 8],
                [3, 5, 1, 0, 6, 2],
                [3, 8, 1, 1, 6, 4],
                [0, 1, 7, 8, 5, 4],
            ],
            // L
            [
                [3, 3, 3, 6, 3, 7],
                [3, 2, 3, 0, 3, 5],
                [3, 4, 3, 1, 3, 8],
                [0, 6, 2, 3, 5, 7],
                [0, 8, 2, 1, 5, 4],
                [0, 1, 2, 4, 5, 8],
                [1, 1, 6, 8, 4, 4],
                [0, 5, 2, 0, 5, 2],
                [0, 0, 2, 2, 5, 5],
            ],
            // B
            [
                [4, 0, 4, 5, 4, 2],
                [4, 6, 4, 7, 4, 3],
                [4, 1, 4, 8, 4, 4],
                [5, 6, 7, 3, 0, 7],
                [5, 5, 7, 0, 0, 2],
                [5, 8, 7, 1, 0, 4],
                [5, 0, 7, 2, 0, 5],
                [5, 1, 7, 4, 0, 8],
                [6, 1, 1, 8, 3, 4],
            ],
            // BL
            [
                [5, 0, 5, 5, 5, 2],
                [5, 6, 5, 7, 5, 3],
                [5, 1, 5, 8, 5, 4],
                [6, 6, 4, 3, 3, 7],
                [6, 5, 4, 0, 3, 2],
                [6, 8, 4, 1, 3, 4],
                [6, 0, 4, 2, 3, 5],
                [6, 1, 4, 4, 3, 8],
                [7, 1, 0, 8, 2, 4],
            ],
            // D
            [
                [6, 0, 6, 5, 6, 2],
                [6, 3, 6, 6, 6, 7],
                [6, 1, 6, 8, 6, 4],
                [7, 6, 5, 3, 2, 7],
                [7, 0, 5, 2, 2, 5],
                [7, 1, 5, 4, 2, 8],
                [7, 5, 5, 0, 2, 2],
                [7, 8, 5, 1, 2, 4],
                [1, 4, 4, 1, 3, 8],
            ],
            // BR
            [
                [7, 2, 7, 0, 7, 5],
                [7, 3, 7, 6, 7, 7],
                [7, 4, 7, 1, 7, 8],
                [4, 6, 6, 3, 1, 7],
                [4, 5, 6, 0, 1, 2],
                [4, 8, 6, 1, 1, 4],
                [1, 5, 4, 0, 6, 2],
                [1, 8, 4, 1, 6, 4],
                [2, 8, 0, 4, 5, 1],
            ],
        ];
        for c in &CYCLES[side] {
            self.three_cycle(c[0], c[1], c[2], c[3], c[4], c[5]);
        }
    }

    fn draw_face(
        svg: &mut Svg,
        scheme: &[Option<Color>; 8],
        sticker_colors: &[u8; 9],
        right_side: bool,
        direction: usize,
    ) {
        let m = f64::from(MARGIN);
        let s = f64::from(STICKER_SIZE);
        let gap = f64::from(FACE_GAP_SIZE);
        let mut a = [m + s * 1.5, m + s * 1.5 - gap];
        let mut b = [m, m - gap];
        let mut c = [3.0 * s + m, m - gap];
        let center = [m + s * 1.5, m + s * 1.5];
        for _ in 0..direction {
            a = rotate_point(a, center);
            b = rotate_point(b, center);
            c = rotate_point(c, center);
        }
        if right_side {
            a = translate_point_right(a);
            b = translate_point_right(b);
            c = translate_point_right(c);
        }

        // The outline of the face.
        let mut p = Element::path();
        p.move_to(a[0], a[1]);
        p.line_to(b[0], b[1]);
        p.line_to(c[0], c[1]);
        p.close_path();
        p.set_fill(Some(Color::WHITE));
        p.set_stroke(Some(Color::BLACK));
        svg.append_child(p);

        let mut index = 0;
        for i in 0..3 {
            for j in 0..3 - i {
                if i + j < 2 {
                    // A center sticker.
                    let color = scheme[sticker_colors[index] as usize];
                    index += 1;
                    svg.append_child(sticker(
                        face_point(a, b, c, i + 1, j),
                        face_point(a, b, c, i, j + 1),
                        face_point(a, b, c, i + 1, j + 1),
                        color,
                    ));
                }
                // A corner or edge sticker.
                let color = scheme[sticker_colors[index] as usize];
                index += 1;
                svg.append_child(sticker(
                    face_point(a, b, c, i, j),
                    face_point(a, b, c, i + 1, j),
                    face_point(a, b, c, i, j + 1),
                    color,
                ));
            }
        }
    }
}

impl PuzzleState for FtoState {
    /// In the iteration order of TNoodle's `HashMap` (keyed by move name).
    fn successors_by_name(&self) -> Vec<(String, Self)> {
        java_hash_order((0..8).flat_map(|face| {
            (1..3).map(move |dir| {
                let mut successor = self.clone();
                successor.turn(face, dir);
                (MOVE_NAMES[face + 8 * (dir - 1)].to_owned(), successor)
            })
        }))
    }

    fn java_hash_code(&self) -> i32 {
        deep_hash_2d(self.image.as_flattened(), 9)
    }

    fn solved(&self) -> Self {
        FtoPuzzle::new().solved_state()
    }

    fn draw(&self, scheme: &ColorScheme) -> Svg {
        let mut svg = Svg::new(FtoPuzzle::new().preferred_size());
        svg.root_mut().set_stroke_style(2, 10, "round");
        let colors: [Option<Color>; 8] =
            std::array::from_fn(|i| scheme.get(MOVE_NAMES[i]).copied());
        for face in 0..8 {
            Self::draw_face(&mut svg, &colors, &self.image[face], face > 3, face % 4);
        }
        svg
    }
}
