//! Chen Shuang's min2phase: a fast implementation of Kociemba's two-phase algorithm for
//! the 3x3x3, used by TNoodle for 3x3x3 random-state scrambles.
//!
//! Cubes are described by 54 facelets in `URFDLB` face order; see [`tools`] for helpers.

mod coord_cube;
mod cubie_cube;
mod search;
pub mod tools;
mod util;

pub use search::{
    APPEND_LENGTH, INVERSE_SOLUTION, OPTIMAL_SOLUTION, Search, SearchWca, USE_SEPARATOR,
};
