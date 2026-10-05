//! Levi Gibson's three phase Face Turning Octahedron solver, used by TNoodle for FTO
//! random-state scrambles.

mod coord;
mod cubie;
mod search;
mod util;

pub use cubie::{FtoCubie, moves};
pub use search::{G1_MOVESET, G2_MOVESET, G3_MOVESET, Search};

/// Parses a move string such as `"R U' BL"` into the resulting cubie.
///
/// # Panics
///
/// Panics on an unrecognised move.
pub fn from_alg(alg: &str) -> FtoCubie {
    util::from_alg(alg)
}
