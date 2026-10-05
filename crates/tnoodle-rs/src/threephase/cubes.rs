//! The piece level models of the 4x4x4: centers, corners and (wing) edges.

use std::sync::LazyLock;

use super::moves::{f3, set8_perm, swap4};
use crate::java::RandomSource;

const FACES: &[u8; 6] = b"URFDLB";

/// The 24 center stickers, numbered as in the Java source:
///
/// ```text
///             0  1
///             3  2
/// 20 21    8  9   16 17   12 13
/// 23 22   11 10   19 18   15 14
///             4  5
///             7  6
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CenterCube {
    pub(crate) ct: [u8; 24],
}

/// The facelet of each center sticker (`FullCube.centerFacelet`).
#[allow(clippy::wildcard_imports, reason = "facelet names")]
pub(crate) const CENTER_FACELET: [u8; 24] = {
    use super::moves::f4::*;
    [
        u5, u6, ua, u9, d5, d6, da, d9, f5, f6, fa, f9, b5, b6, ba, b9, r5, r6, ra, r9, l5, l6, la,
        l9,
    ]
};

const CENTER_333_MAP: [usize; 6] = [0, 4, 2, 1, 5, 3];

impl Default for CenterCube {
    fn default() -> Self {
        let mut ct = [0; 24];
        for (c, f) in ct.iter_mut().zip(CENTER_FACELET) {
            *c = f / 16;
        }
        Self { ct }
    }
}

impl CenterCube {
    pub(crate) fn random(r: &mut dyn RandomSource) -> Self {
        let mut c = Self::default();
        for i in 0..23 {
            let t = i + r.next_int_bounded(24 - i as i32) as usize;
            if c.ct[t] != c.ct[i] {
                c.ct.swap(i, t);
            }
        }
        c
    }

    pub(crate) fn fill_333_facelet(&self, facelet: &mut [u8; 54]) {
        let first_idx = 4;
        let inc = 9;
        for i in 0..6 {
            let idx = CENTER_333_MAP[i] << 2;
            let ct = &self.ct;
            assert!(
                ct[idx] == ct[idx + 1] && ct[idx + 1] == ct[idx + 2] && ct[idx + 2] == ct[idx + 3],
                "Unsolved Center"
            );
            facelet[first_idx + i * inc] = FACES[ct[idx] as usize];
        }
    }

    pub(crate) fn do_move(&mut self, m: usize) {
        center_move(&mut self.ct, m);
    }
}

/// Applies move `m` (0..36) to an array laid out like the center stickers.
pub(crate) fn center_move<T: Copy>(ct: &mut [T], m: usize) {
    let key = m % 3;
    match m / 3 {
        0 => swap4(ct, 0, 1, 2, 3, key),
        1 => swap4(ct, 16, 17, 18, 19, key),
        2 => swap4(ct, 8, 9, 10, 11, key),
        3 => swap4(ct, 4, 5, 6, 7, key),
        4 => swap4(ct, 20, 21, 22, 23, key),
        5 => swap4(ct, 12, 13, 14, 15, key),
        6 => {
            swap4(ct, 0, 1, 2, 3, key);
            swap4(ct, 8, 20, 12, 16, key);
            swap4(ct, 9, 21, 13, 17, key);
        }
        7 => {
            swap4(ct, 16, 17, 18, 19, key);
            swap4(ct, 1, 15, 5, 9, key);
            swap4(ct, 2, 12, 6, 10, key);
        }
        8 => {
            swap4(ct, 8, 9, 10, 11, key);
            swap4(ct, 2, 19, 4, 21, key);
            swap4(ct, 3, 16, 5, 22, key);
        }
        9 => {
            swap4(ct, 4, 5, 6, 7, key);
            swap4(ct, 10, 18, 14, 22, key);
            swap4(ct, 11, 19, 15, 23, key);
        }
        10 => {
            swap4(ct, 20, 21, 22, 23, key);
            swap4(ct, 0, 8, 4, 14, key);
            swap4(ct, 3, 11, 7, 13, key);
        }
        11 => {
            swap4(ct, 12, 13, 14, 15, key);
            swap4(ct, 1, 20, 7, 18, key);
            swap4(ct, 0, 23, 6, 17, key);
        }
        _ => {}
    }
}

/// The corners: permutation `cp` and orientation `co`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CornerCube {
    pub(crate) cp: [u8; 8],
    pub(crate) co: [u8; 8],
}

impl Default for CornerCube {
    fn default() -> Self {
        Self {
            cp: [0, 1, 2, 3, 4, 5, 6, 7],
            co: [0; 8],
        }
    }
}

#[allow(clippy::wildcard_imports, reason = "facelet names")]
const CORNER_FACELET: [[u8; 3]; 8] = {
    use f3::*;
    [
        [U9, R1, F3],
        [U7, F1, L3],
        [U1, L1, B3],
        [U3, B1, R3],
        [D3, F9, R7],
        [D1, L9, F7],
        [D7, B9, L7],
        [D9, R9, B7],
    ]
};

static CORNER_MOVE_CUBE: LazyLock<[CornerCube; 18]> = LazyLock::new(|| {
    let mut mc = [CornerCube::default(); 18];
    mc[0] = CornerCube::from_coords(15120, 0);
    mc[3] = CornerCube::from_coords(21021, 1494);
    mc[6] = CornerCube::from_coords(8064, 1236);
    mc[9] = CornerCube::from_coords(9, 0);
    mc[12] = CornerCube::from_coords(1230, 412);
    mc[15] = CornerCube::from_coords(224, 137);
    for a in (0..18).step_by(3) {
        for p in 0..2 {
            mc[a + p + 1] = CornerCube::mult(&mc[a + p], &mc[a]);
        }
    }
    mc
});

impl CornerCube {
    pub(crate) fn random(r: &mut dyn RandomSource) -> Self {
        let cperm = r.next_int_bounded(40320);
        let twist = r.next_int_bounded(2187);
        Self::from_coords(cperm, twist)
    }

    pub(crate) fn from_coords(cperm: i32, twist: i32) -> Self {
        let mut c = Self::default();
        set8_perm(&mut c.cp, cperm);
        c.set_twist(twist);
        c
    }

    pub(crate) fn parity(&self) -> i32 {
        super::moves::parity(&self.cp)
    }

    pub(crate) fn fill_333_facelet(&self, facelet: &mut [u8; 54]) {
        for corn in 0..8 {
            let j = self.cp[corn] as usize;
            let ori = self.co[corn] as usize;
            for n in 0..3 {
                facelet[CORNER_FACELET[corn][(n + ori) % 3] as usize] =
                    FACES[(CORNER_FACELET[j][n] / 9) as usize];
            }
        }
    }

    /// `prod = a * b`, with mirrored orientations considered.
    fn mult(a: &Self, b: &Self) -> Self {
        let mut prod = Self::default();
        for corn in 0..8 {
            let bc = b.cp[corn] as usize;
            prod.cp[corn] = a.cp[bc];
            let ori_a = a.co[bc];
            let ori_b = b.co[corn];
            let mut ori = ori_a;
            ori += if ori_a < 3 { ori_b } else { 6 - ori_b };
            ori %= 3;
            if (ori_a >= 3) ^ (ori_b >= 3) {
                ori += 3;
            }
            prod.co[corn] = ori;
        }
        prod
    }

    fn set_twist(&mut self, mut idx: i32) {
        let mut twst = 0;
        for i in (0..7).rev() {
            self.co[i] = (idx % 3) as u8;
            twst += i32::from(self.co[i]);
            idx /= 3;
        }
        self.co[7] = ((15 - twst) % 3) as u8;
    }

    pub(crate) fn do_move(&mut self, idx: usize) {
        *self = Self::mult(self, &CORNER_MOVE_CUBE[idx]);
    }
}

/// The 24 wing edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EdgeCube {
    pub(crate) ep: [u8; 24],
}

impl Default for EdgeCube {
    fn default() -> Self {
        let mut ep = [0; 24];
        for (i, e) in ep.iter_mut().enumerate() {
            *e = i as u8;
        }
        Self { ep }
    }
}

const EDGE_COLOR: [[u8; 2]; 12] = [
    [2, 0],
    [4, 0],
    [5, 0],
    [1, 0],
    [5, 3],
    [4, 3],
    [2, 3],
    [1, 3],
    [2, 4],
    [5, 4],
    [5, 1],
    [2, 1],
];

#[allow(clippy::wildcard_imports, reason = "facelet names")]
const EDGE_MAP: [u8; 24] = {
    use f3::*;
    [
        F2, L2, B2, R2, B8, L8, F8, R8, F4, B6, B4, F6, U8, U4, U2, U6, D8, D4, D2, D6, L6, L4, R6,
        R4,
    ]
};

impl EdgeCube {
    pub(crate) fn random(r: &mut dyn RandomSource) -> Self {
        let mut c = Self::default();
        for i in 0..23 {
            let t = i + r.next_int_bounded(24 - i as i32) as usize;
            if t != i {
                c.ep.swap(i, t);
            }
        }
        c
    }

    pub(crate) fn parity(&self) -> i32 {
        super::moves::parity(&self.ep)
    }

    pub(crate) fn fill_333_facelet(&self, facelet: &mut [u8; 54]) {
        for i in 0..24 {
            let e = self.ep[i] as usize;
            facelet[EDGE_MAP[i] as usize] = FACES[EDGE_COLOR[e % 12][e / 12] as usize];
        }
    }

    /// Whether the edges are paired up (the reduction is complete).
    pub(crate) fn check_edge(&self) -> bool {
        let mut ck = 0_i32;
        let mut parity = false;
        for &e in &self.ep[..12] {
            ck |= 1 << e;
            parity = parity != (e >= 12);
        }
        ck &= ck >> 12;
        ck == 0 && !parity
    }

    pub(crate) fn do_move(&mut self, m: usize) {
        let key = m % 3;
        let ep = &mut self.ep;
        match m / 3 {
            0 => {
                swap4(ep, 0, 1, 2, 3, key);
                swap4(ep, 12, 13, 14, 15, key);
            }
            1 => {
                swap4(ep, 11, 15, 10, 19, key);
                swap4(ep, 23, 3, 22, 7, key);
            }
            2 => {
                swap4(ep, 0, 11, 6, 8, key);
                swap4(ep, 12, 23, 18, 20, key);
            }
            3 => {
                swap4(ep, 4, 5, 6, 7, key);
                swap4(ep, 16, 17, 18, 19, key);
            }
            4 => {
                swap4(ep, 1, 20, 5, 21, key);
                swap4(ep, 13, 8, 17, 9, key);
            }
            5 => {
                swap4(ep, 2, 9, 4, 10, key);
                swap4(ep, 14, 21, 16, 22, key);
            }
            6 => {
                swap4(ep, 0, 1, 2, 3, key);
                swap4(ep, 12, 13, 14, 15, key);
                swap4(ep, 9, 22, 11, 20, key);
            }
            7 => {
                swap4(ep, 11, 15, 10, 19, key);
                swap4(ep, 23, 3, 22, 7, key);
                swap4(ep, 2, 16, 6, 12, key);
            }
            8 => {
                swap4(ep, 0, 11, 6, 8, key);
                swap4(ep, 12, 23, 18, 20, key);
                swap4(ep, 3, 19, 5, 13, key);
            }
            9 => {
                swap4(ep, 4, 5, 6, 7, key);
                swap4(ep, 16, 17, 18, 19, key);
                swap4(ep, 8, 23, 10, 21, key);
            }
            10 => {
                swap4(ep, 1, 20, 5, 21, key);
                swap4(ep, 13, 8, 17, 9, key);
                swap4(ep, 14, 0, 18, 4, key);
            }
            11 => {
                swap4(ep, 2, 9, 4, 10, key);
                swap4(ep, 14, 21, 16, 22, key);
                swap4(ep, 7, 15, 1, 17, key);
            }
            _ => {}
        }
    }
}
