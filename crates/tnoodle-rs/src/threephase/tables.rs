//! The lazily built lookup tables of the 4x4x4 solver.

use std::sync::LazyLock;

use super::centers::{Center1Tables, Center2Tables, Center3Tables};
use super::edge3::Edge3Tables;

/// Every table of the 4x4x4 solver. Building them takes a few seconds in release builds,
/// dominated by the edge pruning table.
pub(crate) struct Tables {
    pub(crate) center1: Center1Tables,
    pub(crate) center2: Center2Tables,
    pub(crate) center3: Center3Tables,
    pub(crate) edge3: Edge3Tables,
}

pub(crate) static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    crate::min2phase::Search::init();
    Tables {
        center1: Center1Tables::build(),
        center2: Center2Tables::build(),
        center3: Center3Tables::build(),
        edge3: Edge3Tables::build(),
    }
});
