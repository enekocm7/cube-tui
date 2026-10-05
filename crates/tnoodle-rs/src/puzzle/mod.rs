//! TNoodle's puzzles (the `org.worldcubeassociation.tnoodle.puzzle` package).

mod clock;
mod cube;
mod fto;
mod megaminx;
mod pyraminx;
mod pyraminx_solver;
mod search_random;
mod skewb;
mod skewb_solver;
mod square_one;
mod two_by_two_solver;

pub use clock::{ClockPuzzle, ClockState};
pub use cube::{CubeMove, CubePuzzle, CubeState, CubeVariant, Face};
pub use fto::{FtoPuzzle, FtoState};
pub use megaminx::{MegaminxPuzzle, MegaminxState, MinxFace};
pub use pyraminx::{PyraminxPuzzle, PyraminxState};
pub use pyraminx_solver::{PyraminxSolver, PyraminxSolverState};
pub use skewb::{SkewbPuzzle, SkewbState};
pub use skewb_solver::{SkewbSolver, SkewbSolverState};
pub use square_one::{SquareOnePuzzle, SquareOneState};
pub use two_by_two_solver::{TwoByTwoSolver, TwoByTwoState};

/// Coordinate helpers of the 2x2x2 solver, exposed for tests and tools.
pub mod two_by_two {
    pub use super::two_by_two_solver::{move_tables, pack_orient, pack_perm};
}

/// Coordinate helpers of the Pyraminx solver, exposed for tests and tools.
pub mod pyraminx_coords {
    pub use super::pyraminx_solver::{
        move_tables, pack_corner_orient, pack_edge_orient, pack_edge_perm,
    };
}
