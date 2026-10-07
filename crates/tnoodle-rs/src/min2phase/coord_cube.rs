//! Coordinate move tables, symmetry reductions and pruning tables for min2phase.

use std::sync::LazyLock;

use super::cubie_cube::{CubieCube, SYM_E2C_MAGIC, SymTables, esym_to_csym};
use super::util::{UD2STD, get_comb, get_n_parity};
use crate::parallel;

pub(crate) const N_MOVES: usize = 18;
pub(crate) const N_MOVES2: usize = 10;
pub(crate) const N_SLICE: usize = 495;
pub(crate) const N_TWIST: usize = 2187;
pub(crate) const N_TWIST_SYM: usize = 324;
pub(crate) const N_FLIP: usize = 2048;
pub(crate) const N_FLIP_SYM: usize = 336;
pub(crate) const N_PERM: usize = 40320;
pub(crate) const N_PERM_SYM: usize = 2768;
pub(crate) const N_MPERM: usize = 24;
pub(crate) const N_COMB: usize = 140;
const P2_PARITY_MOVE: i32 = 0xA5;

/// Every table min2phase needs, built once and shared by all searches.
pub(crate) struct Tables {
    pub(crate) sym: SymTables,

    // Symmetry class representatives and raw-to-symmetry coordinate maps.
    pub(crate) flip_s2r: Vec<u16>,
    pub(crate) twist_s2r: Vec<u16>,
    pub(crate) eperm_s2r: Vec<u16>,
    pub(crate) perm2combp: Vec<u8>,
    pub(crate) perm_inv_edge_sym: Vec<u16>,
    pub(crate) mperm_inv: [u8; N_MPERM],
    pub(crate) flip_r2s: Vec<u16>,
    pub(crate) twist_r2s: Vec<u16>,
    pub(crate) eperm_r2s: Vec<u16>,
    pub(crate) flip_s2rf: Vec<u16>,
    pub(crate) sym_state_twist: Vec<u16>,
    pub(crate) sym_state_flip: Vec<u16>,
    pub(crate) sym_state_perm: Vec<u16>,

    // Phase 1.
    pub(crate) ud_slice_move: Vec<[u16; N_MOVES]>,
    pub(crate) twist_move: Vec<[u16; N_MOVES]>,
    pub(crate) flip_move: Vec<[u16; N_MOVES]>,
    pub(crate) ud_slice_conj: Vec<[u16; 8]>,
    pub(crate) ud_slice_twist_prun: Vec<i32>,
    pub(crate) ud_slice_flip_prun: Vec<i32>,
    pub(crate) twist_flip_prun: Vec<i32>,

    // Phase 2.
    pub(crate) cperm_move: Vec<[u16; N_MOVES2]>,
    pub(crate) eperm_move: Vec<[u16; N_MOVES2]>,
    pub(crate) mperm_move: Vec<[u16; N_MOVES2]>,
    pub(crate) mperm_conj: Vec<[u16; 16]>,
    pub(crate) ccombp_move: Vec<[u16; N_MOVES2]>,
    pub(crate) ccombp_conj: Vec<[u16; 16]>,
    pub(crate) mcperm_prun: Vec<i32>,
    pub(crate) eperm_ccombp_prun: Vec<i32>,
}

/// The lazily built tables. Building takes a fraction of a second in release builds.
pub(crate) static TABLES: LazyLock<Tables> = LazyLock::new(Tables::build);

/// Reads the 4-bit pruning value at `index` (Java masks shift distances to 5 bits).
pub(crate) fn get_pruning(table: &[i32], index: usize) -> i32 {
    table[index >> 3] >> ((index << 2) & 31) & 0xf
}

fn set_pruning(table: &mut [i32], index: usize, value: i32) {
    table[index >> 3] ^= value << ((index << 2) & 31);
}

fn has_zero(val: i32) -> bool {
    (val.wrapping_sub(0x1111_1111) & !val & 0x8888_8888_u32 as i32) != 0
}

/// The raw coordinate whose symmetry classes are enumerated by `init_sym2raw`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RawCoord {
    Flip,
    Twist,
    EPerm,
}

impl CubieCube {
    pub(crate) fn flip_sym(&self, t: &Tables) -> i32 {
        i32::from(t.flip_r2s[self.flip() as usize])
    }

    pub(crate) fn twist_sym(&self, t: &Tables) -> i32 {
        i32::from(t.twist_r2s[self.twist() as usize])
    }

    pub(crate) fn cperm_sym(&self, t: &Tables) -> i32 {
        esym_to_csym(i32::from(t.eperm_r2s[self.cperm() as usize]))
    }

    pub(crate) fn eperm_sym(&self, t: &Tables) -> i32 {
        i32::from(t.eperm_r2s[self.eperm() as usize])
    }
}

impl Tables {
    pub(crate) fn perm_sym_inv(&self, idx: i32, sym: i32, is_corner: bool) -> i32 {
        let mut idxi = i32::from(self.perm_inv_edge_sym[idx as usize]);
        if is_corner {
            idxi = esym_to_csym(idxi);
        }
        idxi & 0xfff0 | self.sym.sym_mult[(idxi & 0xf) as usize][sym as usize] as i32
    }

    fn build() -> Self {
        let mut t = Self {
            sym: SymTables::build(),
            flip_s2r: vec![0; N_FLIP_SYM],
            twist_s2r: vec![0; N_TWIST_SYM],
            eperm_s2r: vec![0; N_PERM_SYM],
            perm2combp: vec![0; N_PERM_SYM],
            perm_inv_edge_sym: vec![0; N_PERM_SYM],
            mperm_inv: [0; N_MPERM],
            flip_r2s: vec![0; N_FLIP],
            twist_r2s: vec![0; N_TWIST],
            eperm_r2s: vec![0; N_PERM],
            flip_s2rf: vec![0; N_FLIP_SYM * 8],
            sym_state_twist: vec![0; N_TWIST_SYM],
            sym_state_flip: vec![0; N_FLIP_SYM],
            sym_state_perm: vec![0; N_PERM_SYM],
            ud_slice_move: vec![[0; N_MOVES]; N_SLICE],
            twist_move: vec![[0; N_MOVES]; N_TWIST_SYM],
            flip_move: vec![[0; N_MOVES]; N_FLIP_SYM],
            ud_slice_conj: vec![[0; 8]; N_SLICE],
            ud_slice_twist_prun: vec![0; N_SLICE * N_TWIST_SYM / 8 + 1],
            ud_slice_flip_prun: vec![0; N_SLICE * N_FLIP_SYM / 8 + 1],
            twist_flip_prun: vec![0; N_FLIP * N_TWIST_SYM / 8 + 1],
            cperm_move: vec![[0; N_MOVES2]; N_PERM_SYM],
            eperm_move: vec![[0; N_MOVES2]; N_PERM_SYM],
            mperm_move: vec![[0; N_MOVES2]; N_MPERM],
            mperm_conj: vec![[0; 16]; N_MPERM],
            ccombp_move: vec![[0; N_MOVES2]; N_COMB],
            ccombp_conj: vec![[0; 16]; N_COMB],
            mcperm_prun: vec![0; N_MPERM * N_PERM_SYM / 8 + 1],
            eperm_ccombp_prun: vec![0; N_COMB * N_PERM_SYM / 8 + 1],
        };
        t.init_perm_sym2raw();
        t.init_cperm_move();
        t.init_eperm_move();
        t.init_mperm_move_conj();
        t.init_combp_move_conj();
        t.init_sym2raw(RawCoord::Flip);
        t.init_sym2raw(RawCoord::Twist);
        t.init_flip_move();
        t.init_twist_move();
        t.init_ud_slice_move_conj();
        t.init_prunings();
        t
    }

    fn init_sym2raw(&mut self, coord: RawCoord) {
        let (n_raw, sym_inc, is_edge) = match coord {
            RawCoord::Flip => (N_FLIP, 2, true),
            RawCoord::Twist => (N_TWIST, 2, false),
            RawCoord::EPerm => (N_PERM, 1, true),
        };
        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        let mut count = 0_usize;
        for i in 0..n_raw {
            let raw2sym = match coord {
                RawCoord::Flip => &self.flip_r2s,
                RawCoord::Twist => &self.twist_r2s,
                RawCoord::EPerm => &self.eperm_r2s,
            };
            if raw2sym[i] != 0 {
                continue;
            }
            match coord {
                RawCoord::Flip => c.set_flip(i as i32),
                RawCoord::Twist => c.set_twist(i as i32),
                RawCoord::EPerm => c.set_eperm(i as i32),
            }
            for s in (0..16).step_by(sym_inc) {
                if is_edge {
                    CubieCube::edge_conjugate(&self.sym, &c, s, &mut d);
                } else {
                    CubieCube::corn_conjugate(&self.sym, &c, s, &mut d);
                }
                let idx = match coord {
                    RawCoord::Flip => d.flip(),
                    RawCoord::Twist => d.twist(),
                    RawCoord::EPerm => d.eperm(),
                } as usize;
                if coord == RawCoord::Flip {
                    self.flip_s2rf[count << 3 | s >> 1] = idx as u16;
                }
                let sym_state = match coord {
                    RawCoord::Flip => &mut self.sym_state_flip,
                    RawCoord::Twist => &mut self.sym_state_twist,
                    RawCoord::EPerm => &mut self.sym_state_perm,
                };
                if idx == i {
                    sym_state[count] |= 1 << (s / sym_inc);
                }
                let sym_idx = ((count << 4 | s) / sym_inc) as u16;
                match coord {
                    RawCoord::Flip => self.flip_r2s[idx] = sym_idx,
                    RawCoord::Twist => self.twist_r2s[idx] = sym_idx,
                    RawCoord::EPerm => self.eperm_r2s[idx] = sym_idx,
                }
            }
            let sym2raw = match coord {
                RawCoord::Flip => &mut self.flip_s2r,
                RawCoord::Twist => &mut self.twist_s2r,
                RawCoord::EPerm => &mut self.eperm_s2r,
            };
            sym2raw[count] = i as u16;
            count += 1;
        }
    }

    fn init_perm_sym2raw(&mut self) {
        self.init_sym2raw(RawCoord::EPerm);
        let mut cc = CubieCube::SOLVED;
        for i in 0..N_PERM_SYM {
            cc.set_eperm(i32::from(self.eperm_s2r[i]));
            // Java stores this sum in a byte, so values above 127 wrap; readers mask with 0xff.
            self.perm2combp[i] = (get_comb(&cc.ea, 0, true)
                + get_n_parity(i32::from(self.eperm_s2r[i]), 8) * 70)
                as u8;
            cc.inv_cubie_cube();
            self.perm_inv_edge_sym[i] = cc.eperm_sym(self) as u16;
        }
        for i in 0..N_MPERM {
            cc.set_mperm(i as i32);
            cc.inv_cubie_cube();
            self.mperm_inv[i] = cc.mperm() as u8;
        }
    }

    fn init_cperm_move(&mut self) {
        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        for i in 0..N_PERM_SYM {
            c.set_cperm(i32::from(self.eperm_s2r[i]));
            for j in 0..N_MOVES2 {
                CubieCube::corn_mult(&c, &self.sym.move_cube[UD2STD[j]], &mut d);
                self.cperm_move[i][j] = d.cperm_sym(self) as u16;
            }
        }
    }

    fn init_eperm_move(&mut self) {
        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        for i in 0..N_PERM_SYM {
            c.set_eperm(i32::from(self.eperm_s2r[i]));
            for j in 0..N_MOVES2 {
                CubieCube::edge_mult(&c, &self.sym.move_cube[UD2STD[j]], &mut d);
                self.eperm_move[i][j] = d.eperm_sym(self) as u16;
            }
        }
    }

    fn init_mperm_move_conj(&mut self) {
        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        for i in 0..N_MPERM {
            c.set_mperm(i as i32);
            for j in 0..N_MOVES2 {
                CubieCube::edge_mult(&c, &self.sym.move_cube[UD2STD[j]], &mut d);
                self.mperm_move[i][j] = d.mperm() as u16;
            }
            for j in 0..16 {
                CubieCube::edge_conjugate(&self.sym, &c, self.sym.sym_mult_inv[0][j], &mut d);
                self.mperm_conj[i][j] = d.mperm() as u16;
            }
        }
    }

    fn init_combp_move_conj(&mut self) {
        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        for i in 0..N_COMB {
            c.set_ccomb((i % 70) as i32);
            let parity_half = (i / 70) as i32;
            for j in 0..N_MOVES2 {
                CubieCube::corn_mult(&c, &self.sym.move_cube[UD2STD[j]], &mut d);
                self.ccombp_move[i][j] =
                    (d.ccomb() + 70 * ((P2_PARITY_MOVE >> j & 1) ^ parity_half)) as u16;
            }
            for j in 0..16 {
                CubieCube::corn_conjugate(&self.sym, &c, self.sym.sym_mult_inv[0][j], &mut d);
                self.ccombp_conj[i][j] = (d.ccomb() + 70 * parity_half) as u16;
            }
        }
    }

    fn init_flip_move(&mut self) {
        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        for i in 0..N_FLIP_SYM {
            c.set_flip(i32::from(self.flip_s2r[i]));
            for j in 0..N_MOVES {
                CubieCube::edge_mult(&c, &self.sym.move_cube[j], &mut d);
                self.flip_move[i][j] = d.flip_sym(self) as u16;
            }
        }
    }

    fn init_twist_move(&mut self) {
        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        for i in 0..N_TWIST_SYM {
            c.set_twist(i32::from(self.twist_s2r[i]));
            for j in 0..N_MOVES {
                CubieCube::corn_mult(&c, &self.sym.move_cube[j], &mut d);
                self.twist_move[i][j] = d.twist_sym(self) as u16;
            }
        }
    }

    fn init_ud_slice_move_conj(&mut self) {
        let mut c = CubieCube::SOLVED;
        let mut d = CubieCube::SOLVED;
        for i in 0..N_SLICE {
            c.set_ud_slice(i as i32);
            for j in (0..N_MOVES).step_by(3) {
                CubieCube::edge_mult(&c, &self.sym.move_cube[j], &mut d);
                self.ud_slice_move[i][j] = d.ud_slice() as u16;
            }
            for j in (0..16).step_by(2) {
                CubieCube::edge_conjugate(&self.sym, &c, self.sym.sym_mult_inv[0][j], &mut d);
                self.ud_slice_conj[i][j >> 1] = d.ud_slice() as u16;
            }
        }
        for i in 0..N_SLICE {
            for j in (0..N_MOVES).step_by(3) {
                let mut udslice = usize::from(self.ud_slice_move[i][j]);
                for k in 1..3 {
                    udslice = usize::from(self.ud_slice_move[udslice][j]);
                    self.ud_slice_move[i][j + k] = udslice as u16;
                }
            }
        }
    }

    /// Builds the five (independent) pruning tables, in parallel.
    fn init_prunings(&mut self) {
        let mut mcperm_prun = std::mem::take(&mut self.mcperm_prun);
        let mut eperm_ccombp_prun = std::mem::take(&mut self.eperm_ccombp_prun);
        let mut ud_slice_twist_prun = std::mem::take(&mut self.ud_slice_twist_prun);
        let mut ud_slice_flip_prun = std::mem::take(&mut self.ud_slice_flip_prun);
        let mut twist_flip_prun = std::mem::take(&mut self.twist_flip_prun);
        let t = &*self;
        parallel::join(
            || {
                parallel::join(
                    || t.init_twist_flip_prun(&mut twist_flip_prun),
                    || t.init_mcperm_prun(&mut mcperm_prun),
                )
            },
            || {
                parallel::join(
                    || t.init_eperm_ccombp_prun(&mut eperm_ccombp_prun),
                    || {
                        parallel::join(
                            || t.init_ud_slice_twist_prun(&mut ud_slice_twist_prun),
                            || t.init_ud_slice_flip_prun(&mut ud_slice_flip_prun),
                        )
                    },
                )
            },
        );
        self.mcperm_prun = mcperm_prun;
        self.eperm_ccombp_prun = eperm_ccombp_prun;
        self.ud_slice_twist_prun = ud_slice_twist_prun;
        self.ud_slice_flip_prun = ud_slice_flip_prun;
        self.twist_flip_prun = twist_flip_prun;
    }

    fn init_mcperm_prun(&self, prun: &mut [i32]) {
        init_raw_sym_prun(
            prun,
            Some(&RawTables {
                n_raw: N_MPERM,
                raw_move: &|r, m| usize::from(self.mperm_move[r][m]),
                raw_conj: &|r, s| usize::from(self.mperm_conj[r][s]),
            }),
            N_PERM_SYM,
            &|s, m| usize::from(self.cperm_move[s][m]),
            &self.sym_state_perm,
            self,
            0x8ea34,
        );
    }

    fn init_eperm_ccombp_prun(&self, prun: &mut [i32]) {
        init_raw_sym_prun(
            prun,
            Some(&RawTables {
                n_raw: N_COMB,
                raw_move: &|r, m| usize::from(self.ccombp_move[r][m]),
                raw_conj: &|r, s| usize::from(self.ccombp_conj[r][s]),
            }),
            N_PERM_SYM,
            &|s, m| usize::from(self.eperm_move[s][m]),
            &self.sym_state_perm,
            self,
            0x7d824,
        );
    }

    fn init_ud_slice_twist_prun(&self, prun: &mut [i32]) {
        init_raw_sym_prun(
            prun,
            Some(&RawTables {
                n_raw: N_SLICE,
                raw_move: &|r, m| usize::from(self.ud_slice_move[r][m]),
                raw_conj: &|r, s| usize::from(self.ud_slice_conj[r][s]),
            }),
            N_TWIST_SYM,
            &|s, m| usize::from(self.twist_move[s][m]),
            &self.sym_state_twist,
            self,
            0x69603,
        );
    }

    fn init_ud_slice_flip_prun(&self, prun: &mut [i32]) {
        init_raw_sym_prun(
            prun,
            Some(&RawTables {
                n_raw: N_SLICE,
                raw_move: &|r, m| usize::from(self.ud_slice_move[r][m]),
                raw_conj: &|r, s| usize::from(self.ud_slice_conj[r][s]),
            }),
            N_FLIP_SYM,
            &|s, m| usize::from(self.flip_move[s][m]),
            &self.sym_state_flip,
            self,
            0x69603,
        );
    }

    fn init_twist_flip_prun(&self, prun: &mut [i32]) {
        init_raw_sym_prun(
            prun,
            None,
            N_TWIST_SYM,
            &|s, m| usize::from(self.twist_move[s][m]),
            &self.sym_state_twist,
            self,
            0x19603,
        );
    }
}

/// The raw coordinate of a raw x symmetry pruning table. `None` means the twist-flip
/// table, whose raw coordinate (the flip) is itself handled through symmetry tables.
struct RawTables<'a> {
    n_raw: usize,
    raw_move: &'a dyn Fn(usize, usize) -> usize,
    raw_conj: &'a dyn Fn(usize, usize) -> usize,
}

/// `CoordCube.initRawSymPrun` with `fullInit == true`.
///
/// `prun_flag` packs `MIN_DEPTH | MAX_DEPTH | INV_DEPTH | padding | P2 | E2C | SYM_SHIFT`.
fn init_raw_sym_prun(
    prun_table: &mut [i32],
    raw: Option<&RawTables<'_>>,
    n_sym: usize,
    sym_move: &dyn Fn(usize, usize) -> usize,
    sym_state: &[u16],
    t: &Tables,
    prun_flag: i32,
) {
    let sym_shift = prun_flag & 0xf;
    let sym_e2c_magic = if (prun_flag >> 4) & 1 == 1 {
        SYM_E2C_MAGIC
    } else {
        0
    };
    let is_phase2 = (prun_flag >> 5) & 1 == 1;
    let inv_depth = prun_flag >> 8 & 0xf;
    let search_depth = prun_flag >> 12 & 0xf;

    let sym_mask = (1 << sym_shift) - 1;
    let n_raw = raw.map_or(N_FLIP, |r| r.n_raw);
    let n_size = n_raw * n_sym;
    let n_moves = if is_phase2 { 10 } else { 18 };
    let next_axis_magic = if n_moves == 10 { 0x42 } else { 0x92492 };

    let mut depth = get_pruning(prun_table, n_size) - 1;
    if depth == -1 {
        for v in prun_table.iter_mut().take(n_size / 8 + 1) {
            *v = 0x1111_1111;
        }
        set_pruning(prun_table, 0, 1);
        depth = 0;
    }

    while depth < search_depth {
        let mask = (depth + 1).wrapping_mul(0x1111_1111) ^ -1;
        for v in prun_table.iter_mut() {
            let mut val = *v ^ mask;
            val &= val >> 1;
            *v = v.wrapping_add(val & (val >> 2) & 0x1111_1111);
        }

        let inv = depth > inv_depth;
        let select = if inv { depth + 2 } else { depth };
        let sel_arr_mask = select.wrapping_mul(0x1111_1111);
        let check = if inv { depth } else { depth + 2 };
        depth += 1;
        let xor_val = depth ^ (depth + 1);
        let mut val = 0_i32;
        let mut i = 0;
        while i < n_size {
            'entry: {
                if i.trailing_zeros() >= 3 {
                    val = prun_table[i >> 3];
                    if !has_zero(val ^ sel_arr_mask) {
                        i += 7;
                        break 'entry;
                    }
                }
                if val & 0xf != select {
                    break 'entry;
                }
                let raw_idx = i % n_raw;
                let sym = i / n_raw;
                let (mut flip, mut fsym) = (0, 0);
                if raw.is_none() {
                    flip = usize::from(t.flip_r2s[raw_idx]);
                    fsym = flip & 7;
                    flip >>= 3;
                }
                let mut m = 0;
                while m < n_moves {
                    let mut symx = sym_move(sym, m);
                    let rawx = match raw {
                        None => usize::from(
                            t.flip_s2rf[usize::from(
                                t.flip_move[flip][t.sym.sym8_move[m << 3 | fsym]],
                            ) ^ fsym
                                ^ (symx & sym_mask)],
                        ),
                        Some(r) => (r.raw_conj)((r.raw_move)(raw_idx, m), symx & sym_mask),
                    };
                    symx >>= sym_shift;
                    let idx = symx * n_raw + rawx;
                    let prun = get_pruning(prun_table, idx);
                    if prun != check {
                        if prun < depth - 1 {
                            m += (next_axis_magic >> m & 3) as usize;
                        }
                        m += 1;
                        continue;
                    }
                    if inv {
                        set_pruning(prun_table, i, xor_val);
                        break;
                    }
                    set_pruning(prun_table, idx, xor_val);
                    let mut j = 1;
                    let mut state = i32::from(sym_state[symx]);
                    loop {
                        state >>= 1;
                        if state == 0 {
                            break;
                        }
                        if state & 1 == 1 {
                            let mut idxx = symx * n_raw;
                            idxx += match raw {
                                None => usize::from(t.flip_s2rf[usize::from(t.flip_r2s[rawx]) ^ j]),
                                Some(r) => {
                                    (r.raw_conj)(rawx, j ^ (sym_e2c_magic >> (j << 1) & 3) as usize)
                                }
                            };
                            if get_pruning(prun_table, idxx) == check {
                                set_pruning(prun_table, idxx, xor_val);
                            }
                        }
                        j += 1;
                    }
                    m += 1;
                }
            }
            i += 1;
            val >>= 4;
        }
    }
}

/// The phase 1 coordinates of a node in the search tree (`CoordCube` instances).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CoordCube {
    pub(crate) twist: i32,
    pub(crate) tsym: i32,
    pub(crate) flip: i32,
    pub(crate) fsym: i32,
    pub(crate) slice: i32,
    pub(crate) prun: i32,
    pub(crate) twistc: i32,
    pub(crate) flipc: i32,
}

impl CoordCube {
    pub(crate) fn calc_pruning(&mut self, t: &Tables) {
        self.prun = get_pruning(
            &t.ud_slice_twist_prun,
            self.twist as usize * N_SLICE
                + usize::from(t.ud_slice_conj[self.slice as usize][self.tsym as usize]),
        )
        .max(get_pruning(
            &t.ud_slice_flip_prun,
            self.flip as usize * N_SLICE
                + usize::from(t.ud_slice_conj[self.slice as usize][self.fsym as usize]),
        ))
        .max(
            get_pruning(
                &t.twist_flip_prun,
                ((self.twistc >> 3) << 11) as usize
                    | usize::from(t.flip_s2rf[(self.flipc ^ (self.twistc & 7)) as usize]),
            )
            .max(get_pruning(
                &t.twist_flip_prun,
                (self.twist << 11) as usize
                    | usize::from(t.flip_s2rf[(self.flip << 3 | (self.fsym ^ self.tsym)) as usize]),
            )),
        );
    }

    /// Sets the coordinates from `cc`; returns whether the node can be solved within `depth`
    /// phase 1 moves according to the pruning tables.
    pub(crate) fn set_with_prun(&mut self, t: &Tables, cc: &CubieCube, depth: i32) -> bool {
        self.twist = cc.twist_sym(t);
        self.flip = cc.flip_sym(t);
        self.tsym = self.twist & 7;
        self.twist >>= 3;

        self.prun = get_pruning(
            &t.twist_flip_prun,
            (self.twist << 11) as usize
                | usize::from(t.flip_s2rf[(self.flip ^ self.tsym) as usize]),
        );
        if self.prun > depth {
            return false;
        }

        self.fsym = self.flip & 7;
        self.flip >>= 3;

        self.slice = cc.ud_slice();
        self.prun = self.prun.max(
            get_pruning(
                &t.ud_slice_twist_prun,
                self.twist as usize * N_SLICE
                    + usize::from(t.ud_slice_conj[self.slice as usize][self.tsym as usize]),
            )
            .max(get_pruning(
                &t.ud_slice_flip_prun,
                self.flip as usize * N_SLICE
                    + usize::from(t.ud_slice_conj[self.slice as usize][self.fsym as usize]),
            )),
        );
        if self.prun > depth {
            return false;
        }

        let mut pc = CubieCube::SOLVED;
        CubieCube::corn_conjugate(&t.sym, cc, 1, &mut pc);
        CubieCube::edge_conjugate(&t.sym, cc, 1, &mut pc);
        self.twistc = pc.twist_sym(t);
        self.flipc = pc.flip_sym(t);
        self.prun = self.prun.max(get_pruning(
            &t.twist_flip_prun,
            ((self.twistc >> 3) << 11) as usize
                | usize::from(t.flip_s2rf[(self.flipc ^ (self.twistc & 7)) as usize]),
        ));

        self.prun <= depth
    }

    /// Applies move `m` to `cc`, storing the result in `self`; returns the new pruning value.
    pub(crate) fn do_move_prun(&mut self, t: &Tables, cc: &Self, m: usize) -> i32 {
        self.slice = i32::from(t.ud_slice_move[cc.slice as usize][m]);

        self.flip =
            i32::from(t.flip_move[cc.flip as usize][t.sym.sym8_move[m << 3 | cc.fsym as usize]]);
        self.fsym = (self.flip & 7) ^ cc.fsym;
        self.flip >>= 3;

        self.twist =
            i32::from(t.twist_move[cc.twist as usize][t.sym.sym8_move[m << 3 | cc.tsym as usize]]);
        self.tsym = (self.twist & 7) ^ cc.tsym;
        self.twist >>= 3;

        self.prun = get_pruning(
            &t.ud_slice_twist_prun,
            self.twist as usize * N_SLICE
                + usize::from(t.ud_slice_conj[self.slice as usize][self.tsym as usize]),
        )
        .max(get_pruning(
            &t.ud_slice_flip_prun,
            self.flip as usize * N_SLICE
                + usize::from(t.ud_slice_conj[self.slice as usize][self.fsym as usize]),
        ))
        .max(get_pruning(
            &t.twist_flip_prun,
            (self.twist << 11) as usize
                | usize::from(t.flip_s2rf[(self.flip << 3 | (self.fsym ^ self.tsym)) as usize]),
        ));
        self.prun
    }

    /// Applies move `m` to the conjugated coordinates of `cc`; returns their pruning value.
    pub(crate) fn do_move_prun_conj(&mut self, t: &Tables, cc: &Self, m: usize) -> i32 {
        let m = t.sym.sym_move[3][m];
        self.flipc = i32::from(
            t.flip_move[(cc.flipc >> 3) as usize]
                [t.sym.sym8_move[m << 3 | (cc.flipc & 7) as usize]],
        ) ^ (cc.flipc & 7);
        self.twistc = i32::from(
            t.twist_move[(cc.twistc >> 3) as usize]
                [t.sym.sym8_move[m << 3 | (cc.twistc & 7) as usize]],
        ) ^ (cc.twistc & 7);
        get_pruning(
            &t.twist_flip_prun,
            ((self.twistc >> 3) << 11) as usize
                | usize::from(t.flip_s2rf[(self.flipc ^ (self.twistc & 7)) as usize]),
        )
    }
}
