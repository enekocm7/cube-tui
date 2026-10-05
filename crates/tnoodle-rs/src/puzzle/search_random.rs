//! Where the Pyraminx and Skewb solvers get the randomness that orders their moves.

use crate::java::JavaRandom;

/// The randomness for one solve. TNoodle uses a fresh `new Random()` for every solve, so
/// equally short solutions are picked at random; seeding makes the choice reproducible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum SearchRandom {
    /// Fresh operating system entropy for every solve.
    #[default]
    Entropy,
    /// `new java.util.Random(seed)` for every solve.
    Seeded(i64),
}

impl SearchRandom {
    /// A new generator for one solve.
    pub(crate) fn source(self) -> JavaRandom {
        match self {
            Self::Entropy => JavaRandom::from_entropy(),
            Self::Seeded(seed) => JavaRandom::new(seed),
        }
    }
}
