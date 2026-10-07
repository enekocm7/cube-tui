//! The center coordinates of the three phases (`Center1`, `Center2`, `Center3`).

use super::cubes::{CENTER_FACELET, CenterCube, center_move};
use super::moves::{MOVE2STD, MOVES, WB3, WD2, WD3, WF1, WL3, WR1, WU1, WU2, swap4};
use crate::parallel;

/// Applies the `j`-th step of the walk that enumerates the 48 cube symmetries.
fn sym_step(j: usize, mut rot: impl FnMut(usize)) {
    rot(0);
    if j % 2 == 1 {
        rot(1);
    }
    if j % 8 == 7 {
        rot(2);
    }
    if j % 16 == 15 {
        rot(3);
    }
}

// ----------------------------------------------------------------------------------------
// Phase 1: separate the U/D centers from the rest.

pub(crate) const N_CENTER1_RAW: usize = 735_471;
pub(crate) const N_CENTER1_SYM: usize = 15582;

/// Which of the 24 center stickers belong to the tracked pair of faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Center1 {
    pub(crate) ct: [u8; 24],
}

impl Default for Center1 {
    fn default() -> Self {
        let mut ct = [0; 24];
        ct[..8].fill(1);
        Self { ct }
    }
}

impl Center1 {
    /// The stickers of `c` on the axis `urf` (0 = U/D, 1 = R/L, 2 = F/B).
    pub(crate) fn from_center(c: &CenterCube, urf: u8) -> Self {
        let mut ct = [0; 24];
        for (t, &s) in ct.iter_mut().zip(&c.ct) {
            *t = u8::from(s % 3 == urf);
        }
        Self { ct }
    }

    fn do_move(&mut self, m: usize) {
        center_move(&mut self.ct, m);
    }

    pub(crate) fn set(&mut self, mut idx: i32) {
        let cnk = &MOVES.cnk;
        let mut r = 8;
        for i in (0..24).rev() {
            self.ct[i] = 0;
            if idx >= cnk[i][r] {
                idx -= cnk[i][r];
                r -= 1;
                self.ct[i] = 1;
            }
        }
    }

    pub(crate) fn get(&self) -> i32 {
        let cnk = &MOVES.cnk;
        let mut idx = 0;
        let mut r = 8;
        for i in (0..24).rev() {
            if self.ct[i] == 1 {
                idx += cnk[i][r];
                r -= 1;
            }
        }
        idx
    }

    /// The symmetry coordinate, found by rotating until a class representative is reached
    /// (`getsym` once the raw-to-symmetry table has been released).
    pub(crate) fn get_sym(&mut self, sym2raw: &[i32]) -> i32 {
        for j in 0..48 {
            if let Ok(cord) = sym2raw.binary_search(&self.get()) {
                return cord as i32 * 64 + j as i32;
            }
            sym_step(j, |r| self.rot(r));
        }
        -1
    }

    pub(crate) fn rot(&mut self, r: usize) {
        match r {
            0 => {
                self.do_move(WU2);
                self.do_move(WD2);
            }
            1 => {
                self.do_move(WR1);
                self.do_move(WL3);
            }
            2 => {
                let ct = &mut self.ct;
                swap4(ct, 0, 3, 1, 2, 1);
                swap4(ct, 8, 11, 9, 10, 1);
                swap4(ct, 4, 7, 5, 6, 1);
                swap4(ct, 12, 15, 13, 14, 1);
                swap4(ct, 16, 19, 21, 22, 1);
                swap4(ct, 17, 18, 20, 23, 1);
            }
            3 => {
                self.do_move(WU1);
                self.do_move(WD3);
                self.do_move(WF1);
                self.do_move(WB3);
            }
            _ => {}
        }
    }

    fn rotate(&mut self, r: usize) {
        for j in 0..r {
            sym_step(j, |x| self.rot(x));
        }
    }

    /// The symmetry that brings the (solved) centers of `cube` back to their home faces.
    pub(crate) fn solved_sym(cube: &CenterCube) -> usize {
        let mut c = Self { ct: cube.ct };
        for j in 0..48 {
            if c.ct.iter().zip(CENTER_FACELET).all(|(&a, f)| a == f / 16) {
                return j;
            }
            sym_step(j, |r| c.rot(r));
        }
        usize::MAX
    }
}

/// `Center1`'s static tables.
pub(crate) struct Center1Tables {
    pub(crate) ctsmv: Vec<[i32; 36]>,
    pub(crate) sym2raw: Vec<i32>,
    pub(crate) csprun: Vec<i8>,
    pub(crate) symmult: [[u8; 48]; 48],
    pub(crate) symmove: [[u8; 36]; 48],
    pub(crate) syminv: [u8; 48],
    pub(crate) finish: [i32; 48],
}

impl Center1Tables {
    pub(crate) fn build() -> Self {
        let mut t = Self {
            ctsmv: vec![[0; 36]; N_CENTER1_SYM],
            sym2raw: vec![0; N_CENTER1_SYM],
            csprun: vec![0; N_CENTER1_SYM],
            symmult: [[0; 48]; 48],
            symmove: [[0; 36]; 48],
            syminv: [0; 48],
            finish: [0; 48],
        };
        t.init_sym();
        let raw2sym = t.init_sym2raw();
        t.create_move_table(&raw2sym);
        t.create_prun();
        t
    }

    fn init_sym(&mut self) {
        let mut c = Center1 { ct: [0; 24] };
        for (i, v) in c.ct.iter_mut().enumerate() {
            *v = i as u8;
        }
        let mut d = c;
        let e = c;
        for i in 0..48 {
            for j in 0..48 {
                for k in 0..48 {
                    if c == d {
                        self.symmult[i][j] = k as u8;
                        if k == 0 {
                            self.syminv[i] = j as u8;
                        }
                    }
                    sym_step(k, |r| d.rot(r));
                }
                sym_step(j, |r| c.rot(r));
            }
            sym_step(i, |r| c.rot(r));
        }

        for i in 0..48 {
            c = e;
            c.rotate(usize::from(self.syminv[i]));
            for j in 0..36 {
                d = c;
                d.do_move(j);
                d.rotate(i);
                for k in 0..36 {
                    let mut f = e;
                    f.do_move(k);
                    if f == d {
                        self.symmove[i][j] = k as u8;
                        break;
                    }
                }
            }
        }

        c.set(0);
        for i in 0..48 {
            self.finish[usize::from(self.syminv[i])] = c.get();
            sym_step(i, |r| c.rot(r));
        }
    }

    /// Enumerates the symmetry classes; returns the temporary raw-to-symmetry table.
    fn init_sym2raw(&mut self) -> Vec<i32> {
        let mut raw2sym = vec![0_i32; N_CENTER1_RAW];
        let mut c = Center1::default();
        let mut occ = vec![0_u32; N_CENTER1_RAW / 32 + 1];
        let mut count = 0;
        for i in 0..N_CENTER1_RAW {
            if occ[i >> 5] & (1 << (i & 0x1f)) == 0 {
                c.set(i as i32);
                for j in 0..48 {
                    let idx = c.get() as usize;
                    occ[idx >> 5] |= 1 << (idx & 0x1f);
                    raw2sym[idx] = (count << 6 | usize::from(self.syminv[j])) as i32;
                    sym_step(j, |r| c.rot(r));
                }
                self.sym2raw[count] = i as i32;
                count += 1;
            }
        }
        debug_assert_eq!(count, N_CENTER1_SYM);
        raw2sym
    }

    fn create_move_table(&mut self, raw2sym: &[i32]) {
        let sym2raw = &self.sym2raw;
        parallel::fill(&mut self.ctsmv, |i, row| {
            let mut d = Center1::default();
            d.set(sym2raw[i]);
            for (m, v) in row.iter_mut().enumerate() {
                let mut c = d;
                c.do_move(m);
                *v = raw2sym[c.get() as usize];
            }
        });
    }

    fn create_prun(&mut self) {
        self.csprun.fill(-1);
        self.csprun[0] = 0;
        let mut depth = 0;
        let mut done = 1;
        while done != N_CENTER1_SYM {
            let inv = depth > 4;
            let select = if inv { -1 } else { depth };
            let check = if inv { depth } else { -1 };
            depth += 1;
            for i in 0..N_CENTER1_SYM {
                if i32::from(self.csprun[i]) != select {
                    continue;
                }
                for m in 0..27 {
                    let idx = (self.ctsmv[i][m] as u32 >> 6) as usize;
                    if i32::from(self.csprun[idx]) != check {
                        continue;
                    }
                    done += 1;
                    if inv {
                        self.csprun[i] = depth as i8;
                        break;
                    }
                    self.csprun[idx] = depth as i8;
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------------------
// Phase 2: solve the R/L centers and pair the F/B centers with U/D.

/// The phase 2 center coordinates.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Center2 {
    rl: [i32; 8],
    ct: [i32; 16],
    parity: i32,
}

const CENTER2_PMV: [i32; 36] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0,
    1, 0, 0, 0,
];

pub(crate) const N_CENTER2_CT: usize = 6435;
pub(crate) const N_CENTER2_RL: usize = 70;

impl Center2 {
    pub(crate) fn set(&mut self, c: &CenterCube, edge_parity: i32) {
        for i in 0..16 {
            self.ct[i] = i32::from(c.ct[i] % 3);
        }
        for i in 0..8 {
            self.rl[i] = i32::from(c.ct[i + 16]);
        }
        self.parity = edge_parity;
    }

    pub(crate) fn get_rl(&self) -> i32 {
        let cnk = &MOVES.cnk;
        let mut idx = 0;
        let mut r = 4;
        for i in (0..7).rev() {
            if self.rl[i] != self.rl[7] {
                idx += cnk[i][r];
                r -= 1;
            }
        }
        idx * 2 + self.parity
    }

    fn set_rl(&mut self, idx: i32) {
        let cnk = &MOVES.cnk;
        self.parity = idx & 1;
        let mut idx = idx >> 1;
        let mut r = 4;
        self.rl[7] = 0;
        for i in (0..7).rev() {
            if idx >= cnk[i][r] {
                idx -= cnk[i][r];
                r -= 1;
                self.rl[i] = 1;
            } else {
                self.rl[i] = 0;
            }
        }
    }

    pub(crate) fn get_ct(&self) -> i32 {
        let cnk = &MOVES.cnk;
        let mut idx = 0;
        let mut r = 8;
        for i in (0..15).rev() {
            if self.ct[i] != self.ct[15] {
                idx += cnk[i][r];
                r -= 1;
            }
        }
        idx
    }

    fn set_ct(&mut self, mut idx: i32) {
        let cnk = &MOVES.cnk;
        let mut r = 8;
        self.ct[15] = 0;
        for i in (0..15).rev() {
            if idx >= cnk[i][r] {
                idx -= cnk[i][r];
                r -= 1;
                self.ct[i] = 1;
            } else {
                self.ct[i] = 0;
            }
        }
    }

    fn do_move(&mut self, m: usize) {
        self.parity ^= CENTER2_PMV[m];
        let key = m % 3;
        let (ct, rl) = (&mut self.ct, &mut self.rl);
        match m / 3 {
            0 => swap4(ct, 0, 1, 2, 3, key),
            1 => swap4(rl, 0, 1, 2, 3, key),
            2 => swap4(ct, 8, 9, 10, 11, key),
            3 => swap4(ct, 4, 5, 6, 7, key),
            4 => swap4(rl, 4, 5, 6, 7, key),
            5 => swap4(ct, 12, 13, 14, 15, key),
            6 => {
                swap4(ct, 0, 1, 2, 3, key);
                swap4(rl, 0, 5, 4, 1, key);
                swap4(ct, 8, 9, 12, 13, key);
            }
            7 => {
                swap4(rl, 0, 1, 2, 3, key);
                swap4(ct, 1, 15, 5, 9, key);
                swap4(ct, 2, 12, 6, 10, key);
            }
            8 => {
                swap4(ct, 8, 9, 10, 11, key);
                swap4(rl, 0, 3, 6, 5, key);
                swap4(ct, 3, 2, 5, 4, key);
            }
            9 => {
                swap4(ct, 4, 5, 6, 7, key);
                swap4(rl, 3, 2, 7, 6, key);
                swap4(ct, 11, 10, 15, 14, key);
            }
            10 => {
                swap4(rl, 4, 5, 6, 7, key);
                swap4(ct, 0, 8, 4, 14, key);
                swap4(ct, 3, 11, 7, 13, key);
            }
            11 => {
                swap4(ct, 12, 13, 14, 15, key);
                swap4(rl, 1, 4, 7, 2, key);
                swap4(ct, 1, 0, 7, 6, key);
            }
            _ => {}
        }
    }
}

/// `Center2`'s static tables. (The Java source also builds rotation tables that nothing
/// reads; they are omitted.)
pub(crate) struct Center2Tables {
    pub(crate) rlmv: Vec<[i32; 28]>,
    pub(crate) ctmv: Vec<[u16; 28]>,
    pub(crate) ctprun: Vec<i8>,
}

impl Center2Tables {
    pub(crate) fn build() -> Self {
        let mut c = Center2::default();
        let mut rlmv = vec![[0; 28]; N_CENTER2_RL];
        for (i, row) in rlmv.iter_mut().enumerate() {
            for (m, v) in row.iter_mut().enumerate() {
                c.set_rl(i as i32);
                c.do_move(MOVE2STD[m]);
                *v = c.get_rl();
            }
        }
        let mut ctmv = vec![[0; 28]; N_CENTER2_CT];
        for (i, row) in ctmv.iter_mut().enumerate() {
            for (m, v) in row.iter_mut().enumerate() {
                c.set_ct(i as i32);
                c.do_move(MOVE2STD[m]);
                *v = c.get_ct() as u16;
            }
        }
        let n = N_CENTER2_CT * N_CENTER2_RL;
        let mut ctprun = vec![-1_i8; n];
        for i in [0, 18, 28, 46, 54, 56] {
            ctprun[i] = 0;
        }
        let mut depth = 0;
        let mut done = 6;
        while done != n {
            for i in 0..n {
                if i32::from(ctprun[i]) != depth {
                    continue;
                }
                let ct = i / 70;
                let rl = i % 70;
                for m in 0..23 {
                    let idx = usize::from(ctmv[ct][m]) * 70 + rlmv[rl][m] as usize;
                    if ctprun[idx] == -1 {
                        ctprun[idx] = (depth + 1) as i8;
                        done += 1;
                    }
                }
            }
            depth += 1;
        }
        Self { rlmv, ctmv, ctprun }
    }
}

// ----------------------------------------------------------------------------------------
// Phase 3: solve all the centers.

pub(crate) const N_CENTER3: usize = 35 * 35 * 12 * 2;

/// The phase 3 center coordinates.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Center3 {
    ud: [i32; 8],
    rl: [i32; 8],
    fb: [i32; 8],
    parity: i32,
}

const CENTER3_PMOVE: [i32; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1];
const RL2STD: [i32; 12] = [0, 9, 14, 23, 27, 28, 41, 42, 46, 55, 60, 69];

const STD2RL: [usize; 70] = {
    let mut std2rl = [0; 70];
    let mut i = 0;
    while i < RL2STD.len() {
        std2rl[RL2STD[i] as usize] = i;
        i += 1;
    }
    std2rl
};

impl Center3 {
    pub(crate) fn set(&mut self, c: &CenterCube, exc_parity: i32) {
        let m = |i: usize| c.ct[i] % 3;
        let parity = i32::from(!((m(0) > m(8)) ^ (m(8) > m(16)) ^ (m(0) > m(16))));
        for i in 0..8 {
            self.ud[i] = i32::from(c.ct[i] / 3) ^ 1;
            self.fb[i] = i32::from(c.ct[i + 8] / 3) ^ 1;
            self.rl[i] = i32::from(c.ct[i + 16] / 3) ^ 1 ^ parity;
        }
        self.parity = parity ^ exc_parity;
    }

    pub(crate) fn get_ct(&self) -> i32 {
        let cnk = &MOVES.cnk;
        let mut idx = 0;
        let mut r = 4;
        for i in (0..7).rev() {
            if self.ud[i] != self.ud[7] {
                idx += cnk[i][r];
                r -= 1;
            }
        }
        idx *= 35;
        r = 4;
        for i in (0..7).rev() {
            if self.fb[i] != self.fb[7] {
                idx += cnk[i][r];
                r -= 1;
            }
        }
        idx *= 12;
        let check = self.fb[7] ^ self.ud[7];
        let mut idxrl = 0;
        r = 4;
        for i in (0..8).rev() {
            if self.rl[i] != check {
                idxrl += cnk[i][r];
                r -= 1;
            }
        }
        self.parity + 2 * (idx + STD2RL[idxrl as usize] as i32)
    }

    fn set_ct(&mut self, idx: i32) {
        let cnk = &MOVES.cnk;
        self.parity = idx & 1;
        let mut idx = idx >> 1;
        let mut idxrl = RL2STD[(idx % 12) as usize];
        idx /= 12;
        let mut r = 4;
        for i in (0..8).rev() {
            self.rl[i] = 0;
            if idxrl >= cnk[i][r] {
                idxrl -= cnk[i][r];
                r -= 1;
                self.rl[i] = 1;
            }
        }
        let mut idxfb = idx % 35;
        idx /= 35;
        r = 4;
        self.fb[7] = 0;
        for i in (0..7).rev() {
            if idxfb >= cnk[i][r] {
                idxfb -= cnk[i][r];
                r -= 1;
                self.fb[i] = 1;
            } else {
                self.fb[i] = 0;
            }
        }
        r = 4;
        self.ud[7] = 0;
        for i in (0..7).rev() {
            if idx >= cnk[i][r] {
                idx -= cnk[i][r];
                r -= 1;
                self.ud[i] = 1;
            } else {
                self.ud[i] = 0;
            }
        }
    }

    fn do_move(&mut self, i: usize) {
        self.parity ^= CENTER3_PMOVE[i];
        let (ud, rl, fb) = (&mut self.ud, &mut self.rl, &mut self.fb);
        match i {
            0..=2 => swap4(ud, 0, 1, 2, 3, i % 3),
            3 => swap4(rl, 0, 1, 2, 3, 1),
            4..=6 => swap4(fb, 0, 1, 2, 3, (i - 1) % 3),
            7..=9 => swap4(ud, 4, 5, 6, 7, (i - 1) % 3),
            10 => swap4(rl, 4, 5, 6, 7, 1),
            11..=13 => swap4(fb, 4, 5, 6, 7, (i + 1) % 3),
            14 => {
                swap4(ud, 0, 1, 2, 3, 1);
                swap4(rl, 0, 5, 4, 1, 1);
                swap4(fb, 0, 5, 4, 1, 1);
            }
            15 => {
                swap4(rl, 0, 1, 2, 3, 1);
                swap4(fb, 1, 4, 7, 2, 1);
                swap4(ud, 1, 6, 5, 2, 1);
            }
            16 => {
                swap4(fb, 0, 1, 2, 3, 1);
                swap4(ud, 3, 2, 5, 4, 1);
                swap4(rl, 0, 3, 6, 5, 1);
            }
            17 => {
                swap4(ud, 4, 5, 6, 7, 1);
                swap4(rl, 3, 2, 7, 6, 1);
                swap4(fb, 3, 2, 7, 6, 1);
            }
            18 => {
                swap4(rl, 4, 5, 6, 7, 1);
                swap4(fb, 0, 3, 6, 5, 1);
                swap4(ud, 0, 3, 4, 7, 1);
            }
            19 => {
                swap4(fb, 4, 5, 6, 7, 1);
                swap4(ud, 0, 7, 6, 1, 1);
                swap4(rl, 1, 4, 7, 2, 1);
            }
            _ => {}
        }
    }
}

/// `Center3`'s static tables.
pub(crate) struct Center3Tables {
    pub(crate) ctmove: Vec<[u16; 20]>,
    pub(crate) prun: Vec<i8>,
}

impl Center3Tables {
    pub(crate) fn build() -> Self {
        let mut c = Center3::default();
        let mut ctmove = vec![[0; 20]; N_CENTER3];
        for (i, row) in ctmove.iter_mut().enumerate() {
            for (m, v) in row.iter_mut().enumerate() {
                c.set_ct(i as i32);
                c.do_move(m);
                *v = c.get_ct() as u16;
            }
        }
        let mut prun = vec![-1_i8; N_CENTER3];
        prun[0] = 0;
        let mut depth = 0;
        let mut done = 1;
        while done != N_CENTER3 {
            for i in 0..N_CENTER3 {
                if i32::from(prun[i]) != depth {
                    continue;
                }
                for m in 0..17 {
                    let next = usize::from(ctmove[i][m]);
                    if prun[next] == -1 {
                        prun[next] = (depth + 1) as i8;
                        done += 1;
                    }
                }
            }
            depth += 1;
        }
        Self { ctmove, prun }
    }
}
