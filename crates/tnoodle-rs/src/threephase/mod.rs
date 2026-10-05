//! Chen Shuang's three phase 4x4x4 solver, used by TNoodle for 4x4x4 random-state
//! scrambles.
//!
//! The phases reduce the 4x4x4 to a 3x3x3 (centers, then edge pairing), which is then
//! solved with [`min2phase`](crate::min2phase).

mod centers;
mod cubes;
mod edge3;
mod full_cube;
mod moves;
mod search;
mod tables;

pub use edge3::init_status;
pub use full_cube::FullCube;
pub use search::Search;

/// A uniformly random 4x4x4 as 96 facelets (`Tools.randomCube`).
pub fn random_cube(r: &mut dyn crate::java::RandomSource) -> String {
    FullCube::random(r).to_facelet_string()
}
