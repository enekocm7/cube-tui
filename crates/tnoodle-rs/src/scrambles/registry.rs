//! The WCA puzzles (`PuzzleRegistry`) and a type that can hold any of them.

use std::sync::LazyLock;

use super::image_info::PuzzleImageInfo;
use super::puzzle::{ColorScheme, Puzzle, PuzzleState};
use crate::error::InvalidScrambleError;
use crate::java::RandomSource;
use crate::puzzle::{
    ClockPuzzle, CubePuzzle, FtoPuzzle, MegaminxPuzzle, PyraminxPuzzle, SkewbPuzzle,
    SquareOnePuzzle,
};
use crate::svg::{Dimension, Svg};

/// Any of TNoodle's puzzles, for code that does not know the puzzle type statically.
#[derive(Debug)]
pub enum Scrambler {
    /// NxNxN cubes.
    Cube(CubePuzzle),
    /// Rubik's Clock.
    Clock(ClockPuzzle),
    /// Megaminx.
    Megaminx(MegaminxPuzzle),
    /// Pyraminx.
    Pyraminx(PyraminxPuzzle),
    /// Skewb.
    Skewb(SkewbPuzzle),
    /// Square-1.
    SquareOne(SquareOnePuzzle),
    /// Face Turning Octahedron.
    Fto(FtoPuzzle),
}

/// Calls `$body` with `$p` bound to the inner puzzle, whatever its type.
macro_rules! dispatch {
    ($self:expr, $p:ident => $body:expr) => {
        match $self {
            Scrambler::Cube($p) => $body,
            Scrambler::Clock($p) => $body,
            Scrambler::Megaminx($p) => $body,
            Scrambler::Pyraminx($p) => $body,
            Scrambler::Skewb($p) => $body,
            Scrambler::SquareOne($p) => $body,
            Scrambler::Fto($p) => $body,
        }
    };
}

/// Solves the state reached by `scramble` within `n` moves.
fn solve_scramble_in<P: Puzzle>(
    p: &P,
    scramble: &str,
    n: i32,
) -> Result<Option<String>, InvalidScrambleError> {
    Ok(p.solved_state().apply_algorithm(scramble)?.solve_in(n))
}

impl Scrambler {
    /// A URL friendly name, e.g. `333`.
    pub fn short_name(&self) -> &str {
        dispatch!(self, p => p.short_name())
    }

    /// A human readable name, e.g. `3x3x3`.
    pub fn long_name(&self) -> &str {
        dispatch!(self, p => p.long_name())
    }

    /// The minimum distance from solved of every WCA scramble.
    pub fn wca_min_scramble_distance(&self) -> i32 {
        dispatch!(self, p => p.wca_min_scramble_distance())
    }

    /// How far along the solver initialisation is, from 0 to 1.
    pub fn initialization_status(&self) -> f64 {
        dispatch!(self, p => p.initialization_status())
    }

    /// Generates a WCA scramble from the given randomness.
    pub fn generate_wca_scramble(&self, r: &mut dyn RandomSource) -> String {
        dispatch!(self, p => p.generate_wca_scramble(r))
    }

    /// Generates a WCA scramble from secure randomness.
    pub fn generate_scramble(&self) -> String {
        dispatch!(self, p => p.generate_scramble())
    }

    /// Generates `count` WCA scrambles from secure randomness.
    pub fn generate_scrambles(&self, count: usize) -> Vec<String> {
        dispatch!(self, p => p.generate_scrambles(count))
    }

    /// Generates a WCA scramble determined by `seed` (identical to TNoodle's).
    pub fn generate_seeded_scramble(&self, seed: &str) -> String {
        dispatch!(self, p => p.generate_seeded_scramble(seed))
    }

    /// Generates `count` WCA scrambles determined by `seed` (identical to TNoodle's).
    pub fn generate_seeded_scrambles(&self, seed: &str, count: usize) -> Vec<String> {
        dispatch!(self, p => p.generate_seeded_scrambles(seed, count))
    }

    /// The default colour scheme.
    pub fn default_color_scheme(&self) -> ColorScheme {
        dispatch!(self, p => p.default_color_scheme())
    }

    /// The face names, sorted alphabetically.
    pub fn face_names(&self) -> Vec<String> {
        dispatch!(self, p => p.face_names())
    }

    /// Parses a colour scheme; see [`Puzzle::parse_color_scheme`].
    pub fn parse_color_scheme(&self, scheme: Option<&str>) -> Option<ColorScheme> {
        dispatch!(self, p => p.parse_color_scheme(scheme))
    }

    /// The natural size of the drawing.
    pub fn preferred_size(&self) -> Dimension {
        dispatch!(self, p => p.preferred_size())
    }

    /// The best drawing size within the given bounds; see [`Puzzle::preferred_size_within`].
    pub fn preferred_size_within(&self, max_width: i32, max_height: i32) -> Dimension {
        dispatch!(self, p => p.preferred_size_within(max_width, max_height))
    }

    /// Draws a scramble; see [`Puzzle::draw_scramble`].
    pub fn draw_scramble(
        &self,
        scramble: Option<&str>,
        color_scheme: Option<&ColorScheme>,
    ) -> Result<Svg, InvalidScrambleError> {
        dispatch!(self, p => p.draw_scramble(scramble, color_scheme))
    }

    /// A solution of at most `n` moves for the state reached by `scramble`, if any.
    pub fn solve_scramble_in(
        &self,
        scramble: &str,
        n: i32,
    ) -> Result<Option<String>, InvalidScrambleError> {
        dispatch!(self, p => solve_scramble_in(p, scramble, n))
    }

    /// Whether `scramble` leaves the puzzle solved.
    pub fn is_solved_by(&self, scramble: &str) -> Result<bool, InvalidScrambleError> {
        dispatch!(self, p => Ok(p.solved_state().apply_algorithm(scramble)?.is_solved()))
    }

    /// The default colours and size, for clients that draw puzzles themselves.
    pub fn image_info(&self) -> PuzzleImageInfo {
        dispatch!(self, p => PuzzleImageInfo::new(p))
    }
}

impl std::fmt::Display for Scrambler {
    /// The long name, like Java's `toString`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.long_name())
    }
}

/// The puzzles TNoodle provides (`PuzzleRegistry`).
///
/// ```
/// use tnoodle::scrambles::PuzzleRegistry;
///
/// let three = PuzzleRegistry::Three.scrambler();
/// assert_eq!(three.short_name(), "333");
/// assert_eq!(PuzzleRegistry::from_key("pyram"), Some(PuzzleRegistry::Pyra));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PuzzleRegistry {
    /// 2x2x2
    Two,
    /// 3x3x3
    Three,
    /// 4x4x4
    Four,
    /// 4x4x4 random turns (unofficial)
    FourFast,
    /// 5x5x5
    Five,
    /// 6x6x6
    Six,
    /// 7x7x7
    Seven,
    /// 3x3x3 blindfolded
    ThreeNi,
    /// 4x4x4 blindfolded
    FourNi,
    /// 5x5x5 blindfolded
    FiveNi,
    /// 3x3x3 fewest moves
    ThreeFm,
    /// Pyraminx
    Pyra,
    /// Square-1
    Sq1,
    /// Megaminx
    Mega,
    /// Clock
    Clock,
    /// Skewb
    Skewb,
    /// Face Turning Octahedron
    Fto,
}

macro_rules! lazy_scrambler {
    ($name:ident, $ctor:expr) => {
        static $name: LazyLock<Scrambler> = LazyLock::new(|| $ctor);
    };
}

lazy_scrambler!(TWO, Scrambler::Cube(CubePuzzle::two_by_two()));
lazy_scrambler!(THREE, Scrambler::Cube(CubePuzzle::three_by_three()));
lazy_scrambler!(FOUR, Scrambler::Cube(CubePuzzle::four_by_four()));
lazy_scrambler!(
    FOUR_FAST,
    Scrambler::Cube(CubePuzzle::four_by_four_random_turns())
);
lazy_scrambler!(FIVE, Scrambler::Cube(CubePuzzle::new(5)));
lazy_scrambler!(SIX, Scrambler::Cube(CubePuzzle::new(6)));
lazy_scrambler!(SEVEN, Scrambler::Cube(CubePuzzle::new(7)));
lazy_scrambler!(
    THREE_NI,
    Scrambler::Cube(CubePuzzle::three_by_three_no_inspection())
);
lazy_scrambler!(
    FOUR_NI,
    Scrambler::Cube(CubePuzzle::four_by_four_no_inspection())
);
lazy_scrambler!(
    FIVE_NI,
    Scrambler::Cube(CubePuzzle::five_by_five_no_inspection())
);
lazy_scrambler!(
    THREE_FM,
    Scrambler::Cube(CubePuzzle::three_by_three_fewest_moves())
);
lazy_scrambler!(PYRA, Scrambler::Pyraminx(PyraminxPuzzle::new()));
lazy_scrambler!(SQ1, Scrambler::SquareOne(SquareOnePuzzle::new()));
lazy_scrambler!(MEGA, Scrambler::Megaminx(MegaminxPuzzle::new()));
lazy_scrambler!(CLOCK, Scrambler::Clock(ClockPuzzle::new()));
lazy_scrambler!(SKEWB, Scrambler::Skewb(SkewbPuzzle::new()));
lazy_scrambler!(FTO, Scrambler::Fto(FtoPuzzle::new()));

impl PuzzleRegistry {
    /// Every puzzle, in TNoodle's order.
    pub const ALL: [Self; 17] = [
        Self::Two,
        Self::Three,
        Self::Four,
        Self::FourFast,
        Self::Five,
        Self::Six,
        Self::Seven,
        Self::ThreeNi,
        Self::FourNi,
        Self::FiveNi,
        Self::ThreeFm,
        Self::Pyra,
        Self::Sq1,
        Self::Mega,
        Self::Clock,
        Self::Skewb,
        Self::Fto,
    ];

    /// The shared, lazily created puzzle.
    pub fn scrambler(self) -> &'static Scrambler {
        match self {
            Self::Two => &TWO,
            Self::Three => &THREE,
            Self::Four => &FOUR,
            Self::FourFast => &FOUR_FAST,
            Self::Five => &FIVE,
            Self::Six => &SIX,
            Self::Seven => &SEVEN,
            Self::ThreeNi => &THREE_NI,
            Self::FourNi => &FOUR_NI,
            Self::FiveNi => &FIVE_NI,
            Self::ThreeFm => &THREE_FM,
            Self::Pyra => &PYRA,
            Self::Sq1 => &SQ1,
            Self::Mega => &MEGA,
            Self::Clock => &CLOCK,
            Self::Skewb => &SKEWB,
            Self::Fto => &FTO,
        }
    }

    /// The puzzle's short name (`getKey`).
    pub fn key(self) -> &'static str {
        self.scrambler().short_name()
    }

    /// The puzzle's long name (`getDescription`).
    pub fn description(self) -> &'static str {
        self.scrambler().long_name()
    }

    /// The Java enum constant's name, e.g. `FOUR_FAST`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Two => "TWO",
            Self::Three => "THREE",
            Self::Four => "FOUR",
            Self::FourFast => "FOUR_FAST",
            Self::Five => "FIVE",
            Self::Six => "SIX",
            Self::Seven => "SEVEN",
            Self::ThreeNi => "THREE_NI",
            Self::FourNi => "FOUR_NI",
            Self::FiveNi => "FIVE_NI",
            Self::ThreeFm => "THREE_FM",
            Self::Pyra => "PYRA",
            Self::Sq1 => "SQ1",
            Self::Mega => "MEGA",
            Self::Clock => "CLOCK",
            Self::Skewb => "SKEWB",
            Self::Fto => "FTO",
        }
    }

    /// The puzzle with the given short name, e.g. `333` or `minx`.
    pub fn from_key(key: &str) -> Option<Self> {
        let puzzle = match key {
            "222" => Self::Two,
            "333" => Self::Three,
            "444" => Self::Four,
            "444fast" => Self::FourFast,
            "555" => Self::Five,
            "666" => Self::Six,
            "777" => Self::Seven,
            "333ni" => Self::ThreeNi,
            "444ni" => Self::FourNi,
            "555ni" => Self::FiveNi,
            "333fm" => Self::ThreeFm,
            "pyram" => Self::Pyra,
            "sq1" => Self::Sq1,
            "minx" => Self::Mega,
            "clock" => Self::Clock,
            "skewb" => Self::Skewb,
            "fto" => Self::Fto,
            _ => return None,
        };
        Some(puzzle)
    }
}
