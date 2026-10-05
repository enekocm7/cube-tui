//! The cubie level representation of the 3x3x3 and its symmetry tables.

use super::util::{self, UTIL};

/// A 3x3x3 at the cubie level.
///
/// Each corner byte is `orientation << 3 | piece`, each edge byte is `piece << 1 | flip`.
/// Corner orientations 3..6 only appear in mirrored symmetries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CubieCube {
    pub(crate) ca: [u8; 8],
    pub(crate) ea: [u8; 12],
}

impl Default for CubieCube {
    fn default() -> Self {
        Self::SOLVED
    }
}

/// `0x00DDDD00`: maps edge symmetry indices to corner symmetry indices.
pub(crate) const SYM_E2C_MAGIC: i32 = 0x00DD_DD00;

pub(crate) const URF_MOVE: [[usize; 18]; 6] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
    [6, 7, 8, 0, 1, 2, 3, 4, 5, 15, 16, 17, 9, 10, 11, 12, 13, 14],
    [3, 4, 5, 6, 7, 8, 0, 1, 2, 12, 13, 14, 15, 16, 17, 9, 10, 11],
    [2, 1, 0, 5, 4, 3, 8, 7, 6, 11, 10, 9, 14, 13, 12, 17, 16, 15],
    [8, 7, 6, 2, 1, 0, 5, 4, 3, 17, 16, 15, 11, 10, 9, 14, 13, 12],
    [5, 4, 3, 8, 7, 6, 2, 1, 0, 14, 13, 12, 17, 16, 15, 11, 10, 9],
];

pub(crate) fn esym_to_csym(idx: i32) -> i32 {
    idx ^ (SYM_E2C_MAGIC >> ((idx & 0xf) << 1) & 3)
}

impl CubieCube {
    pub(crate) const SOLVED: Self = Self {
        ca: [0, 1, 2, 3, 4, 5, 6, 7],
        ea: [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22],
    };

    pub(crate) fn from_coords(cperm: i32, twist: i32, eperm: i32, flip: i32) -> Self {
        let mut c = Self::SOLVED;
        c.set_cperm(cperm);
        c.set_twist(twist);
        util::set_n_perm(&mut c.ea, eperm, 12, true);
        c.set_flip(flip);
        c
    }

    pub(crate) fn inv_cubie_cube(&mut self) {
        let mut temps = Self::SOLVED;
        for edge in 0..12 {
            temps.ea[(self.ea[edge] >> 1) as usize] = (edge as u8) << 1 | self.ea[edge] & 1;
        }
        for corn in 0..8 {
            temps.ca[(self.ca[corn] & 0x7) as usize] =
                corn as u8 | (0x20 >> (self.ca[corn] >> 3)) & 0x18;
        }
        *self = temps;
    }

    /// `prod = a * b`, corners only.
    pub(crate) fn corn_mult(a: &Self, b: &Self, prod: &mut Self) {
        for corn in 0..8 {
            let bc = b.ca[corn];
            let ori_a = a.ca[(bc & 7) as usize] >> 3;
            let ori_b = bc >> 3;
            prod.ca[corn] = a.ca[(bc & 7) as usize] & 7 | ((ori_a + ori_b) % 3) << 3;
        }
    }

    /// `prod = a * b`, corners only, with mirrored orientations considered.
    pub(crate) fn corn_mult_full(a: &Self, b: &Self, prod: &mut Self) {
        for corn in 0..8 {
            let bc = b.ca[corn];
            let ori_a = i32::from(a.ca[(bc & 7) as usize] >> 3);
            let ori_b = i32::from(bc >> 3);
            let mut ori = ori_a + if ori_a < 3 { ori_b } else { 6 - ori_b };
            ori = ori % 3 + if (ori_a < 3) == (ori_b < 3) { 0 } else { 3 };
            prod.ca[corn] = a.ca[(bc & 7) as usize] & 7 | (ori << 3) as u8;
        }
    }

    /// `prod = a * b`, edges only.
    pub(crate) fn edge_mult(a: &Self, b: &Self, prod: &mut Self) {
        for ed in 0..12 {
            prod.ea[ed] = a.ea[(b.ea[ed] >> 1) as usize] ^ (b.ea[ed] & 1);
        }
    }

    pub(crate) fn mult(a: &Self, b: &Self, prod: &mut Self) {
        Self::corn_mult(a, b, prod);
        Self::edge_mult(a, b, prod);
    }

    /// `b = S_idx^-1 * a * S_idx`, corners only.
    pub(crate) fn corn_conjugate(t: &SymTables, a: &Self, idx: usize, b: &mut Self) {
        let sinv = &t.cube_sym[t.sym_mult_inv[0][idx]];
        let s = &t.cube_sym[idx];
        for corn in 0..8 {
            let ac = a.ca[(s.ca[corn] & 7) as usize];
            let ori_a = sinv.ca[(ac & 7) as usize] >> 3;
            let ori_b = ac >> 3;
            let ori = if ori_a < 3 { ori_b } else { (3 - ori_b) % 3 };
            b.ca[corn] = sinv.ca[(ac & 7) as usize] & 7 | ori << 3;
        }
    }

    /// `b = S_idx^-1 * a * S_idx`, edges only.
    pub(crate) fn edge_conjugate(t: &SymTables, a: &Self, idx: usize, b: &mut Self) {
        let sinv = &t.cube_sym[t.sym_mult_inv[0][idx]];
        let s = &t.cube_sym[idx];
        for ed in 0..12 {
            let ae = a.ea[(s.ea[ed] >> 1) as usize];
            b.ea[ed] = sinv.ea[(ae >> 1) as usize] ^ (ae & 1) ^ (s.ea[ed] & 1);
        }
    }

    /// `this = S_urf^-1 * this * S_urf`.
    pub(crate) fn urf_conjugate(&mut self, t: &SymTables) {
        let mut temps = Self::SOLVED;
        Self::corn_mult(&t.urf2, self, &mut temps);
        Self::corn_mult(&temps, &t.urf1, self);
        Self::edge_mult(&t.urf2, self, &mut temps);
        Self::edge_mult(&temps, &t.urf1, self);
    }

    // ----- Phase 1 coordinates -----

    pub(crate) fn flip(&self) -> i32 {
        self.ea[..11]
            .iter()
            .fold(0, |idx, &e| idx << 1 | i32::from(e & 1))
    }

    pub(crate) fn set_flip(&mut self, mut idx: i32) {
        let mut parity = 0;
        for i in (0..11).rev() {
            let val = (idx & 1) as u8;
            parity ^= val;
            self.ea[i] = self.ea[i] & 0xfe | val;
            idx >>= 1;
        }
        self.ea[11] = self.ea[11] & 0xfe | parity;
    }

    pub(crate) fn twist(&self) -> i32 {
        self.ca[..7]
            .iter()
            .fold(0, |idx, &c| idx + (idx << 1) + i32::from(c >> 3))
    }

    pub(crate) fn set_twist(&mut self, mut idx: i32) {
        let mut twst = 15;
        for i in (0..7).rev() {
            let val = idx % 3;
            twst -= val;
            self.ca[i] = self.ca[i] & 0x7 | (val << 3) as u8;
            idx /= 3;
        }
        self.ca[7] = self.ca[7] & 0x7 | ((twst % 3) << 3) as u8;
    }

    pub(crate) fn ud_slice(&self) -> i32 {
        494 - util::get_comb(&self.ea, 8, true)
    }

    pub(crate) fn set_ud_slice(&mut self, idx: i32) {
        util::set_comb(&mut self.ea, 494 - idx, 8, true);
    }

    // ----- Phase 2 coordinates -----

    pub(crate) fn cperm(&self) -> i32 {
        util::get_n_perm(&self.ca, 8, false)
    }

    pub(crate) fn set_cperm(&mut self, idx: i32) {
        util::set_n_perm(&mut self.ca, idx, 8, false);
    }

    pub(crate) fn eperm(&self) -> i32 {
        util::get_n_perm(&self.ea, 8, true)
    }

    pub(crate) fn set_eperm(&mut self, idx: i32) {
        util::set_n_perm(&mut self.ea, idx, 8, true);
    }

    pub(crate) fn mperm(&self) -> i32 {
        util::get_n_perm(&self.ea, 12, true) % 24
    }

    pub(crate) fn set_mperm(&mut self, idx: i32) {
        util::set_n_perm(&mut self.ea, idx, 12, true);
    }

    pub(crate) fn ccomb(&self) -> i32 {
        util::get_comb(&self.ca, 0, false)
    }

    pub(crate) fn set_ccomb(&mut self, idx: i32) {
        util::set_comb(&mut self.ca, idx, 0, false);
    }

    /// Checks the cube for solvability, returning min2phase's error code:
    /// `0` solvable, `-2` missing edges, `-3` flipped edge, `-4` missing corners,
    /// `-5` twisted corner, `-6` parity error.
    pub(crate) fn verify(&self) -> i32 {
        let mut sum = 0;
        let mut edge_mask = 0;
        for &e in &self.ea {
            edge_mask |= 1_i32.wrapping_shl(u32::from(e >> 1));
            sum ^= e & 1;
        }
        if edge_mask != 0xfff {
            return -2;
        }
        if sum != 0 {
            return -3;
        }
        let mut corn_mask = 0;
        let mut sum = 0;
        for &c in &self.ca {
            corn_mask |= 1 << (c & 7);
            sum += i32::from(c >> 3);
        }
        if corn_mask != 0xff {
            return -4;
        }
        if sum % 3 != 0 {
            return -5;
        }
        if self.edge_parity_bit() ^ self.corner_parity_bit() != 0 {
            return -6;
        }
        0
    }

    pub(crate) fn edge_parity_bit(&self) -> i32 {
        util::get_n_parity(util::get_n_perm(&self.ea, 12, true), 12)
    }

    pub(crate) fn corner_parity_bit(&self) -> i32 {
        util::get_n_parity(self.cperm(), 8)
    }

    /// A bitmask of the symmetries (and anti-symmetries) this cube has.
    pub(crate) fn self_symmetry(&self, t: &SymTables) -> i64 {
        let mut c = *self;
        let mut d = Self::SOLVED;
        let mut sym = 0_i64;
        for i in 0..96 {
            Self::corn_conjugate(t, &c, t.sym_mult_inv[0][i % 16], &mut d);
            if d.ca == self.ca {
                Self::edge_conjugate(t, &c, t.sym_mult_inv[0][i % 16], &mut d);
                if d.ea == self.ea {
                    sym |= 1_i64 << i.min(48);
                }
            }
            if i % 16 == 15 {
                c.urf_conjugate(t);
            }
            if i % 48 == 47 {
                c.inv_cubie_cube();
            }
        }
        sym
    }
}

/// The symmetry tables (`CubieCube`'s static initialisation: `initMove` and `initSym`).
pub(crate) struct SymTables {
    /// 16 symmetries generated by `S_F2`, `S_U4` and `S_LR2`.
    pub(crate) cube_sym: [CubieCube; 16],
    /// The 18 face turns.
    pub(crate) move_cube: [CubieCube; 18],
    pub(crate) move_cube_sym: [i64; 18],
    pub(crate) first_move_sym: [i32; 48],
    pub(crate) sym_mult: [[usize; 16]; 16],
    pub(crate) sym_mult_inv: [[usize; 16]; 16],
    pub(crate) sym_move: [[usize; 18]; 16],
    pub(crate) sym8_move: [usize; 8 * 18],
    pub(crate) sym_move_ud: [[usize; 18]; 16],
    pub(crate) urf1: CubieCube,
    pub(crate) urf2: CubieCube,
}

impl SymTables {
    pub(crate) fn build() -> Self {
        let mut move_cube = [CubieCube::SOLVED; 18];
        move_cube[0] = CubieCube::from_coords(15120, 0, 119_750_400, 0);
        move_cube[3] = CubieCube::from_coords(21021, 1494, 323_403_417, 0);
        move_cube[6] = CubieCube::from_coords(8064, 1236, 29_441_808, 550);
        move_cube[9] = CubieCube::from_coords(9, 0, 5880, 0);
        move_cube[12] = CubieCube::from_coords(1230, 412, 2_949_660, 0);
        move_cube[15] = CubieCube::from_coords(224, 137, 328_552, 137);
        for a in (0..18).step_by(3) {
            for p in 0..2 {
                let mut next = CubieCube::SOLVED;
                CubieCube::mult(&move_cube[a + p], &move_cube[a], &mut next);
                move_cube[a + p + 1] = next;
            }
        }

        let mut t = Self {
            cube_sym: [CubieCube::SOLVED; 16],
            move_cube,
            move_cube_sym: [0; 18],
            first_move_sym: [0; 48],
            sym_mult: [[0; 16]; 16],
            sym_mult_inv: [[0; 16]; 16],
            sym_move: [[0; 18]; 16],
            sym8_move: [0; 8 * 18],
            sym_move_ud: [[0; 18]; 16],
            urf1: CubieCube::from_coords(2531, 1373, 67_026_819, 1367),
            urf2: CubieCube::from_coords(2089, 1906, 322_752_913, 2040),
        };

        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        let f2 = CubieCube::from_coords(28783, 0, 259_268_407, 0);
        let u4 = CubieCube::from_coords(15138, 0, 119_765_538, 7);
        let mut lr2 = CubieCube::from_coords(5167, 0, 83_473_207, 0);
        for ca in &mut lr2.ca {
            *ca |= 3 << 3;
        }
        for i in 0..16 {
            t.cube_sym[i] = c;
            CubieCube::corn_mult_full(&c, &u4, &mut d);
            CubieCube::edge_mult(&c, &u4, &mut d);
            std::mem::swap(&mut c, &mut d);
            if i % 4 == 3 {
                CubieCube::corn_mult_full(&c, &lr2, &mut d);
                CubieCube::edge_mult(&c, &lr2, &mut d);
                std::mem::swap(&mut c, &mut d);
            }
            if i % 8 == 7 {
                CubieCube::corn_mult_full(&c, &f2, &mut d);
                CubieCube::edge_mult(&c, &f2, &mut d);
                std::mem::swap(&mut c, &mut d);
            }
        }
        for i in 0..16 {
            for j in 0..16 {
                CubieCube::corn_mult_full(&t.cube_sym[i], &t.cube_sym[j], &mut c);
                if let Some(k) = (0..16).find(|&k| t.cube_sym[k].ca == c.ca) {
                    t.sym_mult[i][j] = k;
                    t.sym_mult_inv[k][j] = i;
                }
            }
        }
        let std2ud = &UTIL.std2ud;
        for j in 0..18 {
            for s in 0..16 {
                CubieCube::corn_conjugate(&t, &t.move_cube[j], t.sym_mult_inv[0][s], &mut c);
                if let Some(m) = (0..18).find(|&m| t.move_cube[m].ca == c.ca) {
                    t.sym_move[s][j] = m;
                    t.sym_move_ud[s][std2ud[j]] = std2ud[m];
                }
                if s % 2 == 0 {
                    t.sym8_move[j << 3 | s >> 1] = t.sym_move[s][j];
                }
            }
        }
        for i in 0..18 {
            t.move_cube_sym[i] = t.move_cube[i].self_symmetry(&t);
            let mut j = i;
            for s in 0..48 {
                if t.sym_move[s % 16][j] < i {
                    t.first_move_sym[s] |= 1 << i;
                }
                if s % 16 == 15 {
                    j = URF_MOVE[2][j];
                }
            }
        }
        t
    }

    /// `CubieCube.getSkipMoves`: moves that are redundant because of the symmetries in `ssym`.
    pub(crate) fn skip_moves(&self, mut ssym: i64) -> i32 {
        let mut ret = 0;
        let mut i = 1;
        loop {
            ssym >>= 1;
            if ssym == 0 {
                break;
            }
            if ssym & 1 == 1 {
                ret |= self.first_move_sym[i];
            }
            i += 1;
        }
        ret
    }
}
