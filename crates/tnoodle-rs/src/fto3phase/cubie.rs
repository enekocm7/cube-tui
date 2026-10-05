//! The piece model of the Face Turning Octahedron (`levigibson.fto3phase.FtoCubie`).

use std::sync::LazyLock;

use super::coord;
use super::util::{is_parity, n_cr, pack_perm, pack_subset, pow, unpack_perm, unpack_subset};
use crate::java::{RandomSource, array_hash};

/// Move numbers: clockwise turns are even, their inverses odd.
pub mod moves {
    /// `R`
    pub const R: usize = 0;
    /// `R'`
    pub const RP: usize = 1;
    /// `L`
    pub const L: usize = 2;
    /// `L'`
    pub const LP: usize = 3;
    /// `B`
    pub const B: usize = 4;
    /// `B'`
    pub const BP: usize = 5;
    /// `U`
    pub const U: usize = 6;
    /// `U'`
    pub const UP: usize = 7;
    /// `D`
    pub const D: usize = 8;
    /// `D'`
    pub const DP: usize = 9;
    /// `F`
    pub const F: usize = 10;
    /// `F'`
    pub const FP: usize = 11;
    /// `BR`
    pub const BR: usize = 12;
    /// `BR'`
    pub const BRP: usize = 13;
    /// `BL`
    pub const BL: usize = 14;
    /// `BL'`
    pub const BLP: usize = 15;
}
use moves::{B, BL, BR, D, F, L, R, U};

// Triangle colours of the R/L/B/D orbit (the U/F/BR/BL orbit uses 0..4 in that order).
const TOR: i32 = 0;
const TOL: i32 = 1;
const TOB: i32 = 2;
const TOD: i32 = 3;

// Triangle positions of the U/F/BR/BL orbit.
const TIUBL: usize = 0;
const TIUBR: usize = 1;
const TIUF: usize = 2;
const TIFU: usize = 3;
const TIFBR: usize = 4;
const TIFBL: usize = 5;
const TIBRU: usize = 6;
const TIBRBL: usize = 7;
const TIBRF: usize = 8;
const TIBLU: usize = 9;
const TIBLF: usize = 10;
const TIBLBR: usize = 11;

// Triangle positions of the R/L/B/D orbit.
const TIRL: usize = 0;
const TIRB: usize = 1;
const TIRD: usize = 2;
const TILB: usize = 3;
const TILR: usize = 4;
const TILD: usize = 5;
const TIBR: usize = 6;
const TIBL: usize = 7;
const TIBD: usize = 8;
const TIDR: usize = 9;
const TIDL: usize = 10;
const TIDB: usize = 11;

// Edges.
const EUB: usize = 0;
const EUR: usize = 1;
const EUL: usize = 2;
const EFL: usize = 3;
const EFR: usize = 4;
const ERBR: usize = 5;
const EBRB: usize = 6;
const EBLB: usize = 7;
const ELBL: usize = 8;
const EDF: usize = 9;
const EDBR: usize = 10;
const EDBL: usize = 11;

// Corners.
const CUF: usize = 0;
const CUBR: usize = 1;
const CUBL: usize = 2;
const CDL: usize = 3;
const CDR: usize = 4;
const CDB: usize = 5;

const G2_EDGE_COLORS: [i32; 9] = [TOB, TOR, TOL, TOL, TOR, TOR, TOB, TOB, TOL];
const G2_EDGE_NORM: [i32; 9] = [0, 0, 0, 1, 1, 2, 1, 2, 2];
const G2_EDGE_NORM_INV: [[i32; 3]; 3] = [
    [EUR as i32, EFR as i32, ERBR as i32],
    [EUL as i32, EFL as i32, ELBL as i32],
    [EUB as i32, EBRB as i32, EBLB as i32],
];
const G3_EDGES: [[usize; 3]; 4] = [
    [EUB, EBRB, EBLB],
    [EUR, EFR, ERBR],
    [EUL, ELBL, EFL],
    [EDF, EDBR, EDBL],
];
const CORNER_TRIPLE_COLOR_LOOKUP: [[i32; 6]; 4] = [
    [0, 0, 0, -1, -1, -1],
    [1, -1, -1, 0, 1, -1],
    [-1, 1, -1, -1, 0, 1],
    [-1, -1, 1, 1, -1, 0],
];

const G2_TRIPLE_CORNER_SIZE: i32 = n_cr(6, 3) * pow(2, 3);
const G2_TRIPLE_TRIANGLES_SIZE: i32 = n_cr(12, 3);
const ALL_TRIANGLES_INDEX_COEFFICIENTS: [i32; 3] = [n_cr(9, 3) * n_cr(6, 3), n_cr(6, 3), 1];

/// The effect of one move as source indices (`MoveEffect`).
#[derive(Debug, Clone, Copy, Default)]
struct MoveEffect {
    co: [i32; 6],
    cp: [usize; 6],
    tp_u: [usize; 12],
    tp_r: [usize; 12],
    ep: [usize; 12],
}

static MOVE_EFFECTS: LazyLock<[MoveEffect; 16]> = LazyLock::new(|| {
    let mut me = [MoveEffect::default(); 16];
    me[R] = MoveEffect {
        cp: [CDR, CUF, CUBL, CDL, CUBR, CDB],
        co: [1, 1, 0, 0, 0, 0],
        ep: [
            EUB, EFR, EUL, EFL, ERBR, EUR, EBRB, EBLB, ELBL, EDF, EDBR, EDBL,
        ],
        tp_u: [
            TIUBL, TIFU, TIFBR, TIBRF, TIBRU, TIFBL, TIUF, TIBRBL, TIUBR, TIBLU, TIBLF, TIBLBR,
        ],
        tp_r: [
            TIRD, TIRL, TIRB, TILB, TILR, TILD, TIBR, TIBL, TIBD, TIDR, TIDL, TIDB,
        ],
    };
    me[L] = MoveEffect {
        cp: [CUBL, CUBR, CDL, CUF, CDR, CDB],
        co: [1, 0, 1, 0, 0, 0],
        ep: [
            EUB, EUR, ELBL, EUL, EFR, ERBR, EBRB, EBLB, EFL, EDF, EDBR, EDBL,
        ],
        tp_u: [
            TIBLF, TIUBR, TIBLU, TIUBL, TIFBR, TIUF, TIBRU, TIBRBL, TIBRF, TIFBL, TIFU, TIBLBR,
        ],
        tp_r: [
            TIRL, TIRB, TIRD, TILD, TILB, TILR, TIBR, TIBL, TIBD, TIDR, TIDL, TIDB,
        ],
    };
    me[B] = MoveEffect {
        cp: [CUF, CDB, CUBR, CDL, CDR, CUBL],
        co: [0, 1, 1, 0, 0, 0],
        ep: [
            EBRB, EUR, EUL, EFL, EFR, ERBR, EBLB, EUB, ELBL, EDF, EDBR, EDBL,
        ],
        tp_u: [
            TIBRU, TIBRBL, TIUF, TIFU, TIFBR, TIFBL, TIBLBR, TIBLU, TIBRF, TIUBR, TIBLF, TIUBL,
        ],
        tp_r: [
            TIRL, TIRB, TIRD, TILB, TILR, TILD, TIBD, TIBR, TIBL, TIDR, TIDL, TIDB,
        ],
    };
    me[D] = MoveEffect {
        cp: [CUF, CUBR, CUBL, CDB, CDL, CDR],
        co: [0, 0, 0, 0, 0, 0],
        ep: [
            EUB, EUR, EUL, EFL, EFR, ERBR, EBRB, EBLB, ELBL, EDBL, EDF, EDBR,
        ],
        tp_u: [
            TIUBL, TIUBR, TIUF, TIFU, TIBLF, TIBLBR, TIBRU, TIFBR, TIFBL, TIBLU, TIBRBL, TIBRF,
        ],
        tp_r: [
            TIRL, TIRB, TIRD, TILB, TILR, TILD, TIBR, TIBL, TIBD, TIDL, TIDB, TIDR,
        ],
    };
    me[U] = MoveEffect {
        cp: [CUBR, CUBL, CUF, CDL, CDR, CDB],
        co: [0, 0, 0, 0, 0, 0],
        ep: [
            EUL, EUB, EUR, EFL, EFR, ERBR, EBRB, EBLB, ELBL, EDF, EDBR, EDBL,
        ],
        tp_u: [
            TIUF, TIUBL, TIUBR, TIFU, TIFBR, TIFBL, TIBRU, TIBRBL, TIBRF, TIBLU, TIBLF, TIBLBR,
        ],
        tp_r: [
            TIBR, TIBL, TIRD, TIRL, TIRB, TILD, TILB, TILR, TIBD, TIDR, TIDL, TIDB,
        ],
    };
    me[F] = MoveEffect {
        cp: [CDL, CUBR, CUBL, CDR, CUF, CDB],
        co: [1, 0, 0, 1, 0, 0],
        ep: [
            EUB, EUR, EUL, EDF, EFL, ERBR, EBRB, EBLB, ELBL, EFR, EDBR, EDBL,
        ],
        tp_u: [
            TIUBL, TIUBR, TIUF, TIFBL, TIFU, TIFBR, TIBRU, TIBRBL, TIBRF, TIBLU, TIBLF, TIBLBR,
        ],
        tp_r: [
            TILD, TIRB, TILR, TILB, TIDL, TIDR, TIBR, TIBL, TIBD, TIRL, TIRD, TIDB,
        ],
    };
    me[BR] = MoveEffect {
        cp: [CUF, CDR, CUBL, CDL, CDB, CUBR],
        co: [0, 1, 0, 0, 1, 0],
        ep: [
            EUB, EUR, EUL, EFL, EFR, EDBR, ERBR, EBLB, ELBL, EDF, EBRB, EDBL,
        ],
        tp_u: [
            TIUBL, TIUBR, TIUF, TIFU, TIFBR, TIFBL, TIBRF, TIBRU, TIBRBL, TIBLU, TIBLF, TIBLBR,
        ],
        tp_r: [
            TIRL, TIDR, TIDB, TILB, TILR, TILD, TIRD, TIBL, TIRB, TIBD, TIDL, TIBR,
        ],
    };
    me[BL] = MoveEffect {
        cp: [CUF, CUBR, CDB, CUBL, CDR, CDL],
        co: [0, 0, 1, 0, 0, 1],
        ep: [
            EUB, EUR, EUL, EFL, EFR, ERBR, EBRB, EDBL, EBLB, EDF, EDBR, ELBL,
        ],
        tp_u: [
            TIUBL, TIUBR, TIUF, TIFU, TIFBR, TIFBL, TIBRU, TIBRBL, TIBRF, TIBLBR, TIBLU, TIBLF,
        ],
        tp_r: [
            TIRL, TIRB, TIRD, TIBD, TILR, TIBL, TIBR, TIDB, TIDL, TIDR, TILB, TILD,
        ],
    };
    for mv in (1..16).step_by(2) {
        let normal = me[mv - 1];
        let mut inverse = MoveEffect::default();
        for i in 0..6 {
            inverse.cp[normal.cp[i]] = i;
        }
        for i in 0..6 {
            inverse.co[i] = normal.co[inverse.cp[i]];
        }
        for i in 0..12 {
            inverse.ep[normal.ep[i]] = i;
            inverse.tp_u[normal.tp_u[i]] = i;
            inverse.tp_r[normal.tp_r[i]] = i;
        }
        me[mv] = inverse;
    }
    me
});

/// The pieces of a Face Turning Octahedron: 6 corners (permutation and orientation),
/// 12 edges and two orbits of 12 center triangles.
///
/// Like the Java class, packing functions may temporarily leave unknown pieces as `-1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FtoCubie {
    corner_perm: [i32; 6],
    corner_ori: [i32; 6],
    edges: [i32; 12],
    triangles_ufbrbl: [i32; 12],
    triangles_rlbd: [i32; 12],
}

impl Default for FtoCubie {
    fn default() -> Self {
        let mut c = Self {
            corner_perm: [0, 1, 2, 3, 4, 5],
            corner_ori: [0; 6],
            edges: [0; 12],
            triangles_ufbrbl: [0; 12],
            triangles_rlbd: [0; 12],
        };
        for i in 0..12 {
            c.edges[i] = i as i32;
            c.triangles_ufbrbl[i] = i as i32 / 3;
            c.triangles_rlbd[i] = i as i32 / 3;
        }
        c
    }
}

impl FtoCubie {
    /// A solved FTO.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies a move (see [`moves`]).
    pub fn turn(&mut self, mv: usize) {
        *self = self.turned(mv);
    }

    /// The result of applying a move (`turnInto`).
    #[must_use]
    pub fn turned(&self, mv: usize) -> Self {
        let cycles = &MOVE_EFFECTS[mv];
        let mut out = *self;
        for i in 0..6 {
            out.corner_perm[i] = self.corner_perm[cycles.cp[i]];
            out.corner_ori[i] = self.corner_ori[cycles.cp[i]] ^ cycles.co[i];
        }
        for i in 0..12 {
            out.edges[i] = self.edges[cycles.ep[i]];
            out.triangles_ufbrbl[i] = self.triangles_ufbrbl[cycles.tp_u[i]];
            out.triangles_rlbd[i] = self.triangles_rlbd[cycles.tp_r[i]];
        }
        out
    }

    /// A uniformly random FTO (`randomCube`).
    pub fn random(r: &mut dyn RandomSource) -> Self {
        let mut fto = Self::new();
        fto.set_all_edges(r.next_int_bounded(coord::ALL_EDGES_SIZE));
        fto.set_all_corner_orientation(r.next_int_bounded(coord::ALL_CORNER_ORIENTATION_SIZE));
        fto.set_all_corner_permutation(r.next_int_bounded(coord::ALL_CORNER_PERMUTATION_SIZE));
        fto.set_all_triangles(r.next_int_bounded(coord::ALL_TRIANGLE_SIZE), 0);
        fto.set_all_triangles(r.next_int_bounded(coord::ALL_TRIANGLE_SIZE), 1);
        fto
    }

    /// Applies `moves` to a copy of `self` (`applyMovesInto`).
    #[must_use]
    pub fn with_moves(&self, moves: &[usize]) -> Self {
        let mut out = *self;
        for &m in moves {
            out.turn(m);
        }
        out
    }

    /// The corner permutation.
    pub fn corner_perm(&self) -> [i32; 6] {
        self.corner_perm
    }

    /// The corner orientations.
    pub fn corner_ori(&self) -> [i32; 6] {
        self.corner_ori
    }

    /// The edge permutation.
    pub fn edges(&self) -> [i32; 12] {
        self.edges
    }

    /// The colours of the U/F/BR/BL triangle orbit.
    pub fn triangles_ufbrbl(&self) -> [i32; 12] {
        self.triangles_ufbrbl
    }

    /// The colours of the R/L/B/D triangle orbit.
    pub fn triangles_rlbd(&self) -> [i32; 12] {
        self.triangles_rlbd
    }

    /// Java's `hashCode()`.
    pub fn java_hash(&self) -> i32 {
        array_hash([
            array_hash(self.edges),
            array_hash(self.triangles_ufbrbl),
            array_hash(self.triangles_rlbd),
            array_hash(self.corner_ori),
            array_hash(self.corner_perm),
        ])
    }

    // ----- Whole puzzle coordinates -----

    /// Index of the edge permutation (always even).
    pub fn pack_all_edges(&self) -> i32 {
        pack_perm(&self.edges, true, 12)
    }

    /// Sets the edge permutation from its index.
    pub fn set_all_edges(&mut self, idx: i32) {
        assert!(
            (0..coord::ALL_EDGES_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        unpack_perm(&mut self.edges, idx, 12, true);
    }

    /// Index of the corner permutation (always even).
    pub fn pack_all_corner_permutation(&self) -> i32 {
        pack_perm(&self.corner_perm, true, 6)
    }

    /// Sets the corner permutation from its index.
    pub fn set_all_corner_permutation(&mut self, idx: i32) {
        assert!(
            (0..coord::ALL_CORNER_PERMUTATION_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        unpack_perm(&mut self.corner_perm, idx, 6, true);
    }

    /// Index of the corner orientations.
    pub fn pack_all_corner_orientation(&self) -> i32 {
        (0..5).fold(0, |index, i| index | self.corner_ori[i] << i)
    }

    /// Sets the corner orientations from their index.
    pub fn set_all_corner_orientation(&mut self, idx: i32) {
        assert!(
            (0..coord::ALL_CORNER_ORIENTATION_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        for i in 0..5 {
            self.corner_ori[i] = (idx >> i) & 1;
        }
        self.corner_ori[5] = (idx.count_ones() % 2) as i32;
    }

    fn orbit(&self, orbit: usize) -> &[i32; 12] {
        assert!(orbit == 0 || orbit == 1, "Orbit must be 0 or 1");
        if orbit == 1 {
            &self.triangles_rlbd
        } else {
            &self.triangles_ufbrbl
        }
    }

    /// Index of the triangle colours of an orbit (0 = U/F/BR/BL, 1 = R/L/B/D).
    pub fn pack_all_triangles(&self, orbit: usize) -> i32 {
        let triangles = self.orbit(orbit);
        let mut used = [false; 12];
        let mut loc = [[0; 3]; 3];
        for xo in 0..3 {
            let mut found = 0;
            let mut passed = 0;
            for i in 0..12 {
                if triangles[i] == xo {
                    assert!(found < 3, "Expected found=3. Instead, found=more");
                    loc[xo as usize][found] = passed;
                    found += 1;
                    used[i] = true;
                    passed += 1;
                } else if !used[i] {
                    passed += 1;
                }
            }
            assert!(found == 3, "Expected found=3. Instead, found={found}");
        }
        pack_subset(&loc[0]) * ALL_TRIANGLES_INDEX_COEFFICIENTS[0]
            + pack_subset(&loc[1]) * ALL_TRIANGLES_INDEX_COEFFICIENTS[1]
            + pack_subset(&loc[2]) * ALL_TRIANGLES_INDEX_COEFFICIENTS[2]
    }

    /// Sets the triangle colours of an orbit from their index.
    pub fn set_all_triangles(&mut self, idx: i32, orbit: usize) {
        assert!(
            (0..coord::ALL_TRIANGLE_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        assert!(orbit == 0 || orbit == 1, "Orbit must be 0 or 1");
        let mut loc = [[0; 3]; 3];
        let mut remaining = idx;
        for color in 0..3 {
            let coefficient = ALL_TRIANGLES_INDEX_COEFFICIENTS[color];
            let digit = remaining / coefficient;
            unpack_subset(&mut loc[color], digit);
            remaining -= coefficient * digit;
        }
        let triangles = if orbit == 1 {
            &mut self.triangles_rlbd
        } else {
            &mut self.triangles_ufbrbl
        };
        triangles.fill(3);
        for color in 0..3 {
            let mut nz = 0;
            let mut li = 0;
            for t in triangles.iter_mut() {
                if *t < color as i32 {
                    continue;
                }
                if li < 3 && nz == loc[color][li] {
                    *t = color as i32;
                    li += 1;
                }
                nz += 1;
            }
        }
    }

    // ----- Phase 1 -----

    /// Phase 1 edge coordinate: where the D layer edges are, and their parity.
    pub fn g1_pack_edges(&self) -> i32 {
        let mut loc = [0; 3];
        let mut perm = [0; 3];
        let mut count = 0;
        for i in 0..12 {
            if self.edges[i] >= EDF as i32 {
                assert!(
                    count < 3,
                    "Expected 3 D-face edges. This is not a possible FTO state."
                );
                loc[count] = i as i32;
                perm[count] = self.edges[i] - EDF as i32;
                count += 1;
            }
        }
        assert!(
            count == 3,
            "Expected 3 D-face edges. This is not a possible FTO state."
        );
        let parity = i32::from(is_parity(&perm));
        pack_subset(&loc) * 2 + parity
    }

    /// Sets the phase 1 edge coordinate; the other edges become `-1`.
    pub fn g1_set_edges(&mut self, idx: i32) {
        assert!(
            (0..coord::G1_EDGES_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        let loc_idx = idx / 2;
        let parity = idx % 2;
        let mut loc = [0; 3];
        let mut perm = [EDF as i32, EDBR as i32, EDBL as i32];
        if parity == 1 {
            perm.swap(1, 2);
        }
        unpack_subset(&mut loc, loc_idx);
        self.edges.fill(-1);
        for i in 0..3 {
            self.edges[loc[i] as usize] = perm[i];
        }
    }

    /// Phase 1 triangle coordinate: where the D triangles of the R/L/B/D orbit are.
    pub fn g1_pack_triangles(&self) -> i32 {
        let mut idx = [0; 3];
        let mut count = 0;
        for i in 0..12 {
            if self.triangles_rlbd[i] == TOD {
                assert!(count < 3, "Less than 3 d-layer triangles");
                idx[count] = i as i32;
                count += 1;
            }
        }
        assert!(count == 3, "Less than 3 d-layer triangles");
        pack_subset(&idx)
    }

    /// Sets the phase 1 triangle coordinate; the other triangles become `-1`.
    pub fn g1_set_triangles(&mut self, idx: i32) {
        assert!(
            (0..coord::G1_TRIANGLES_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        let mut loc = [0; 3];
        unpack_subset(&mut loc, idx);
        self.triangles_rlbd.fill(-1);
        for l in loc {
            self.triangles_rlbd[l as usize] = TOD;
        }
    }

    // ----- Phase 2 -----

    /// Phase 2 edge coordinate.
    pub fn g2_pack_edges(&self) -> i32 {
        assert!(
            self.edges[..9].iter().all(|&e| e <= 8),
            "Edges not in phase 1"
        );
        let mut used = [false; 9];
        let mut loc = [[0; 3]; 2];
        let mut perm = [[0; 3]; 2];
        for xo in 0..2 {
            let mut found = 0;
            let mut passed = 0;
            for i in 0..9 {
                let e = self.edges[i] as usize;
                if G2_EDGE_COLORS[e] == xo {
                    assert!(found < 3, "Expected found=3. Instead, found=more");
                    perm[xo as usize][found] = G2_EDGE_NORM[e];
                    loc[xo as usize][found] = passed;
                    found += 1;
                    used[i] = true;
                    passed += 1;
                } else if !used[i] {
                    passed += 1;
                }
            }
            assert!(found == 3, "Expected found=3. Instead, found={found}");
        }
        let parity = [
            i32::from(is_parity(&perm[0])),
            i32::from(is_parity(&perm[1])),
        ];
        let subset_index = pack_subset(&loc[1]) + pack_subset(&loc[0]) * n_cr(6, 3);
        let parity_index = parity[1] * 2 + parity[0];
        subset_index * 4 + parity_index
    }

    /// Sets the phase 2 edge coordinate.
    pub fn g2_set_edges(&mut self, idx: i32) {
        assert!(
            (0..coord::G2_EDGES_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        let subset_index = idx / 4;
        let parity_index = idx % 4;
        let mut loc = [[0; 3]; 2];
        let mut perm = [[0; 3]; 2];
        let parity = [parity_index & 1, (parity_index >> 1) & 1];
        unpack_subset(&mut loc[0], subset_index / n_cr(6, 3));
        unpack_subset(&mut loc[1], subset_index % n_cr(6, 3));
        for i in 0..2 {
            perm[i] = G2_EDGE_NORM_INV[i];
            if parity[i] == 1 {
                perm[i].swap(1, 2);
            }
        }
        self.edges.fill(-1);
        self.edges[EDF] = EDF as i32;
        self.edges[EDBL] = EDBL as i32;
        self.edges[EDBR] = EDBR as i32;
        for i in 0..3 {
            self.edges[loc[0][i] as usize] = perm[0][i];
        }
        let mut nz = 0;
        let mut li = 0;
        for i in 0..9 {
            if self.edges[i] != -1 {
                continue;
            }
            if li < 3 && nz == loc[1][li] {
                self.edges[i] = perm[1][li];
                li += 1;
            }
            nz += 1;
        }
        let mut bloc = [0; 3];
        li = 0;
        for i in 0..9 {
            if self.edges[i] == -1 {
                bloc[li] = i;
                self.edges[i] = G2_EDGE_NORM_INV[2][li];
                li += 1;
            }
        }
        if is_parity(&self.edges) {
            self.edges.swap(bloc[0], bloc[1]);
        }
    }

    /// Phase 2 triangle coordinate (R/L/B/D orbit).
    pub fn g2_pack_triangles(&self) -> i32 {
        assert!(
            self.triangles_rlbd[9..].iter().all(|&t| t == TOD),
            "Tris must be in phase 1"
        );
        let mut used = [false; 9];
        let mut loc = [[0; 3]; 2];
        for xo in 0..2 {
            let mut found = 0;
            let mut passed = 0;
            for i in 0..9 {
                if self.triangles_rlbd[i] == xo {
                    assert!(found < 3, "Expected found=3. Instead, found=more");
                    loc[xo as usize][found] = passed;
                    found += 1;
                    used[i] = true;
                    passed += 1;
                } else if !used[i] {
                    passed += 1;
                }
            }
            assert!(found == 3, "Expected found=3. Instead, found={found}");
        }
        pack_subset(&loc[1]) + pack_subset(&loc[0]) * n_cr(6, 3)
    }

    /// Sets the phase 2 triangle coordinate.
    pub fn g2_set_triangles(&mut self, idx: i32) {
        assert!(
            (0..coord::G2_TRIANGLES_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        let mut loc0 = [0; 3];
        let mut loc1 = [0; 3];
        unpack_subset(&mut loc0, idx / n_cr(6, 3));
        unpack_subset(&mut loc1, idx % n_cr(6, 3));
        self.triangles_rlbd[..9].fill(2);
        for v in loc0 {
            self.triangles_rlbd[v as usize] = 0;
        }
        let mut nz = 0;
        let mut li = 0;
        for i in 0..9 {
            if self.triangles_rlbd[i] == 0 {
                continue;
            }
            if li < 3 && nz == loc1[li] {
                self.triangles_rlbd[i] = 1;
                li += 1;
            }
            nz += 1;
        }
        self.triangles_rlbd[9..].fill(TOD);
    }

    /// The triple (three triangles and three corner stickers) coordinate of `color`
    /// (0 = U, 1 = F, 2 = BR, 3 = BL).
    pub fn g2_pack_triples(&self, color: usize) -> i32 {
        assert!(color <= 3, "color must be U, F, BR, or BL");
        G2_TRIPLE_CORNER_SIZE * self.g2_pack_triple_tris(color) + self.g2_pack_triple_corners(color)
    }

    /// Sets the triple coordinate of `color`; other pieces become `-1`.
    pub fn g2_set_triples(&mut self, idx: i32, color: usize) {
        assert!(
            (0..coord::G2_TRIPLE_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        self.g2_set_triple_tris(idx / G2_TRIPLE_CORNER_SIZE, color);
        self.g2_set_triple_corners(idx % G2_TRIPLE_CORNER_SIZE, color);
    }

    fn g2_pack_triple_corners(&self, color: usize) -> i32 {
        let mut idx = [0; 3];
        let mut orientation = 0;
        let mut found = 0;
        for i in 0..6 {
            let corner = self.corner_perm[i];
            let ori = self.corner_ori[i];
            if corner == -1 {
                continue;
            }
            let parity = CORNER_TRIPLE_COLOR_LOOKUP[color][corner as usize];
            if parity != -1 {
                assert!(found < 3, "Expected 3 matching corners");
                orientation |= (parity ^ ori) << found;
                idx[found] = i as i32;
                found += 1;
            }
        }
        assert!(found == 3, "Expected 3 matching corners");
        pack_subset(&idx) * 8 + orientation
    }

    fn g2_pack_triple_tris(&self, color: usize) -> i32 {
        let mut idx = [0; 3];
        let mut found = 0;
        for i in 0..12 {
            if self.triangles_ufbrbl[i] == color as i32 {
                idx[found] = i as i32;
                found += 1;
            }
        }
        pack_subset(&idx)
    }

    fn g2_set_triple_tris(&mut self, idx: i32, color: usize) {
        assert!(
            (0..G2_TRIPLE_TRIANGLES_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        let mut loc = [0; 3];
        unpack_subset(&mut loc, idx);
        self.triangles_ufbrbl.fill(-1);
        for l in loc {
            self.triangles_ufbrbl[l as usize] = color as i32;
        }
    }

    fn g2_set_triple_corners(&mut self, idx: i32, color: usize) {
        assert!(
            (0..G2_TRIPLE_CORNER_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        let mut loc = [0; 3];
        unpack_subset(&mut loc, idx / 8);
        let orientation = idx % 8;
        let mut relevant = [0; 3];
        let mut found = 0;
        for i in 0..6 {
            if CORNER_TRIPLE_COLOR_LOOKUP[color][i] != -1 {
                relevant[found] = i;
                found += 1;
            }
        }
        self.corner_perm.fill(-1);
        self.corner_ori.fill(-1);
        for i in 0..3 {
            let perm = loc[i] as usize;
            let ori = (orientation >> i) & 1;
            self.corner_perm[perm] = relevant[i] as i32;
            self.corner_ori[perm] = ori ^ CORNER_TRIPLE_COLOR_LOOKUP[color][relevant[i]];
        }
    }

    // ----- Phase 3 -----

    /// Phase 3 corner coordinate (permutation and orientation).
    pub fn g3_pack_corners(&self) -> i32 {
        self.pack_all_corner_permutation() * coord::ALL_CORNER_ORIENTATION_SIZE
            + self.pack_all_corner_orientation()
    }

    /// Sets the phase 3 corner coordinate.
    pub fn g3_set_corners(&mut self, idx: i32) {
        assert!(
            (0..coord::ALL_CORNER_ORIENTATION_SIZE * coord::ALL_CORNER_PERMUTATION_SIZE)
                .contains(&idx),
            "Index {idx} out of range"
        );
        self.set_all_corner_permutation(idx / coord::ALL_CORNER_ORIENTATION_SIZE);
        self.set_all_corner_orientation(idx % coord::ALL_CORNER_ORIENTATION_SIZE);
    }

    /// Phase 3 edge coordinate: the rotation of each of the four edge triples.
    pub fn g3_pack_edges(&self) -> i32 {
        let mut loc = [-1; 4];
        for axis in 0..4 {
            for i in 0..3 {
                if G3_EDGES[axis][i] as i32 == self.edges[G3_EDGES[axis][0]] {
                    loc[axis] = i as i32;
                }
            }
        }
        assert!(loc.iter().all(|&l| l != -1), "Cound not find G3 edges");
        27 * loc[0] + 9 * loc[1] + 3 * loc[2] + loc[3]
    }

    /// Sets the phase 3 edge coordinate.
    pub fn g3_set_edges(&mut self, idx: i32) {
        assert!(
            (0..coord::G3_EDGE_SIZE).contains(&idx),
            "Index {idx} out of range"
        );
        for (i, e) in self.edges.iter_mut().enumerate() {
            *e = i as i32;
        }
        let mut remaining = idx;
        for axis in 0..4 {
            let coefficient = pow(3, 3 - axis as u32);
            let digit = remaining / coefficient;
            for _ in 0..3 {
                if self.edges[G3_EDGES[axis][0]] == G3_EDGES[axis][digit as usize] as i32 {
                    break;
                }
                for i in 0..2 {
                    self.edges.swap(G3_EDGES[axis][0], G3_EDGES[axis][i + 1]);
                }
            }
            remaining -= coefficient * digit;
        }
    }
}
