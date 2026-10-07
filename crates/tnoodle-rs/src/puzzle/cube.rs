//! NxNxN cubes (`CubePuzzle` and its 2x2x2, 3x3x3, 4x4x4 and blindfolded/FMC variants).

use std::borrow::Cow;
use std::sync::LazyLock;
use std::time::Duration;

use super::two_by_two_solver::{self, TwoByTwoSolver, TwoByTwoState};
use crate::java::{RandomSource, array_hash};
use crate::min2phase::{INVERSE_SOLUTION, SearchWca, tools as min2phase_tools};
use crate::scrambles::per_thread::PerThread;
use crate::scrambles::{
    AlgorithmBuilder, ColorScheme, MergingMode, Puzzle, PuzzleState, PuzzleStateAndGenerator,
    first_canonical_move_to, generate_random_turns, order_by_state_hash, split_algorithm,
};
use crate::svg::{Color, Dimension, Element, Svg};
use crate::threephase;

/// A face of a cube, in TNoodle's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Face {
    /// Right
    R,
    /// Up
    U,
    /// Front
    F,
    /// Left
    L,
    /// Down
    D,
    /// Back
    B,
}

impl Face {
    /// All faces in order.
    pub const ALL: [Self; 6] = [Self::R, Self::U, Self::F, Self::L, Self::D, Self::B];

    /// The opposite face.
    #[must_use]
    pub const fn opposite(self) -> Self {
        Self::ALL[(self as usize + 3) % 6]
    }

    /// The face's letter.
    pub const fn name(self) -> &'static str {
        match self {
            Self::R => "R",
            Self::U => "U",
            Self::F => "F",
            Self::L => "L",
            Self::D => "D",
            Self::B => "B",
        }
    }

    fn from_letter(c: char) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name().starts_with(c))
    }

    fn index(self) -> usize {
        self as usize
    }
}

const DIR_TO_STR: [&str; 4] = ["", "", "2", "'"];

/// A turn of the `inner_slice + 1` outermost layers of a face (`CubeMove`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CubeMove {
    /// The turned face.
    pub face: Face,
    /// Quarter turns clockwise: 1, 2 or 3.
    pub dir: usize,
    /// The innermost turned layer (0 = only the face).
    pub inner_slice: usize,
}

impl CubeMove {
    /// A move of a cube of size `size`.
    pub const fn new(face: Face, dir: usize, inner_slice: usize) -> Self {
        Self {
            face,
            dir,
            inner_slice,
        }
    }

    /// The move's name on a cube of `size`: `R`, `Rw2`, `3Rw'`, or a rotation `x`, `y`,
    /// `z`. Rotations around L, D and B have no name.
    pub fn name(&self, size: usize) -> Option<String> {
        let f = self.face.name();
        let mv = if self.inner_slice == 0 {
            f.to_owned()
        } else if self.inner_slice == 1 {
            format!("{f}w")
        } else if self.inner_slice == size - 1 {
            match self.face {
                Face::R => "x".to_owned(),
                Face::U => "y".to_owned(),
                Face::F => "z".to_owned(),
                _ => return None,
            }
        } else {
            format!("{}{f}w", self.inner_slice + 1)
        };
        Some(format!("{mv}{}", DIR_TO_STR[self.dir]))
    }
}

/// Random-turn scramble lengths by cube size.
const DEFAULT_LENGTHS: [i32; 12] = [0, 0, 25, 25, 40, 60, 80, 100, 120, 140, 160, 180];
const GAP: i32 = 2;
const CUBIE_SIZE: i32 = 10;

const THREE_BY_THREE_MAX_SCRAMBLE_LENGTH: i32 = 21;
const THREE_BY_THREE_TIMEOUT_MS: i64 = 60 * 1000;
const TWO_BY_TWO_MIN_SCRAMBLE_LENGTH: i32 = 11;

static DEFAULT_COLOR_SCHEME: [(&str, Color); 6] = [
    ("B", Color::BLUE),
    ("D", Color::YELLOW),
    ("F", Color::GREEN),
    ("L", Color::ORANGE),
    ("R", Color::RED),
    ("U", Color::WHITE),
];

/// How a cube variant generates scrambles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CubeVariant {
    /// Random turns (e.g. 5x5x5, 6x6x6, 7x7x7, or the unofficial fast 4x4x4).
    RandomTurns,
    /// Random state 2x2x2.
    TwoByTwo,
    /// Random state 3x3x3.
    ThreeByThree,
    /// Random state 3x3x3 with a random orientation (3x3x3 blindfolded).
    ThreeByThreeNoInspection,
    /// Random state 3x3x3 padded with `R' U' F` (fewest moves).
    ThreeByThreeFewestMoves,
    /// Random state 4x4x4.
    FourByFour,
    /// Random state 4x4x4 with a random orientation (4x4x4 blindfolded).
    FourByFourNoInspection,
    /// Random turn 5x5x5 with a random orientation (5x5x5 blindfolded).
    FiveByFiveNoInspection,
}

/// Which solver a cube state uses to find short solutions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CubeSolver {
    Bfs,
    TwoByTwo,
    ThreeByThree,
}

/// An NxNxN cube puzzle (`CubePuzzle` and its subclasses).
#[derive(Debug)]
pub struct CubePuzzle {
    size: usize,
    variant: CubeVariant,
    short_name: String,
    long_name: String,
    wca_min_scramble_distance: i32,
    min_search_time: Duration,
    four_searchers: PerThread<threephase::Search>,
}

impl CubePuzzle {
    fn with_variant(
        size: usize,
        variant: CubeVariant,
        short_name: String,
        long_name: String,
    ) -> Self {
        assert!(size < DEFAULT_LENGTHS.len(), "Invalid cube size");
        let mut wca_min_scramble_distance = 2;
        if variant == CubeVariant::TwoByTwo {
            wca_min_scramble_distance = 4;
        }
        if matches!(
            variant,
            CubeVariant::ThreeByThree
                | CubeVariant::ThreeByThreeNoInspection
                | CubeVariant::ThreeByThreeFewestMoves
        ) && let Some(d) = std::env::var("TNOODLE_333_MIN_DISTANCE")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            wca_min_scramble_distance = d;
        }
        Self {
            size,
            variant,
            short_name,
            long_name,
            wca_min_scramble_distance,
            min_search_time: Duration::ZERO,
            four_searchers: PerThread::default(),
        }
    }

    /// A random-turn NxNxN cube (`new CubePuzzle(size)`), for `size < 12`.
    ///
    /// # Panics
    ///
    /// Panics if `size >= 12`.
    pub fn new(size: usize) -> Self {
        Self::with_variant(
            size,
            CubeVariant::RandomTurns,
            format!("{size}{size}{size}"),
            format!("{size}x{size}x{size}"),
        )
    }

    /// The official 2x2x2 (random state).
    pub fn two_by_two() -> Self {
        Self::with_variant(2, CubeVariant::TwoByTwo, "222".into(), "2x2x2".into())
    }

    /// The official 3x3x3 (random state).
    ///
    /// The minimum scramble distance can be overridden with the `TNOODLE_333_MIN_DISTANCE`
    /// environment variable, like in TNoodle.
    pub fn three_by_three() -> Self {
        Self::with_variant(3, CubeVariant::ThreeByThree, "333".into(), "3x3x3".into())
    }

    /// 3x3x3 blindfolded: random state plus a random orientation.
    pub fn three_by_three_no_inspection() -> Self {
        Self::with_variant(
            3,
            CubeVariant::ThreeByThreeNoInspection,
            "333ni".into(),
            "3x3x3 no inspection".into(),
        )
    }

    /// 3x3x3 fewest moves: random state padded with `R' U' F` on both sides.
    pub fn three_by_three_fewest_moves() -> Self {
        Self::with_variant(
            3,
            CubeVariant::ThreeByThreeFewestMoves,
            "333fm".into(),
            "3x3x3 Fewest Moves".into(),
        )
    }

    /// The official 4x4x4 (random state).
    pub fn four_by_four() -> Self {
        Self::with_variant(4, CubeVariant::FourByFour, "444".into(), "4x4x4".into())
    }

    /// A light-weight random-turn 4x4x4 (unofficial).
    pub fn four_by_four_random_turns() -> Self {
        Self::with_variant(
            4,
            CubeVariant::RandomTurns,
            "444fast".into(),
            "4x4x4 (fast, unofficial)".into(),
        )
    }

    /// 4x4x4 blindfolded: random state plus a random orientation.
    pub fn four_by_four_no_inspection() -> Self {
        Self::with_variant(
            4,
            CubeVariant::FourByFourNoInspection,
            "444ni".into(),
            "4x4x4 no inspection".into(),
        )
    }

    /// 5x5x5 blindfolded: random turns plus a random orientation.
    pub fn five_by_five_no_inspection() -> Self {
        Self::with_variant(
            5,
            CubeVariant::FiveByFiveNoInspection,
            "555ni".into(),
            "5x5x5 no inspection".into(),
        )
    }

    /// Sets how long the 3x3x3 variants keep looking for shorter solutions after finding
    /// one. The default, [`Duration::ZERO`], uses the first solution, which is fast and makes
    /// scrambles a deterministic function of the random source; TNoodle uses 200ms.
    #[must_use]
    pub fn with_min_search_time(mut self, min_search_time: Duration) -> Self {
        self.min_search_time = min_search_time;
        self
    }

    /// The number of layers.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The scrambling variant.
    pub fn variant(&self) -> CubeVariant {
        self.variant
    }

    fn solver(&self) -> CubeSolver {
        match self.variant {
            CubeVariant::TwoByTwo => CubeSolver::TwoByTwo,
            CubeVariant::ThreeByThree
            | CubeVariant::ThreeByThreeNoInspection
            | CubeVariant::ThreeByThreeFewestMoves => CubeSolver::ThreeByThree,
            _ => CubeSolver::Bfs,
        }
    }

    /// The moves that randomly reorient the cube, by turning `thickness + 1` layers
    /// (`getRandomOrientationMoves`).
    pub fn random_orientation_moves(&self, thickness: usize) -> Vec<Vec<CubeMove>> {
        let u_face = [
            None,
            Some(CubeMove::new(Face::R, 1, thickness)),
            Some(CubeMove::new(Face::R, 2, thickness)),
            Some(CubeMove::new(Face::R, 3, thickness)),
            Some(CubeMove::new(Face::F, 1, thickness)),
            Some(CubeMove::new(Face::F, 3, thickness)),
        ];
        let f_face = [
            None,
            Some(CubeMove::new(Face::U, 1, thickness)),
            Some(CubeMove::new(Face::U, 2, thickness)),
            Some(CubeMove::new(Face::U, 3, thickness)),
        ];
        u_face
            .iter()
            .flat_map(|u| {
                f_face
                    .iter()
                    .map(move |f| u.iter().chain(f.iter()).copied().collect())
            })
            .collect()
    }

    /// Solves `state` in at most `n` moves with min2phase, optionally forbidding the first
    /// and last moves from turning the axis of the given faces (3x3x3 only).
    pub fn solve_in_restricted(
        state: &CubeState,
        n: i32,
        first_axis: Option<&str>,
        last_axis: Option<&str>,
    ) -> Option<String> {
        three_by_three_solve_in(state, n, first_axis, last_axis)
    }

    fn three_random_moves(
        &self,
        r: &mut dyn RandomSource,
        first: Option<&str>,
        last: Option<&str>,
    ) -> PuzzleStateAndGenerator<CubeState> {
        let random_state = min2phase_tools::random_cube(r);
        let scramble = SearchWca::new()
            .solution(
                &random_state,
                THREE_BY_THREE_MAX_SCRAMBLE_LENGTH,
                THREE_BY_THREE_TIMEOUT_MS,
                self.min_search_time.as_millis() as i64,
                INVERSE_SOLUTION,
                first,
                last,
            )
            .trim()
            .to_owned();
        self.canonicalized(&scramble)
    }

    fn canonicalized(&self, scramble: &str) -> PuzzleStateAndGenerator<CubeState> {
        let mut ab =
            AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, self.solved_state());
        ab.append_algorithm(scramble)
            .unwrap_or_else(|e| panic!("Invalid scramble: {scramble} ({e})"));
        ab.state_and_generator()
    }

    fn four_random_moves(&self, r: &mut dyn RandomSource) -> PuzzleStateAndGenerator<CubeState> {
        let scramble = self
            .four_searchers
            .with(threephase::Search::new, |s| s.random_state(r));
        self.canonicalized(&scramble)
    }

    /// Appends a reorientation to a scramble (`NoInspectionFiveByFiveCubePuzzle.applyOrientation`).
    /// Moves at the end of the scramble that would be redundant with the first
    /// reorientation move are discarded.
    pub fn apply_orientation(
        &self,
        orientation: &[CubeMove],
        psag: PuzzleStateAndGenerator<CubeState>,
    ) -> PuzzleStateAndGenerator<CubeState> {
        let Some(first) = orientation.first() else {
            return psag;
        };
        let mut ab = AlgorithmBuilder::with_state(MergingMode::NoMerging, self.solved_state());
        ab.append_algorithm(&psag.generator)
            .expect("a valid scramble");
        let first = self.move_name(first);
        while ab
            .is_redundant(&first)
            .expect("orientation moves are valid")
        {
            let im = ab
                .find_best_index_for_move(&first, MergingMode::CanonicalizeMoves)
                .expect("orientation moves are valid");
            ab.pop_move(im.index);
        }
        for cm in orientation {
            ab.append_move(&self.move_name(cm))
                .expect("orientation moves are valid");
        }
        ab.state_and_generator()
    }

    /// Appends a reorientation to a scramble without any merging
    /// (`NoInspectionFourByFourCubePuzzle.applyOrientation`).
    fn append_orientation(
        &self,
        orientation: &[CubeMove],
        psag: PuzzleStateAndGenerator<CubeState>,
    ) -> PuzzleStateAndGenerator<CubeState> {
        if orientation.is_empty() {
            return psag;
        }
        let mut ab = AlgorithmBuilder::with_state(MergingMode::NoMerging, self.solved_state());
        ab.append_algorithm(&psag.generator)
            .expect("a valid scramble");
        for cm in orientation {
            ab.append_move(&self.move_name(cm))
                .expect("orientation moves are valid");
        }
        ab.state_and_generator()
    }

    fn move_name(&self, m: &CubeMove) -> String {
        m.name(self.size).unwrap_or_else(|| "null".to_owned())
    }

    fn random_orientation(&self, r: &mut dyn RandomSource, thickness: usize) -> Vec<CubeMove> {
        let mut all = self.random_orientation_moves(thickness);
        let i = r.next_int_bounded(all.len() as i32) as usize;
        all.swap_remove(i)
    }
}

impl Puzzle for CubePuzzle {
    type State = CubeState;

    fn short_name(&self) -> &str {
        &self.short_name
    }

    fn long_name(&self) -> &str {
        &self.long_name
    }

    fn wca_min_scramble_distance(&self) -> i32 {
        self.wca_min_scramble_distance
    }

    fn default_color_scheme_entries(&self) -> &'static [(&'static str, Color)] {
        &DEFAULT_COLOR_SCHEME
    }

    fn preferred_size(&self) -> Dimension {
        image_size(GAP, CUBIE_SIZE, self.size as i32)
    }

    fn solved_state(&self) -> CubeState {
        CubeState::solved_with(self.size, self.solver())
    }

    fn random_move_count(&self) -> i32 {
        DEFAULT_LENGTHS[self.size]
    }

    fn initialization_status(&self) -> f64 {
        match self.variant {
            CubeVariant::FourByFour | CubeVariant::FourByFourNoInspection => {
                threephase::init_status()
            }
            _ => 1.0,
        }
    }

    fn generate_random_moves(
        &self,
        r: &mut dyn RandomSource,
    ) -> PuzzleStateAndGenerator<CubeState> {
        match self.variant {
            CubeVariant::RandomTurns => {
                generate_random_turns(self.solved_state(), self.random_move_count(), r)
            }
            CubeVariant::TwoByTwo => {
                let solver = TwoByTwoSolver::new();
                let state = solver.random_state(r);
                let scramble = solver
                    .generate_exactly(state, TWO_BY_TWO_MIN_SCRAMBLE_LENGTH)
                    .expect("every 2x2x2 state can be generated in 11 moves");
                self.canonicalized(&scramble)
            }
            CubeVariant::ThreeByThree => self.three_random_moves(r, None, None),
            CubeVariant::ThreeByThreeNoInspection => {
                let orientation = self.random_orientation(r, self.size / 2);
                // The restriction covers the whole axis, so the orientation can never be
                // redundant with the scramble.
                let first = orientation.first().map(|m| m.face.name());
                let psag = self.three_random_moves(r, first, None);
                self.apply_orientation(&orientation, psag)
            }
            CubeVariant::ThreeByThreeFewestMoves => {
                // "Tom2" scrambles: pad a random-state scramble with R' U' F on both sides
                // so that no move cancels; this hides the orientation step of two-phase
                // solutions without changing the distribution of states.
                let prefix = split_algorithm("R' U' F");
                let suffix = split_algorithm("R' U' F");
                let last = &prefix[prefix.len() - 1][..1];
                let first = &suffix[0][..1];
                let psag = self.three_random_moves(r, Some(first), Some(last));
                let mut ab =
                    AlgorithmBuilder::with_state(MergingMode::NoMerging, self.solved_state());
                ab.append_algorithms(prefix.iter().copied())
                    .and_then(|()| ab.append_algorithm(&psag.generator))
                    .and_then(|()| ab.append_algorithms(suffix.iter().copied()))
                    .expect("a valid padded scramble");
                ab.state_and_generator()
            }
            CubeVariant::FourByFour => self.four_random_moves(r),
            CubeVariant::FourByFourNoInspection => {
                let orientation = self.random_orientation(r, self.size - 1);
                let psag = self.four_random_moves(r);
                self.append_orientation(&orientation, psag)
            }
            CubeVariant::FiveByFiveNoInspection => {
                let orientation = self.random_orientation(r, self.size / 2);
                let psag = generate_random_turns(self.solved_state(), self.random_move_count(), r);
                self.apply_orientation(&orientation, psag)
            }
        }
    }
}

fn image_size(gap: i32, unit: i32, size: i32) -> Dimension {
    Dimension::new((size * unit + gap) * 4 + gap, (size * unit + gap) * 3 + gap)
}

/// The state of an NxNxN: the colour of every sticker, face by face (in [`Face`] order),
/// row by row.
#[derive(Debug, Clone)]
pub struct CubeState {
    size: usize,
    image: Box<[u8]>,
    solver: CubeSolver,
}

impl PartialEq for CubeState {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.image == other.image
    }
}

impl Eq for CubeState {}

impl std::hash::Hash for CubeState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.image.hash(state);
    }
}

/// The successor moves of every cube size, in TNoodle's order: `(name, move)`.
struct MoveSet {
    by_name: Vec<(String, CubeMove)>,
    scramble: Vec<(String, CubeMove)>,
}

static MOVE_SETS: LazyLock<Vec<MoveSet>> = LazyLock::new(|| {
    (0..DEFAULT_LENGTHS.len())
        .map(|size| MoveSet {
            by_name: moves_within_slice(size, size.saturating_sub(1), true),
            scramble: if size >= 2 {
                moves_within_slice(size, size / 2 - 1, false)
            } else {
                Vec::new()
            },
        })
        .collect()
});

/// `getSuccessorsWithinSlice`: named moves turning at most `max_slice + 1` layers.
fn moves_within_slice(
    size: usize,
    max_slice: usize,
    include_redundant: bool,
) -> Vec<(String, CubeMove)> {
    let mut moves = Vec::new();
    if size == 0 {
        return moves;
    }
    for inner_slice in 0..=max_slice {
        for face in Face::ALL {
            let half_of_even_cube = size.is_multiple_of(2) && inner_slice + 1 == size / 2;
            if !include_redundant && face.index() >= 3 && half_of_even_cube {
                // Skip turning the other halves of even sized cubes.
                continue;
            }
            for dir in 1..=3 {
                let mv = CubeMove::new(face, dir, inner_slice);
                if let Some(name) = mv.name(size) {
                    moves.push((name, mv));
                }
            }
        }
    }
    moves
}

impl CubeState {
    fn solved_with(size: usize, solver: CubeSolver) -> Self {
        let mut image = vec![0; 6 * size * size].into_boxed_slice();
        for (i, v) in image.iter_mut().enumerate() {
            *v = (i / (size * size).max(1)) as u8;
        }
        Self {
            size,
            image,
            solver,
        }
    }

    /// The number of layers.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The colour (as a [`Face`] index) of a sticker.
    pub fn sticker(&self, face: Face, row: usize, col: usize) -> u8 {
        self.image[self.idx(face.index(), row, col)]
    }

    fn idx(&self, face: usize, row: usize, col: usize) -> usize {
        (face * self.size + row) * self.size + col
    }

    fn apply_move(&self, mv: &CubeMove) -> Self {
        let mut next = self.clone();
        for slice_index in 0..=mv.inner_slice {
            next.slice(mv.face, slice_index, mv.dir);
        }
        next
    }

    fn successors(&self, moves: &[(String, CubeMove)]) -> Vec<(String, Self)> {
        moves
            .iter()
            .map(|(name, mv)| (name.clone(), self.apply_move(mv)))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn swap(
        &mut self,
        a: (usize, usize, usize),
        b: (usize, usize, usize),
        c: (usize, usize, usize),
        d: (usize, usize, usize),
        dir: usize,
    ) {
        let ia = self.idx(a.0, a.1, a.2);
        let ib = self.idx(b.0, b.1, b.2);
        let ic = self.idx(c.0, c.1, c.2);
        let id = self.idx(d.0, d.1, d.2);
        let img = &mut self.image;
        match dir {
            1 => {
                let temp = img[ia];
                img[ia] = img[ib];
                img[ib] = img[ic];
                img[ic] = img[id];
                img[id] = temp;
            }
            2 => {
                img.swap(ia, ic);
                img.swap(ib, id);
            }
            3 => {
                let temp = img[id];
                img[id] = img[ic];
                img[ic] = img[ib];
                img[ib] = img[ia];
                img[ia] = temp;
            }
            _ => {}
        }
    }

    /// Turns one layer, `slice` layers in from `face`.
    fn slice(&mut self, face: Face, slice: usize, dir: usize) {
        let size = self.size;
        let (mut sface, mut sslice, mut sdir) = (face, slice, dir);
        if !matches!(face, Face::L | Face::D | Face::B) {
            sface = face.opposite();
            sslice = size - 1 - slice;
            sdir = 4 - dir;
        }
        let (u, r, f, l, d, b) = (1, 0, 2, 3, 4, 5);
        for j in 0..size {
            match sface {
                Face::L => self.swap(
                    (u, j, sslice),
                    (b, size - 1 - j, size - 1 - sslice),
                    (d, j, sslice),
                    (f, j, sslice),
                    sdir,
                ),
                Face::D => self.swap(
                    (l, size - 1 - sslice, j),
                    (b, size - 1 - sslice, j),
                    (r, size - 1 - sslice, j),
                    (f, size - 1 - sslice, j),
                    sdir,
                ),
                Face::B => self.swap(
                    (u, sslice, j),
                    (r, j, size - 1 - sslice),
                    (d, size - 1 - sslice, size - 1 - j),
                    (l, size - 1 - j, sslice),
                    sdir,
                ),
                _ => {}
            }
        }
        if slice == 0 || slice == size - 1 {
            let (fi, sdir) = if slice == 0 {
                (face.index(), 4 - dir)
            } else {
                (face.opposite().index(), dir)
            };
            for j in 0..size.div_ceil(2) {
                for k in 0..size / 2 {
                    self.swap(
                        (fi, j, k),
                        (fi, k, size - 1 - j),
                        (fi, size - 1 - j, size - 1 - k),
                        (fi, size - 1 - k, j),
                        sdir,
                    );
                }
            }
        }
    }

    fn spin(&mut self, face: Face, dir: usize) {
        for slice in 0..self.size {
            self.slice(face, slice, dir);
        }
    }

    /// The colour counts of each face, sorted. Whole-cube rotations only move faces around
    /// and turn them, so states a rotation apart have the same fingerprint.
    fn rotation_invariant_fingerprint(&self) -> [[u8; 6]; 6] {
        let mut counts = [[0_u8; 6]; 6];
        let face_len = self.size * self.size;
        if face_len > 0 {
            for (face, stickers) in counts.iter_mut().zip(self.image.chunks_exact(face_len)) {
                for &s in stickers {
                    face[usize::from(s)] += 1;
                }
            }
        }
        counts.sort_unstable();
        counts
    }

    /// Normalized when the BLD corner is solved.
    fn image_is_normalized(&self) -> bool {
        let s = self.size - 1;
        self.image[self.idx(Face::B.index(), s, s)] == Face::B as u8
            && self.image[self.idx(Face::L.index(), s, 0)] == Face::L as u8
            && self.image[self.idx(Face::D.index(), s, 0)] == Face::D as u8
    }

    /// The stickers of each corner, in TNoodle's corner order (`getStickersByPiece`).
    fn stickers_by_piece(&self) -> [[u8; 3]; 8] {
        use Face::{B, D, F, L, R, U};
        let s = self.size - 1;
        let g = |f: Face, r: usize, c: usize| self.image[self.idx(f.index(), r, c)];
        [
            [g(U, s, s), g(R, 0, 0), g(F, 0, s)],
            [g(U, s, 0), g(F, 0, 0), g(L, 0, s)],
            [g(U, 0, s), g(B, 0, 0), g(R, 0, s)],
            [g(U, 0, 0), g(L, 0, 0), g(B, 0, s)],
            [g(D, 0, s), g(F, s, s), g(R, s, 0)],
            [g(D, 0, 0), g(L, s, s), g(F, s, 0)],
            [g(D, s, s), g(R, s, s), g(B, s, 0)],
            [g(D, s, 0), g(B, s, s), g(L, s, 0)],
        ]
    }

    fn normalize(&self) -> Self {
        let mut image = self.clone();
        while !image.image_is_normalized() {
            let stickers = image.stickers_by_piece();
            let goal = 1 << Face::B as u8 | 1 << Face::L as u8 | 1 << Face::D as u8;
            let idx = stickers
                .iter()
                .position(|p| p.iter().fold(0, |t, &s| t | 1 << s) == goal)
                .expect("a cube has a BLD corner");
            let d = Face::D as u8;
            let (f, dir) = if stickers[idx][0] == d {
                if idx < 4 {
                    (Face::F, 2) // on U
                } else {
                    // on D
                    (
                        Face::U,
                        match idx {
                            4 => 2,
                            6 => 3,
                            _ => 1,
                        },
                    )
                }
            } else if stickers[idx][1] == d {
                (
                    match idx {
                        0 | 6 => Face::F, // on R
                        1 | 4 => Face::L, // on F
                        2 | 7 => Face::R, // on B
                        _ => Face::B,     // on L
                    },
                    1,
                )
            } else {
                (
                    match idx {
                        2 | 4 => Face::F, // on R
                        0 | 5 => Face::L, // on F
                        3 | 6 => Face::R, // on B
                        _ => Face::B,     // on L
                    },
                    1,
                )
            };
            image.spin(f, dir);
        }
        image
    }

    /// The equivalent 2x2x2 solver state (2x2x2 only).
    pub fn to_two_by_two_state(&self) -> TwoByTwoState {
        let stickers = self.stickers_by_piece();
        // Each piece gets a unique id from the sum of its stickers' values.
        let d_color = stickers[7][0] as usize;
        let b_color = stickers[7][1] as usize;
        let l_color = stickers[7][2] as usize;
        let u_color = Face::ALL[d_color].opposite().index();
        let f_color = Face::ALL[b_color].opposite().index();
        let r_color = Face::ALL[l_color].opposite().index();

        let mut color_to_val = [0; 8];
        color_to_val[u_color] = 0;
        color_to_val[f_color] = 0;
        color_to_val[r_color] = 0;
        color_to_val[l_color] = 1;
        color_to_val[b_color] = 2;
        color_to_val[d_color] = 4;

        let mut pieces = [0; 7];
        for (i, piece) in pieces.iter_mut().enumerate() {
            let st = stickers[i];
            let piece_val: i32 = st.iter().map(|&s| color_to_val[s as usize]).sum();
            let mut turns = 0;
            while st[turns] as usize != u_color && st[turns] as usize != d_color {
                turns += 1;
            }
            *piece = ((turns as i32) << 3) + piece_val;
        }
        TwoByTwoState {
            permutation: two_by_two_solver::pack_perm(&pieces),
            orientation: two_by_two_solver::pack_orient(&pieces),
        }
    }

    /// The 54 facelets in `URFDLB` order, each named after the face it belongs to when
    /// solved (3x3x3 only).
    pub fn to_face_cube(&self) -> String {
        let mut state = String::with_capacity(6 * self.size * self.size);
        for c in "URFDLB".chars() {
            let face = Face::from_letter(c).expect("a face letter");
            let start = self.idx(face.index(), 0, 0);
            for &piece in &self.image[start..start + self.size * self.size] {
                state.push_str(Face::ALL[piece as usize].name());
            }
        }
        state
    }

    fn draw_face(&self, svg: &mut Svg, x: i32, y: i32, face: Face, scheme: &ColorScheme) {
        for row in 0..self.size {
            for col in 0..self.size {
                let tempx = x + col as i32 * CUBIE_SIZE;
                let tempy = y + row as i32 * CUBIE_SIZE;
                let mut rect = Element::rectangle(
                    f64::from(tempx),
                    f64::from(tempy),
                    f64::from(CUBIE_SIZE),
                    f64::from(CUBIE_SIZE),
                );
                let sticker = Face::ALL[self.sticker(face, row, col) as usize];
                rect.set_fill(scheme.get(sticker.name()).copied());
                rect.set_stroke(Some(Color::BLACK));
                svg.append_child(rect);
            }
        }
    }
}

fn three_by_three_solve_in(
    state: &CubeState,
    n: i32,
    first: Option<&str>,
    last: Option<&str>,
) -> Option<String> {
    if *state == state.solved() {
        // min2phase can't solve the solved cube.
        return Some(String::new());
    }
    let solution = SearchWca::new()
        .solution(
            &state.to_face_cube(),
            n,
            THREE_BY_THREE_TIMEOUT_MS,
            0,
            0,
            first,
            last,
        )
        .trim()
        .to_owned();
    // "Error 7" means there is no solution of at most n moves; TNoodle treats any other
    // error the same way.
    if solution.starts_with("Error") {
        None
    } else {
        Some(solution)
    }
}

impl PuzzleState for CubeState {
    fn successors_by_name(&self) -> Vec<(String, Self)> {
        self.successors(&MOVE_SETS[self.size].by_name)
    }

    fn scramble_successors(&self) -> Vec<(String, Self)> {
        self.successors(&MOVE_SETS[self.size].scramble)
    }

    /// Like the default, but avoids hashing and normalizing every successor: a state only
    /// matches if its rotation-invariant fingerprint does, and the Java hash order only
    /// matters when several different states match.
    fn canonical_move_to(&self, target: &Self) -> Option<String> {
        let fingerprint = target.rotation_invariant_fingerprint();
        let mut found: Option<(Self, &String)> = None;
        for (name, mv) in &MOVE_SETS[self.size].scramble {
            let state = self.apply_move(mv);
            if state.rotation_invariant_fingerprint() != fingerprint
                || *state.normalized() != *target
            {
                continue;
            }
            match &found {
                // Like `HashMap#put`, a later move to the same state replaces the name.
                Some((s, _)) if *s != state => return first_canonical_move_to(self, target),
                _ => found = Some((state, name)),
            }
        }
        found.map(|(_, name)| name.clone())
    }

    fn scramble_successor_names(&self) -> Vec<String> {
        MOVE_SETS[self.size]
            .scramble
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn canonical_moves_by_state(&self) -> Vec<(Self, String)> {
        order_by_state_hash(
            self.scramble_successors()
                .into_iter()
                .map(|(name, s)| (s, name))
                .collect(),
        )
    }

    fn apply(&self, mv: &str) -> Result<Self, crate::InvalidMoveError> {
        MOVE_SETS[self.size]
            .by_name
            .iter()
            .find(|(name, _)| name == mv)
            .map(|(_, m)| self.apply_move(m))
            .ok_or_else(|| crate::InvalidMoveError::unrecognized(mv))
    }

    fn java_hash_code(&self) -> i32 {
        let face_len = self.size * self.size;
        array_hash((0..6).map(|f| {
            array_hash((0..self.size).map(|r| {
                let start = f * face_len + r * self.size;
                array_hash(
                    self.image[start..start + self.size]
                        .iter()
                        .map(|&v| i32::from(v)),
                )
            }))
        }))
    }

    fn solved(&self) -> Self {
        Self::solved_with(self.size, self.solver)
    }

    fn normalized(&self) -> Cow<'_, Self> {
        if self.size == 0 || self.image_is_normalized() {
            Cow::Borrowed(self)
        } else {
            Cow::Owned(self.normalize())
        }
    }

    fn is_normalized(&self) -> bool {
        self.image_is_normalized()
    }

    fn solve_in(&self, n: i32) -> Option<String> {
        match self.solver {
            CubeSolver::Bfs => crate::scrambles::solve_in_bfs(self, n),
            CubeSolver::TwoByTwo => TwoByTwoSolver::new().solve_in(self.to_two_by_two_state(), n),
            CubeSolver::ThreeByThree => three_by_three_solve_in(self, n, None, None),
        }
    }

    fn draw(&self, scheme: &ColorScheme) -> Svg {
        let size = self.size as i32;
        let mut svg = Svg::new(image_size(GAP, CUBIE_SIZE, size));
        let (g, c) = (GAP, CUBIE_SIZE);
        self.draw_face(&mut svg, g, 2 * g + size * c, Face::L, scheme);
        self.draw_face(
            &mut svg,
            2 * g + size * c,
            3 * g + 2 * size * c,
            Face::D,
            scheme,
        );
        self.draw_face(
            &mut svg,
            4 * g + 3 * size * c,
            2 * g + size * c,
            Face::B,
            scheme,
        );
        self.draw_face(
            &mut svg,
            3 * g + 2 * size * c,
            2 * g + size * c,
            Face::R,
            scheme,
        );
        self.draw_face(&mut svg, 2 * g + size * c, g, Face::U, scheme);
        self.draw_face(
            &mut svg,
            2 * g + size * c,
            2 * g + size * c,
            Face::F,
            scheme,
        );
        svg
    }
}
