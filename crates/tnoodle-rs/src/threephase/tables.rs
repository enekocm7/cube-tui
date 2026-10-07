//! The lazily built lookup tables of the 4x4x4 solver.

use std::sync::LazyLock;

use super::centers::{Center1Tables, Center2Tables, Center3Tables};
use super::edge3::Edge3Tables;
use crate::parallel;

/// Every table of the 4x4x4 solver. Building them takes about 0.4s of CPU time in release
/// builds, dominated by the edge pruning table, spread over all cores.
pub(crate) struct Tables {
    pub(crate) center1: Center1Tables,
    pub(crate) center2: Center2Tables,
    pub(crate) center3: Center3Tables,
    pub(crate) edge3: Edge3Tables,
}

pub(crate) static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    // The tables are independent; the edge pruning table, the slowest, is itself built in
    // parallel.
    let ((edge3, center1), ((), (center2, center3))) = parallel::join(
        || parallel::join(Edge3Tables::build, Center1Tables::build),
        || {
            parallel::join(crate::min2phase::Search::init, || {
                parallel::join(Center2Tables::build, Center3Tables::build)
            })
        },
    );
    Tables {
        center1,
        center2,
        center3,
        edge3,
    }
});
