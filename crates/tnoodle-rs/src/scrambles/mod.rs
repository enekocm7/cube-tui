//! The scrambling framework: puzzles, puzzle states, move merging, solving and the registry
//! of WCA puzzles (TNoodle's `org.worldcubeassociation.tnoodle.scrambles` package).

mod algorithm_builder;
mod cacher;
mod image_info;
pub(crate) mod per_thread;
mod puzzle;
mod registry;
mod solve;

pub use algorithm_builder::{AlgorithmBuilder, IndexAndMove, MergingMode, split_algorithm};
pub use cacher::{CacheListener, CacherError, ScrambleCacher};
pub use image_info::PuzzleImageInfo;
pub use puzzle::{ColorScheme, Puzzle, PuzzleState, PuzzleStateAndGenerator};
pub(crate) use puzzle::{generate_random_turns, order_by_state_hash};
pub use registry::{PuzzleRegistry, Scrambler};
pub(crate) use solve::solve_in_bfs;
