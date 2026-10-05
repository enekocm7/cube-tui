//! Statistical checks that 3x3x3 scrambles produce uniformly random states
//! (TNoodle's `scrambleanalysis` module).
//!
//! The checks look at edge and corner orientation, where each piece ends up, and parity,
//! and compare them with the distributions of uniformly random states.

pub mod statistics;

use std::fmt;
use std::path::Path;

use thiserror::Error;

use crate::InvalidScrambleError;
use crate::java::RandomSource;
use crate::min2phase;
use crate::puzzle::{CubePuzzle, CubeState};
use crate::scrambles::{Puzzle, PuzzleState};

/// A facelet string that does not describe a 3x3x3 (`RepresentationException`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("the facelets do not describe a valid 3x3x3")]
pub struct RepresentationError;

const EDGES: usize = 12;
const CORNERS: usize = 8;
const CENTRAL: usize = 4;
const STICKERS_PER_FACE: usize = 9;

/// Edge sticker indices (in `toFaceCube` order): UB, UL, UR, UF, DF, DL, DR, DB, FL, FR,
/// BR, BL.
pub const EDGES_INDEX: [usize; EDGES] = [1, 3, 5, 7, 28, 30, 32, 34, 21, 23, 48, 50];
/// The other sticker of each edge in [`EDGES_INDEX`].
pub const ATTACHED_EDGES_INDEX: [usize; EDGES] = [46, 37, 10, 19, 25, 43, 16, 52, 41, 12, 14, 39];
/// U/D sticker indices of the corners: UBL, UBR, UFL, UFR, DFL, DFR, DBL, DBR.
const CORNERS_INDEX: [usize; CORNERS] = [0, 2, 6, 8, 27, 29, 33, 35];
const CORNERS_INDEX_CLOCKWISE: [usize; CORNERS] = [36, 45, 18, 9, 44, 26, 53, 17];
const CORNERS_INDEX_COUNTER_CLOCKWISE: [usize; CORNERS] = [47, 11, 38, 20, 24, 15, 42, 51];

fn chars(representation: &str) -> Result<Vec<char>, RepresentationError> {
    let chars: Vec<char> = representation.chars().collect();
    if chars.len() == 54 {
        Ok(chars)
    } else {
        Err(RepresentationError)
    }
}

/// Whether the edge at position `index` is oriented with respect to the F/B axis.
pub fn is_oriented_edge(representation: &str, index: usize) -> Result<bool, RepresentationError> {
    let r = chars(representation)?;
    let u = r[CENTRAL];
    let right = r[CENTRAL + STICKERS_PER_FACE];
    let d = r[CENTRAL + 3 * STICKERS_PER_FACE];
    let l = r[CENTRAL + 4 * STICKERS_PER_FACE];
    let color = r[EDGES_INDEX[index]];
    let attached = r[ATTACHED_EDGES_INDEX[index]];
    if color == u || color == d {
        return Ok(true);
    }
    if color == right || color == l {
        return Ok(false);
    }
    // Only the F and B colours are left.
    if attached == u || attached == d {
        Ok(false)
    } else if attached == right || attached == l {
        Ok(true)
    } else {
        Err(RepresentationError)
    }
}

/// The number of edges that are misoriented with respect to the F/B axis.
pub fn count_misoriented_edges(representation: &str) -> Result<usize, RepresentationError> {
    let mut count = 0;
    for i in 0..EDGES {
        if !is_oriented_edge(representation, i)? {
            count += 1;
        }
    }
    Ok(count)
}

/// The orientation of corner `corner_index`: 0 oriented, 1 clockwise, 2 counter-clockwise.
pub fn corner_orientation_number(
    representation: &str,
    corner_index: usize,
) -> Result<usize, RepresentationError> {
    let r = chars(representation)?;
    let u = r[CENTRAL];
    let d = r[CENTRAL + 3 * STICKERS_PER_FACE];
    let is_ud = |c: char| c == u || c == d;
    if is_ud(r[CORNERS_INDEX[corner_index]]) {
        Ok(0)
    } else if is_ud(r[CORNERS_INDEX_CLOCKWISE[corner_index]]) {
        Ok(1)
    } else if is_ud(r[CORNERS_INDEX_COUNTER_CLOCKWISE[corner_index]]) {
        Ok(2)
    } else {
        Err(RepresentationError)
    }
}

/// The sum of all corner orientations.
pub fn corner_orientation_sum(representation: &str) -> Result<usize, RepresentationError> {
    (0..CORNERS)
        .map(|i| corner_orientation_number(representation, i))
        .sum()
}

/// Whether the corner (or, equivalently, edge) permutation is odd.
///
/// # Panics
///
/// Panics if min2phase cannot parse the facelets, like TNoodle.
pub fn has_parity(representation: &str) -> bool {
    let mut search = min2phase::Search::new();
    let errors = search.verify(representation);
    assert!(
        errors == 0,
        "min2phase cannot handle the cube: Error {errors}"
    );
    let (edge_parity, corner_parity) = search.last_cube_parities();
    edge_parity == 1 || corner_parity == 1
}

/// Where the edge that is at position `i` when solved has moved to.
pub fn final_position_of_edge(
    representation: &str,
    i: usize,
) -> Result<usize, RepresentationError> {
    assert!(i < EDGES, "Make sure 0 <= i <= 11.");
    let r = chars(representation)?;
    let center_of = |index: usize| r[CENTRAL + index / STICKERS_PER_FACE * STICKERS_PER_FACE];
    let initial = center_of(EDGES_INDEX[i]);
    let initial_attached = center_of(ATTACHED_EDGES_INDEX[i]);
    (0..EDGES)
        .find(|&j| {
            let (c, a) = (r[EDGES_INDEX[j]], r[ATTACHED_EDGES_INDEX[j]]);
            (c == initial && a == initial_attached) || (c == initial_attached && a == initial)
        })
        .ok_or(RepresentationError)
}

/// Whether two strings consist of the same characters, in any order.
pub fn string_compare_ignoring_order(a: &str, b: &str) -> bool {
    let mut a: Vec<char> = a.chars().collect();
    let mut b: Vec<char> = b.chars().collect();
    a.sort_unstable();
    b.sort_unstable();
    a == b
}

/// Where the corner that is at position `i` when solved has moved to.
pub fn final_position_of_corner(
    representation: &str,
    i: usize,
) -> Result<usize, RepresentationError> {
    assert!(i < CORNERS, "Make sure 0 <= i <= 7.");
    let r = chars(representation)?;
    let center_of = |index: usize| r[index - index % STICKERS_PER_FACE + CENTRAL];
    let initial: String = [
        CORNERS_INDEX[i],
        CORNERS_INDEX_CLOCKWISE[i],
        CORNERS_INDEX_COUNTER_CLOCKWISE[i],
    ]
    .map(center_of)
    .iter()
    .collect();
    (0..CORNERS)
        .find(|&j| {
            let current: String = [
                CORNERS_INDEX[j],
                CORNERS_INDEX_CLOCKWISE[j],
                CORNERS_INDEX_COUNTER_CLOCKWISE[j],
            ]
            .map(|index| r[index])
            .iter()
            .collect();
            string_compare_ignoring_order(&initial, &current)
        })
        .ok_or(RepresentationError)
}

/// The outcome of [`test_scrambles`].
#[allow(
    clippy::struct_excessive_bools,
    reason = "one flag per statistical test"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisReport {
    /// Edge orientation follows the random state distribution.
    pub random_edge_orientation: bool,
    /// Every edge ends up in every position uniformly.
    pub edges_in_random_position: bool,
    /// Corner orientation follows the random state distribution.
    pub random_corner_orientation: bool,
    /// Every corner ends up in every position uniformly.
    pub corners_in_random_position: bool,
    /// Odd and even permutations are equally likely.
    pub random_parity: bool,
}

impl AnalysisReport {
    /// Whether every test passed.
    pub fn passed(&self) -> bool {
        self.random_edge_orientation
            && self.edges_in_random_position
            && self.random_corner_orientation
            && self.corners_in_random_position
            && self.random_parity
    }
}

impl fmt::Display for AnalysisReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Random EO? {}", self.random_edge_orientation)?;
        writeln!(
            f,
            "Edges in random position? {}",
            self.edges_in_random_position
        )?;
        writeln!(f, "Random CO? {}", self.random_corner_orientation)?;
        writeln!(
            f,
            "Corners in random position? {}",
            self.corners_in_random_position
        )?;
        write!(f, "Random parity? {}", self.random_parity)
    }
}

/// Why scrambles could not be analysed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AnalysisError {
    /// Fewer states than [`statistics::minimum_sample_size`].
    #[error("Minimum sample size is {0}")]
    SampleTooSmall(u64),
    /// A state could not be read.
    #[error(transparent)]
    Representation(#[from] RepresentationError),
}

/// Tests whether the states look like uniformly random 3x3x3 states, at significance 0.01
/// (`CubeTest.testScrambles`).
pub fn test_scrambles(states: &[CubeState]) -> Result<AnalysisReport, AnalysisError> {
    let n = states.len() as u64;
    let minimum = statistics::minimum_sample_size();
    if n < minimum {
        return Err(AnalysisError::SampleTooSmall(minimum));
    }
    let mut misoriented_edges = [0_u64; 7];
    let mut final_edges = [[0_u64; EDGES]; EDGES];
    let mut misoriented_corners = [0_u64; 6];
    let mut final_corners = [[0_u64; CORNERS]; CORNERS];
    let mut parity = 0;
    for state in states {
        let representation = state.to_face_cube();
        misoriented_edges[count_misoriented_edges(&representation)? / 2] += 1;
        misoriented_corners[corner_orientation_sum(&representation)? / 3] += 1;
        for (j, counts) in final_edges.iter_mut().enumerate() {
            counts[final_position_of_edge(&representation, j)?] += 1;
        }
        for (j, counts) in final_corners.iter_mut().enumerate() {
            counts[final_position_of_corner(&representation, j)?] += 1;
        }
        if has_parity(&representation) {
            parity += 1;
        }
    }
    let alpha = 0.01;
    let expected_edges = statistics::expected_edges_final_position(n);
    let expected_corners = statistics::expected_corners_final_position(n);
    Ok(AnalysisReport {
        random_edge_orientation: !statistics::chi_square_rejects(
            &statistics::expected_edges_orientation_probability(),
            &misoriented_edges,
            alpha,
        ),
        edges_in_random_position: final_edges
            .iter()
            .all(|counts| !statistics::chi_square_data_sets_differ(&expected_edges, counts, alpha)),
        random_corner_orientation: !statistics::chi_square_rejects(
            &statistics::expected_corners_orientation_probability(),
            &misoriented_corners,
            alpha,
        ),
        corners_in_random_position: final_corners.iter().all(|counts| {
            !statistics::chi_square_data_sets_differ(&expected_corners, counts, alpha)
        }),
        random_parity: statistics::binomial_test_two_sided(n, parity, 0.5) >= alpha,
    })
}

/// Reads one scramble per non-empty line of a file (`ScrambleProvider.getScrambles`).
pub fn read_scrambles(path: &Path) -> std::io::Result<Vec<String>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Generates `n` WCA scrambles of `puzzle`.
pub fn generate_wca_scrambles(
    puzzle: &CubePuzzle,
    n: usize,
    r: &mut dyn RandomSource,
) -> Vec<String> {
    (0..n).map(|_| puzzle.generate_wca_scramble(r)).collect()
}

/// Applies each scramble to a solved 3x3x3.
pub fn convert_to_cube_states(
    scrambles: &[String],
) -> Result<Vec<CubeState>, InvalidScrambleError> {
    let puzzle = CubePuzzle::new(3);
    scrambles
        .iter()
        .map(|s| puzzle.solved_state().apply_algorithm(s))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn representation(scramble: &str) -> String {
        CubePuzzle::three_by_three()
            .solved_state()
            .apply_algorithm(scramble)
            .unwrap()
            .to_face_cube()
    }

    #[test]
    fn parity() {
        assert!(has_parity(&representation("U")));
        assert!(has_parity(&representation("U'")));
        assert!(!has_parity(&representation("U2")));
        assert!(has_parity(&representation(
            "F R U' R' U' R U R' F' R U R' U' R' F R F'"
        )));
        assert!(!has_parity(&representation("R2 U' R' U' R U R U R U' R")));
    }

    #[test]
    fn misoriented_edges() {
        assert_eq!(count_misoriented_edges(&representation("F")), Ok(4));
        assert_eq!(count_misoriented_edges(&representation("F' B")), Ok(8));
        assert_eq!(count_misoriented_edges(&representation("F U F")), Ok(2));
        assert_eq!(count_misoriented_edges("short"), Err(RepresentationError));
    }

    #[test]
    fn oriented_edges() {
        let r = representation("F B'");
        for i in [1, 2, 5, 6] {
            assert_eq!(is_oriented_edge(&r, i), Ok(true), "edge {i}");
        }
        for i in [0, 3, 4, 7, 8, 9, 10, 11] {
            assert_eq!(is_oriented_edge(&r, i), Ok(false), "edge {i}");
        }
    }

    #[test]
    fn final_positions() {
        let r1 = representation("U2");
        let r2 = representation("R U R' U R U2 R'");
        assert_eq!(final_position_of_edge(&r1, 0), Ok(3));
        assert_eq!(final_position_of_edge(&r2, 0), Ok(1));
        assert_eq!(final_position_of_corner(&r1, 0), Ok(3));
        assert_eq!(final_position_of_corner(&r2, 0), Ok(3));
        assert_eq!(final_position_of_corner(&representation("R"), 1), Ok(7));
        assert_eq!(final_position_of_corner(&representation("R'"), 1), Ok(3));
    }

    #[test]
    fn orientation_sums() {
        let r = representation("R U F D L B R2");
        assert_eq!(count_misoriented_edges(&r).unwrap() % 2, 0);
        assert_eq!(corner_orientation_sum(&r).unwrap() % 3, 0);
    }

    #[test]
    fn string_comparison() {
        assert!(string_compare_ignoring_order("FRU", "RUF"));
        assert!(string_compare_ignoring_order("UBL", "LBU"));
        assert!(string_compare_ignoring_order("ABC", "BAC"));
        assert!(!string_compare_ignoring_order("FRU", "FRR"));
        assert!(!string_compare_ignoring_order("AA", "AAA"));
        assert!(!string_compare_ignoring_order("FRU", "fru"));
    }

    #[test]
    fn small_samples_are_rejected() {
        let states = convert_to_cube_states(&["R".to_owned()]).unwrap();
        assert_eq!(
            test_scrambles(&states),
            Err(AnalysisError::SampleTooSmall(6144))
        );
    }

    #[test]
    fn reads_scramble_files() {
        let dir = std::env::temp_dir().join(format!("tnoodle-rs-analysis-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("scrambles.txt");
        std::fs::write(&path, "R U\n\n  F2  \n").unwrap();
        assert_eq!(read_scrambles(&path).unwrap(), ["R U", "F2"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
