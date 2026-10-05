//! A 4x4x4 with a lazily applied move buffer (`cs.threephase.FullCube`).

use super::centers::Center1;
use super::cubes::{CENTER_FACELET, CenterCube, CornerCube, EdgeCube};
use super::moves::{MOVE2STR, WD1, f4};
use super::tables::Tables;
use crate::java::RandomSource;

const FACES: &[u8; 6] = b"URFDLB";

#[allow(clippy::wildcard_imports, reason = "facelet names")]
const EDGE_FACELET: [[u8; 2]; 24] = {
    use f4::*;
    [
        [ud, f1],
        [u4, l1],
        [u2, b1],
        [ub, r1],
        [dd, be],
        [d4, le],
        [d2, fe],
        [db, re],
        [lb, f8],
        [l4, b7],
        [rb, b8],
        [r4, f7],
        [f2, ue],
        [l2, u8],
        [b2, u1],
        [r2, u7],
        [bd, de],
        [ld, d8],
        [fd, d1],
        [rd, d7],
        [f4, l7],
        [bb, l8],
        [b4, r7],
        [fb, r8],
    ]
};

#[allow(clippy::wildcard_imports, reason = "facelet names")]
const CORNER_FACELET: [[u8; 3]; 8] = {
    use f4::*;
    [
        [uf, r0, f3],
        [uc, f0, l3],
        [u0, l0, b3],
        [u3, b0, r3],
        [d3, ff, rc],
        [d0, lf, fc],
        [dc, bf, lc],
        [df, rf, bc],
    ]
};

const MOVE2ROT: [usize; 9] = [35, 1, 34, 2, 4, 6, 22, 5, 19];

/// A 4x4x4 cube state plus the bookkeeping of a partial solution.
#[derive(Debug, Clone)]
pub struct FullCube {
    edge: EdgeCube,
    center: CenterCube,
    corner: CornerCube,

    pub(crate) value: i32,
    pub(crate) add1: bool,
    pub(crate) length1: i32,
    pub(crate) length2: i32,

    move_buffer: [u8; 60],
    move_length: usize,
    edge_avail: usize,
    center_avail: usize,
    corner_avail: usize,

    pub(crate) sym: usize,
}

impl Default for FullCube {
    fn default() -> Self {
        Self {
            edge: EdgeCube::default(),
            center: CenterCube::default(),
            corner: CornerCube::default(),
            value: 0,
            add1: false,
            length1: 0,
            length2: 0,
            move_buffer: [0; 60],
            move_length: 0,
            edge_avail: 0,
            center_avail: 0,
            corner_avail: 0,
            sym: 0,
        }
    }
}

impl FullCube {
    /// A solved cube.
    pub fn new() -> Self {
        Self::default()
    }

    /// A uniformly random cube (edges, then centers, then corners are drawn from `r`).
    pub fn random(r: &mut dyn RandomSource) -> Self {
        let edge = EdgeCube::random(r);
        let center = CenterCube::random(r);
        let corner = CornerCube::random(r);
        Self {
            edge,
            center,
            corner,
            ..Self::default()
        }
    }

    /// The cube obtained by applying `moves` (0..36) to a solved cube.
    pub fn from_moves(moves: &[usize]) -> Self {
        let mut c = Self::default();
        for &m in moves {
            c.do_move(m);
        }
        c
    }

    /// Parses 96 facelet colours (0..6 in `URFDLB` order).
    pub(crate) fn from_facelets(f: &[u8; 96]) -> Self {
        let mut c = Self::default();
        for i in 0..24 {
            c.center.ct[i] = f[CENTER_FACELET[i] as usize];
        }
        for i in 0..24 {
            for j in 0..24 {
                if f[EDGE_FACELET[i][0] as usize] == EDGE_FACELET[j][0] / 16
                    && f[EDGE_FACELET[i][1] as usize] == EDGE_FACELET[j][1] / 16
                {
                    c.edge.ep[i] = j as u8;
                }
            }
        }
        for i in 0..8 {
            let mut ori = 0;
            while ori < 3 {
                let col = f[CORNER_FACELET[i][ori] as usize];
                if col == f4::u0 / 16 || col == f4::d0 / 16 {
                    break;
                }
                ori += 1;
            }
            let col1 = f[CORNER_FACELET[i][(ori + 1) % 3] as usize];
            let col2 = f[CORNER_FACELET[i][(ori + 2) % 3] as usize];
            for j in 0..8 {
                if col1 == CORNER_FACELET[j][1] / 16 && col2 == CORNER_FACELET[j][2] / 16 {
                    c.corner.cp[i] = j as u8;
                    c.corner.co[i] = (ori % 3) as u8;
                    break;
                }
            }
        }
        c
    }

    /// Writes the 96 facelet colours of the cube.
    pub(crate) fn to_facelets(&self, f: &mut [u8; 96]) {
        for i in 0..24 {
            f[CENTER_FACELET[i] as usize] = self.center.ct[i];
        }
        for i in 0..24 {
            let e = self.edge.ep[i] as usize;
            f[EDGE_FACELET[i][0] as usize] = EDGE_FACELET[e][0] / 16;
            f[EDGE_FACELET[i][1] as usize] = EDGE_FACELET[e][1] / 16;
        }
        for c in 0..8 {
            let j = self.corner.cp[c] as usize;
            let ori = self.corner.co[c] as usize;
            for n in 0..3 {
                f[CORNER_FACELET[c][(n + ori) % 3] as usize] = CORNER_FACELET[j][n] / 16;
            }
        }
    }

    /// The 96 character facelet string (`URFDLB` letters).
    pub fn to_facelet_string(&mut self) -> String {
        self.edge();
        self.center();
        self.corner();
        let mut f = [0_u8; 96];
        self.to_facelets(&mut f);
        f.iter().map(|&c| char::from(FACES[c as usize])).collect()
    }

    pub(crate) fn copy_from(&mut self, c: &Self) {
        self.clone_from(c);
    }

    pub(crate) fn check_edge(&mut self) -> bool {
        self.edge().check_edge()
    }

    /// The move string of the solution recorded in the move buffer.
    pub(crate) fn move_string(&mut self, t: &Tables, inverse: bool, rotation: bool) -> String {
        let c1 = &t.center1;
        let skip = if self.add1 { 2 } else { 0 };
        let mut fixed_moves = Vec::with_capacity(self.move_length - skip);
        for i in 0..self.length1 as usize {
            fixed_moves.push(usize::from(self.move_buffer[i]));
        }
        let mut sym = self.sym;
        for i in self.length1 as usize + skip..self.move_length {
            let m = usize::from(c1.symmove[sym][usize::from(self.move_buffer[i])]);
            if m >= WD1 {
                fixed_moves.push(m - 9);
                let rot = MOVE2ROT[m - WD1];
                sym = usize::from(c1.symmult[sym][rot]);
            } else {
                fixed_moves.push(m);
            }
        }
        let finish_sym = usize::from(
            c1.symmult[usize::from(c1.syminv[sym])][Center1::solved_sym(self.center())],
        );

        let mut sb = String::new();
        sym = finish_sym;
        if inverse {
            for &m in fixed_moves.iter().rev() {
                let m = m / 3 * 3 + (2 - m % 3);
                let s = usize::from(c1.symmove[sym][m]);
                if s >= WD1 {
                    sb.push_str(MOVE2STR[s - 9]);
                    sb.push(' ');
                    let rot = MOVE2ROT[s - WD1];
                    sym = usize::from(c1.symmult[sym][rot]);
                } else {
                    sb.push_str(MOVE2STR[s]);
                    sb.push(' ');
                }
            }
            if rotation {
                // Cube rotation after the solution; omitted for WCA scrambles.
                sb.push_str(ROT2STR[usize::from(c1.syminv[sym])]);
                sb.push(' ');
            }
        } else {
            for &m in &fixed_moves {
                sb.push_str(MOVE2STR[m]);
                sb.push(' ');
            }
            if rotation {
                sb.push_str(ROT2STR[finish_sym]);
            }
        }
        sb
    }

    /// The facelets of the reduced 3x3x3.
    pub(crate) fn reduced_333_facelets(&mut self) -> String {
        let mut ret = [0_u8; 54];
        self.edge().fill_333_facelet(&mut ret);
        self.center().fill_333_facelet(&mut ret);
        self.corner().fill_333_facelet(&mut ret);
        String::from_utf8(ret.to_vec()).expect("facelets are ASCII")
    }

    /// Records a move in the buffer; it is applied to the pieces lazily.
    pub(crate) fn push_move(&mut self, m: usize) {
        self.move_buffer[self.move_length] = m as u8;
        self.move_length += 1;
    }

    fn do_move(&mut self, m: usize) {
        self.edge().do_move(m);
        self.center().do_move(m);
        self.corner().do_move(m % 18);
    }

    pub(crate) fn edge(&mut self) -> &mut EdgeCube {
        while self.edge_avail < self.move_length {
            self.edge
                .do_move(usize::from(self.move_buffer[self.edge_avail]));
            self.edge_avail += 1;
        }
        &mut self.edge
    }

    pub(crate) fn center(&mut self) -> &mut CenterCube {
        while self.center_avail < self.move_length {
            self.center
                .do_move(usize::from(self.move_buffer[self.center_avail]));
            self.center_avail += 1;
        }
        &mut self.center
    }

    pub(crate) fn corner(&mut self) -> &mut CornerCube {
        while self.corner_avail < self.move_length {
            self.corner
                .do_move(usize::from(self.move_buffer[self.corner_avail]) % 18);
            self.corner_avail += 1;
        }
        &mut self.corner
    }
}

/// Rotation names indexed by symmetry (`Center1.rot2str`).
const ROT2STR: [&str; 48] = [
    "", "y2", "x", "x y2", "x2", "z2", "x'", "x' y2", "", "", "", "", "", "", "", "", "y z",
    "y' z'", "y2 z", "z'", "y' z", "y z'", "z", "z y2", "", "", "", "", "", "", "", "", "y' x'",
    "y x", "y'", "y", "y' x", "y x'", "y z2", "y' z2", "", "", "", "", "", "", "", "",
];
