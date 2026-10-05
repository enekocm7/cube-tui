//! Chen Shuang's two phase Square-1 solver, used by TNoodle for Square-1 random-state
//! scrambles and to check scramble distances.

mod full_cube;
mod search;
mod tables;

pub use full_cube::FullCube;
pub use search::{INVERSE_SOLUTION, Search};
