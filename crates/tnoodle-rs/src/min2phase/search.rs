//! The two-phase search (`cs.min2phase.Search` and `SearchWCA`).

use std::fmt::Write as _;
use std::time::Instant;

use super::coord_cube::{CoordCube, N_COMB, N_MPERM, TABLES, Tables, get_pruning};
use super::cubie_cube::{CubieCube, URF_MOVE};
use super::util::{self, MOVE2STR, UD2STD, UTIL};

/// Insert `".  "` (or `"."` for the WCA searcher) between the phase 1 and phase 2 moves.
pub const USE_SEPARATOR: i32 = 0x1;
/// Return the inverse of the solution, i.e. a scramble that generates the input state.
pub const INVERSE_SOLUTION: i32 = 0x2;
/// Append the solution length, e.g. `(21f)`.
pub const APPEND_LENGTH: i32 = 0x4;
/// Search for an optimal solution.
pub const OPTIMAL_SOLUTION: i32 = 0x8;

const MAX_PRE_MOVES: i32 = 20;
const MIN_P1LENGTH_PRE: i32 = 7;
const MAX_DEPTH2: i32 = 12;

/// The move names used by the WCA searcher (no padding).
const WCA_MOVE2STR: [&str; 18] = [
    "U", "U2", "U'", "R", "R2", "R'", "F", "F2", "F'", "D", "D2", "D'", "L", "L2", "L'", "B", "B2",
    "B'",
];

/// The extra state of `SearchWCA`: axis restrictions and a wall-clock time budget.
#[derive(Debug, Clone)]
struct WcaState {
    first_move_filter: [i32; 6],
    last_move_filter: [i32; 6],
    is_axis_restricted: bool,
    start_time: Instant,
}

/// Kociemba's two-phase algorithm, as implemented by Chen Shuang's min2phase.
///
/// One searcher can be reused for any number of solves; the lookup tables are shared by
/// every searcher and built on first use.
///
/// ```
/// use tnoodle::min2phase::{Search, tools};
///
/// let facelets = tools::from_scramble("R U R' U'");
/// let solution = Search::new().solution(&facelets, 21, 100_000, 0, 0);
/// // Like min2phase, moves are padded to two characters and followed by a space.
/// assert_eq!(solution, "U  R  U' R' ");
/// ```
#[derive(Clone)]
pub struct Search {
    t: &'static Tables,
    moves: [i32; 31],
    move_sol: [i32; 31],
    node_ud: [CoordCube; 21],
    node_rl: [CoordCube; 21],
    node_fb: [CoordCube; 21],
    self_sym: i64,
    conj_mask: i32,
    urf_idx: usize,
    length1: i32,
    depth1: i32,
    max_dep2: i32,
    sol: i32,
    solution: Option<String>,
    probe: i64,
    probe_max: i64,
    probe_min: i64,
    verbose: i32,
    valid1: i32,
    allow_shorter: bool,
    cc: CubieCube,
    urf_cubie_cube: [CubieCube; 6],
    urf_coord_cube: [CoordCube; 6],
    phase1_cubie: [CubieCube; 21],
    pre_moves: [i32; MAX_PRE_MOVES as usize],
    pre_move_len: i32,
    max_pre_moves: i32,
    is_rec: bool,
    wca: Option<WcaState>,
}

impl std::fmt::Debug for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Search")
            .field("solution", &self.solution)
            .field("length", &self.sol)
            .field("probes", &self.probe)
            .field("wca", &self.wca.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

impl Search {
    /// A searcher with min2phase's plain behaviour: `probe_max` and `probe_min` count
    /// phase 2 probes.
    pub fn new() -> Self {
        Self {
            t: &TABLES,
            moves: [0; 31],
            move_sol: [0; 31],
            node_ud: [CoordCube::default(); 21],
            node_rl: [CoordCube::default(); 21],
            node_fb: [CoordCube::default(); 21],
            self_sym: 0,
            conj_mask: 0,
            urf_idx: 0,
            length1: 0,
            depth1: 0,
            max_dep2: 0,
            sol: 0,
            solution: None,
            probe: 0,
            probe_max: 0,
            probe_min: 0,
            verbose: 0,
            valid1: 0,
            allow_shorter: false,
            cc: CubieCube::SOLVED,
            urf_cubie_cube: [CubieCube::SOLVED; 6],
            urf_coord_cube: [CoordCube::default(); 6],
            phase1_cubie: [CubieCube::SOLVED; 21],
            pre_moves: [0; MAX_PRE_MOVES as usize],
            pre_move_len: 0,
            max_pre_moves: 0,
            is_rec: false,
            wca: None,
        }
    }

    /// Builds the lookup tables now instead of on the first solve.
    pub fn init() {
        let _ = &*TABLES;
    }

    /// Solves a cube given as 54 facelets in `URFDLB` order (see [`tools`](super::tools)).
    ///
    /// * `max_depth`: the maximum solution length.
    /// * `probe_max`: give up (`Error 8`) after this many phase 2 probes without a solution.
    /// * `probe_min`: after finding a solution, keep looking for shorter ones for at least
    ///   this many probes.
    /// * `verbose`: a combination of [`USE_SEPARATOR`], [`INVERSE_SOLUTION`],
    ///   [`APPEND_LENGTH`] and [`OPTIMAL_SOLUTION`].
    ///
    /// Returns the solution (each move followed by a space) or an error code:
    /// `Error 1` wrong number of facelets of a colour, `Error 2` missing edges, `Error 3`
    /// flipped edge, `Error 4` missing corners, `Error 5` twisted corner, `Error 6` parity
    /// error, `Error 7` no solution within `max_depth`, `Error 8` probe limit exceeded.
    pub fn solution(
        &mut self,
        facelets: &str,
        max_depth: i32,
        probe_max: i64,
        probe_min: i64,
        verbose: i32,
    ) -> String {
        let check = self.verify(facelets);
        if check != 0 {
            return format!("Error {}", check.abs());
        }
        self.sol = max_depth + 1;
        self.probe = 0;
        self.probe_max = probe_max;
        self.probe_min = probe_min.min(probe_max);
        self.verbose = verbose;
        self.solution = None;
        self.is_rec = false;

        self.init_search();

        if verbose & OPTIMAL_SOLUTION == 0 {
            self.search()
        } else {
            self.search_opt()
        }
    }

    /// Continues the previous search, looking for the next (shorter) solution.
    pub fn next(&mut self, probe_max: i64, probe_min: i64, verbose: i32) -> String {
        self.probe = 0;
        self.probe_max = probe_max;
        self.probe_min = probe_min.min(probe_max);
        self.solution = None;
        self.is_rec = (self.verbose & OPTIMAL_SOLUTION) == (verbose & OPTIMAL_SOLUTION);
        self.verbose = verbose;
        if verbose & OPTIMAL_SOLUTION == 0 {
            self.search()
        } else {
            self.search_opt()
        }
    }

    /// The number of phase 2 probes used by the last search.
    pub fn number_of_probes(&self) -> i64 {
        self.probe
    }

    /// The length of the last solution found.
    pub fn length(&self) -> i32 {
        self.sol
    }

    /// Checks a facelet string, returning `0` if it describes a solvable cube or the negated
    /// error code (see [`solution`](Self::solution)).
    pub fn verify(&mut self, facelets: &str) -> i32 {
        let chars: Vec<u16> = facelets.encode_utf16().collect();
        if chars.len() < 54 {
            return -1;
        }
        let center = [
            chars[usize::from(util::U5)],
            chars[usize::from(util::R5)],
            chars[usize::from(util::F5)],
            chars[usize::from(util::D5)],
            chars[usize::from(util::L5)],
            chars[usize::from(util::B5)],
        ];
        let mut count = 0;
        let mut f = [0_u8; 54];
        for (i, slot) in f.iter_mut().enumerate() {
            let Some(color) = center.iter().position(|&c| c == chars[i]) else {
                return -1;
            };
            *slot = color as u8;
            count += 1 << (color << 2);
        }
        if count != 0x0099_9999 {
            return -1;
        }
        util::to_cubie_cube(&f, &mut self.cc);
        self.cc.verify()
    }

    /// The edge and corner permutation parities of the cube parsed by the last
    /// [`verify`](Self::verify).
    pub(crate) fn last_cube_parities(&self) -> (i32, i32) {
        (self.cc.edge_parity_bit(), self.cc.corner_parity_bit())
    }

    fn init_search(&mut self) {
        let t = self.t;
        self.conj_mask = 0;
        self.self_sym = self.cc.self_symmetry(&t.sym);
        if (self.self_sym >> 16 & 0xffff) != 0 {
            self.conj_mask |= 0x12;
        }
        if (self.self_sym >> 32 & 0xffff) != 0 {
            self.conj_mask |= 0x24;
        }
        if (self.self_sym >> 48 & 0xffff) != 0 {
            self.conj_mask |= 0x38;
        }
        self.self_sym &= 0xffff_ffff_ffff;
        self.max_pre_moves = if self.conj_mask > 7 { 0 } else { MAX_PRE_MOVES };

        for i in 0..6 {
            self.urf_cubie_cube[i] = self.cc;
            self.urf_coord_cube[i].set_with_prun(t, &self.urf_cubie_cube[i], 20);
            self.cc.urf_conjugate(&t.sym);
            if i % 3 == 2 {
                self.cc.inv_cubie_cube();
            }
        }

        if self.wca.as_ref().is_some_and(|w| w.is_axis_restricted) {
            self.self_sym = 1;
            self.conj_mask = 0;
            self.max_pre_moves = MAX_PRE_MOVES;
        }
    }

    fn phase1_pre_moves(&mut self, maxl: i32, lm: i32, cc: CubieCube, ssym: i32) -> i32 {
        if let Some(w) = &self.wca
            && maxl == self.max_pre_moves - 1
            && (w.last_move_filter[self.urf_idx] >> lm & 1) != 0
        {
            return 1;
        }

        let t = self.t;
        self.pre_move_len = self.max_pre_moves - maxl;
        let try_phase1 = if self.is_rec {
            self.depth1 == self.length1 - self.pre_move_len
        } else {
            self.pre_move_len == 0 || (0x36FB7 >> lm & 1) == 0
        };
        if try_phase1 {
            self.depth1 = self.length1 - self.pre_move_len;
            self.phase1_cubie[0] = cc;
            self.allow_shorter = self.depth1 == MIN_P1LENGTH_PRE && self.pre_move_len != 0;
            let d = self.depth1 as usize;
            if self.node_ud[d + 1].set_with_prun(t, &cc, self.depth1)
                && self.phase1(d + 1, ssym, self.depth1, -1) == 0
            {
                return 0;
            }
        }

        if maxl == 0 || self.pre_move_len + MIN_P1LENGTH_PRE >= self.length1 {
            return 1;
        }

        let mut skip_moves = t.sym.skip_moves(i64::from(ssym));
        if maxl == 1 || self.pre_move_len + 1 + MIN_P1LENGTH_PRE >= self.length1 {
            skip_moves |= 0x36FB7;
        }

        let lm = lm / 3 * 3;
        let mut m = 0;
        while m < 18 {
            if m == lm || m == lm - 9 || m == lm + 9 {
                m += 3;
                continue;
            }
            if self.is_rec && m != self.pre_moves[(self.max_pre_moves - maxl) as usize]
                || (skip_moves & 1 << m) != 0
            {
                m += 1;
                continue;
            }
            let mut next = CubieCube::SOLVED;
            CubieCube::mult(&t.sym.move_cube[m as usize], &cc, &mut next);
            self.pre_moves[(self.max_pre_moves - maxl) as usize] = m;
            let ret = self.phase1_pre_moves(
                maxl - 1,
                m,
                next,
                ssym & t.sym.move_cube_sym[m as usize] as i32,
            );
            if ret == 0 {
                return 0;
            }
            m += 1;
        }
        1
    }

    fn search(&mut self) -> String {
        if !self.is_rec {
            self.length1 = 0;
        }
        while self.length1 < self.sol {
            self.max_dep2 = MAX_DEPTH2.min(self.sol - self.length1 - 1);
            if !self.is_rec {
                self.urf_idx = 0;
            }
            while self.urf_idx < 6 {
                if (self.conj_mask & 1 << self.urf_idx) == 0 {
                    let cube = self.urf_cubie_cube[self.urf_idx];
                    let ssym = (self.self_sym & 0xffff) as i32;
                    if self.phase1_pre_moves(self.max_pre_moves, -30, cube, ssym) == 0 {
                        return self
                            .solution
                            .clone()
                            .unwrap_or_else(|| "Error 8".to_owned());
                    }
                }
                self.urf_idx += 1;
            }
            self.length1 += 1;
        }
        self.solution
            .clone()
            .unwrap_or_else(|| "Error 7".to_owned())
    }

    /// Returns `0` if a solution was found or the probe limit exceeded, `1` to try the next
    /// power and `2` to try the next axis.
    fn init_phase2_pre(&mut self) -> i32 {
        self.is_rec = false;
        if let Some(w) = &self.wca {
            let elapsed = w.start_time.elapsed().as_millis() as i64;
            if elapsed
                >= if self.solution.is_none() {
                    self.probe_max
                } else {
                    self.probe_min
                }
            {
                return 0;
            }
            self.probe = -1;
        }
        if self.probe
            >= if self.solution.is_none() {
                self.probe_max
            } else {
                self.probe_min
            }
        {
            return 0;
        }
        self.probe += 1;

        let t = self.t;
        for i in self.valid1..self.depth1 {
            let i = i as usize;
            let (lo, hi) = self.phase1_cubie.split_at_mut(i + 1);
            CubieCube::mult(&lo[i], &t.sym.move_cube[self.moves[i] as usize], &mut hi[0]);
        }
        self.valid1 = self.depth1;

        let p1 = self.phase1_cubie[self.depth1 as usize];
        let mut p2corn = p1.cperm_sym(t);
        let mut p2csym = p2corn & 0xf;
        p2corn >>= 4;
        let mut p2edge = p1.eperm_sym(t);
        let mut p2esym = p2edge & 0xf;
        p2edge >>= 4;
        let mut p2mid = p1.mperm();
        let mut edgei = t.perm_sym_inv(p2edge, p2esym, false);
        let mut corni = t.perm_sym_inv(p2corn, p2csym, true);

        let last_move = if self.depth1 == 0 {
            -1
        } else {
            self.moves[(self.depth1 - 1) as usize]
        };
        let last_pre = if self.pre_move_len == 0 {
            -1
        } else {
            self.pre_moves[(self.pre_move_len - 1) as usize]
        };

        let std2ud = &UTIL.std2ud;
        let mut ret = 0;
        let p2switch_max =
            (if self.pre_move_len == 0 { 1 } else { 2 }) * (if self.depth1 == 0 { 1 } else { 2 });
        let mut p2switch_mask = (1 << p2switch_max) - 1;
        for p2switch in 0..p2switch_max {
            // 0 normal; 1 lastmove; 2 lastmove + premove; 3 premove
            if (p2switch_mask >> p2switch & 1) != 0 {
                p2switch_mask &= !(1 << p2switch);
                ret = self.init_phase2(p2corn, p2csym, p2edge, p2esym, p2mid, edgei, corni);
                if ret == 0 || ret > 2 {
                    break;
                } else if ret == 2 {
                    p2switch_mask &= 0x4 << p2switch;
                }
            }
            if p2switch_mask == 0 {
                break;
            }
            if (p2switch & 1) == 0 && self.depth1 > 0 {
                let m = std2ud[(last_move / 3 * 3 + 1) as usize];
                let d = (self.depth1 - 1) as usize;
                self.moves[d] = UD2STD[m] as i32 * 2 - self.moves[d];

                p2mid = i32::from(t.mperm_move[p2mid as usize][m]);
                p2corn =
                    i32::from(t.cperm_move[p2corn as usize][t.sym.sym_move_ud[p2csym as usize][m]]);
                p2csym = t.sym.sym_mult[(p2corn & 0xf) as usize][p2csym as usize] as i32;
                p2corn >>= 4;
                p2edge =
                    i32::from(t.eperm_move[p2edge as usize][t.sym.sym_move_ud[p2esym as usize][m]]);
                p2esym = t.sym.sym_mult[(p2edge & 0xf) as usize][p2esym as usize] as i32;
                p2edge >>= 4;
                corni = t.perm_sym_inv(p2corn, p2csym, true);
                edgei = t.perm_sym_inv(p2edge, p2esym, false);
            } else if self.pre_move_len > 0 {
                let m = std2ud[(last_pre / 3 * 3 + 1) as usize];
                let d = (self.pre_move_len - 1) as usize;
                self.pre_moves[d] = UD2STD[m] as i32 * 2 - self.pre_moves[d];

                p2mid = i32::from(
                    t.mperm_inv
                        [usize::from(t.mperm_move[usize::from(t.mperm_inv[p2mid as usize])][m])],
                );
                p2corn = i32::from(
                    t.cperm_move[(corni >> 4) as usize]
                        [t.sym.sym_move_ud[(corni & 0xf) as usize][m]],
                );
                corni = p2corn & !0xf
                    | t.sym.sym_mult[(p2corn & 0xf) as usize][(corni & 0xf) as usize] as i32;
                p2corn = t.perm_sym_inv(corni >> 4, corni & 0xf, true);
                p2csym = p2corn & 0xf;
                p2corn >>= 4;
                p2edge = i32::from(
                    t.eperm_move[(edgei >> 4) as usize]
                        [t.sym.sym_move_ud[(edgei & 0xf) as usize][m]],
                );
                edgei = p2edge & !0xf
                    | t.sym.sym_mult[(p2edge & 0xf) as usize][(edgei & 0xf) as usize] as i32;
                p2edge = t.perm_sym_inv(edgei >> 4, edgei & 0xf, false);
                p2esym = p2edge & 0xf;
                p2edge >>= 4;
            }
        }
        if self.depth1 > 0 {
            self.moves[(self.depth1 - 1) as usize] = last_move;
        }
        if self.pre_move_len > 0 {
            self.pre_moves[(self.pre_move_len - 1) as usize] = last_pre;
        }
        i32::from(ret != 0) * 2
    }

    fn init_phase2(
        &mut self,
        p2corn: i32,
        p2csym: i32,
        p2edge: i32,
        p2esym: i32,
        p2mid: i32,
        edgei: i32,
        corni: i32,
    ) -> i32 {
        let t = self.t;
        let prun = get_pruning(
            &t.eperm_ccombp_prun,
            (edgei >> 4) as usize * N_COMB
                + usize::from(
                    t.ccombp_conj[usize::from(t.perm2combp[(corni >> 4) as usize])]
                        [t.sym.sym_mult_inv[(edgei & 0xf) as usize][(corni & 0xf) as usize]],
                ),
        )
        .max(
            get_pruning(
                &t.eperm_ccombp_prun,
                p2edge as usize * N_COMB
                    + usize::from(
                        t.ccombp_conj[usize::from(t.perm2combp[p2corn as usize])]
                            [t.sym.sym_mult_inv[p2esym as usize][p2csym as usize]],
                    ),
            )
            .max(get_pruning(
                &t.mcperm_prun,
                p2corn as usize * N_MPERM
                    + usize::from(t.mperm_conj[p2mid as usize][p2csym as usize]),
            )),
        );

        if prun > self.max_dep2 {
            return prun - self.max_dep2;
        }

        let mut depth2 = self.max_dep2;
        while depth2 >= prun {
            let ret = self.phase2(
                p2edge,
                p2esym,
                p2corn,
                p2csym,
                p2mid,
                depth2,
                self.depth1,
                10,
            );
            if ret < 0 {
                break;
            }
            depth2 -= ret;
            self.sol = 0;
            for i in 0..(self.depth1 + depth2) as usize {
                self.append_sol_move(self.moves[i]);
            }
            for i in (0..self.pre_move_len as usize).rev() {
                self.append_sol_move(self.pre_moves[i]);
            }
            self.solution = Some(self.solution_to_string());
            depth2 -= 1;
        }

        if depth2 != self.max_dep2 {
            // At least one solution has been found.
            self.max_dep2 = MAX_DEPTH2.min(self.sol - self.length1 - 1);
            return i32::from(self.probe < self.probe_min);
        }
        1
    }

    /// Returns `0` if a solution was found or the probe limit exceeded, `1` to try the next
    /// power and `2` or more to try the next axis. `node` indexes `node_ud`.
    fn phase1(&mut self, node: usize, ssym: i32, maxl: i32, lm: i32) -> i32 {
        if let Some(w) = &self.wca
            && maxl == self.depth1 - 1
            && (w.first_move_filter[self.urf_idx] >> lm & 1) != 0
        {
            return 1;
        }

        let t = self.t;
        let node_cube = self.node_ud[node];
        if node_cube.prun == 0 && maxl < 5 {
            if self.allow_shorter || maxl == 0 {
                self.depth1 -= maxl;
                let ret = self.init_phase2_pre();
                self.depth1 += maxl;
                return ret;
            }
            return 1;
        }

        let skip_moves = t.sym.skip_moves(i64::from(ssym));
        let ml = maxl as usize;
        let mut axis = 0;
        while axis < 18 {
            if axis == lm || axis == lm - 9 {
                axis += 3;
                continue;
            }
            for power in 0..3 {
                let m = axis + power;
                if self.is_rec && m != self.moves[(self.depth1 - maxl) as usize]
                    || skip_moves != 0 && (skip_moves & 1 << m) != 0
                {
                    continue;
                }

                let prun = self.node_ud[ml].do_move_prun(t, &node_cube, m as usize);
                if prun > maxl {
                    break;
                } else if prun == maxl {
                    continue;
                }
                let prun = self.node_ud[ml].do_move_prun_conj(t, &node_cube, m as usize);
                if prun > maxl {
                    break;
                } else if prun == maxl {
                    continue;
                }

                self.moves[(self.depth1 - maxl) as usize] = m;
                self.valid1 = self.valid1.min(self.depth1 - maxl);
                let ret = self.phase1(
                    ml,
                    ssym & t.sym.move_cube_sym[m as usize] as i32,
                    maxl - 1,
                    axis,
                );
                if ret == 0 {
                    return 0;
                } else if ret >= 2 {
                    break;
                }
            }
            axis += 3;
        }
        1
    }

    fn search_opt(&mut self) -> String {
        let t = self.t;
        let mut maxprun1 = 0;
        let mut maxprun2 = 0;
        for i in 0..6 {
            self.urf_coord_cube[i].calc_pruning(t);
            if i < 3 {
                maxprun1 = maxprun1.max(self.urf_coord_cube[i].prun);
            } else {
                maxprun2 = maxprun2.max(self.urf_coord_cube[i].prun);
            }
        }
        self.urf_idx = if maxprun2 > maxprun1 { 3 } else { 0 };
        self.phase1_cubie[0] = self.urf_cubie_cube[self.urf_idx];
        if !self.is_rec {
            self.length1 = 0;
        }
        while self.length1 < self.sol {
            let ud = self.urf_coord_cube[self.urf_idx];
            let rl = self.urf_coord_cube[1 + self.urf_idx];
            let fb = self.urf_coord_cube[2 + self.urf_idx];
            if ud.prun <= self.length1
                && rl.prun <= self.length1
                && fb.prun <= self.length1
                && self.phase1_opt(ud, rl, fb, self.self_sym, self.length1, -1) == 0
            {
                return self
                    .solution
                    .clone()
                    .unwrap_or_else(|| "Error 8".to_owned());
            }
            self.length1 += 1;
        }
        self.solution
            .clone()
            .unwrap_or_else(|| "Error 7".to_owned())
    }

    fn phase1_opt(
        &mut self,
        ud: CoordCube,
        rl: CoordCube,
        fb: CoordCube,
        ssym: i64,
        maxl: i32,
        lm: i32,
    ) -> i32 {
        if ud.prun == 0 && rl.prun == 0 && fb.prun == 0 && maxl < 5 {
            self.max_dep2 = maxl;
            self.depth1 = self.length1 - maxl;
            return i32::from(self.init_phase2_pre() != 0);
        }

        let t = self.t;
        let skip_moves = t.sym.skip_moves(ssym);
        let ml = maxl as usize;
        let mut axis = 0;
        while axis < 18 {
            if axis == lm || axis == lm - 9 {
                axis += 3;
                continue;
            }
            for power in 0..3 {
                let mut m = (axis + power) as usize;
                if self.is_rec && m as i32 != self.moves[(self.length1 - maxl) as usize]
                    || skip_moves != 0 && (skip_moves & 1 << m) != 0
                {
                    continue;
                }

                // UD axis
                let prun_ud = self.node_ud[ml]
                    .do_move_prun(t, &ud, m)
                    .max(self.node_ud[ml].do_move_prun_conj(t, &ud, m));
                if prun_ud > maxl {
                    break;
                } else if prun_ud == maxl {
                    continue;
                }

                // RL axis
                m = URF_MOVE[2][m];
                let prun_rl = self.node_rl[ml]
                    .do_move_prun(t, &rl, m)
                    .max(self.node_rl[ml].do_move_prun_conj(t, &rl, m));
                if prun_rl > maxl {
                    break;
                } else if prun_rl == maxl {
                    continue;
                }

                // FB axis
                m = URF_MOVE[2][m];
                let mut prun_fb = self.node_fb[ml]
                    .do_move_prun(t, &fb, m)
                    .max(self.node_fb[ml].do_move_prun_conj(t, &fb, m));
                if prun_ud == prun_rl && prun_rl == prun_fb && prun_fb != 0 {
                    prun_fb += 1;
                }
                if prun_fb > maxl {
                    break;
                } else if prun_fb == maxl {
                    continue;
                }

                m = URF_MOVE[2][m];
                self.moves[(self.length1 - maxl) as usize] = m as i32;
                self.valid1 = self.valid1.min(self.length1 - maxl);
                let (nud, nrl, nfb) = (self.node_ud[ml], self.node_rl[ml], self.node_fb[ml]);
                let ret =
                    self.phase1_opt(nud, nrl, nfb, ssym & t.sym.move_cube_sym[m], maxl - 1, axis);
                if ret == 0 {
                    return 0;
                }
            }
            axis += 3;
        }
        1
    }

    fn append_sol_move(&mut self, cur_move: i32) {
        if self.sol == 0 {
            self.move_sol[0] = cur_move;
            self.sol = 1;
            return;
        }
        let s = self.sol as usize;
        let axis_cur = cur_move / 3;
        let axis_last = self.move_sol[s - 1] / 3;
        if axis_cur == axis_last {
            let pow = (cur_move % 3 + self.move_sol[s - 1] % 3 + 1) % 4;
            if pow == 3 {
                self.sol -= 1;
            } else {
                self.move_sol[s - 1] = axis_cur * 3 + pow;
            }
            return;
        }
        if self.sol > 1 && axis_cur % 3 == axis_last % 3 && axis_cur == self.move_sol[s - 2] / 3 {
            let pow = (cur_move % 3 + self.move_sol[s - 2] % 3 + 1) % 4;
            if pow == 3 {
                self.move_sol[s - 2] = self.move_sol[s - 1];
                self.sol -= 1;
            } else {
                self.move_sol[s - 2] = axis_cur * 3 + pow;
            }
            return;
        }
        self.move_sol[s] = cur_move;
        self.sol += 1;
    }

    /// Returns `-1` if no solution was found, otherwise how many moves shorter than `maxl`
    /// the solution found is.
    fn phase2(
        &mut self,
        edge: i32,
        esym: i32,
        corn: i32,
        csym: i32,
        mid: i32,
        maxl: i32,
        depth: i32,
        lm: usize,
    ) -> i32 {
        if let Some(w) = &self.wca {
            if self.depth1 == 0
                && depth == 1
                && (w.first_move_filter[self.urf_idx] >> UD2STD[lm] & 1) != 0
            {
                return -1;
            }
            if edge == 0
                && corn == 0
                && mid == 0
                && (self.pre_move_len > 0
                    || (w.last_move_filter[self.urf_idx] >> UD2STD[lm] & 1) == 0)
            {
                return maxl;
            }
        } else if edge == 0 && corn == 0 && mid == 0 {
            return maxl;
        }

        let t = self.t;
        let move_mask = UTIL.ckmv2bit[lm];
        let mut m = 0_usize;
        while m < 10 {
            if (move_mask >> m & 1) != 0 {
                m += (0x42 >> m & 3) as usize + 1;
                continue;
            }
            let midx = i32::from(t.mperm_move[mid as usize][m]);
            let mut cornx =
                i32::from(t.cperm_move[corn as usize][t.sym.sym_move_ud[csym as usize][m]]);
            let csymx = t.sym.sym_mult[(cornx & 0xf) as usize][csym as usize] as i32;
            cornx >>= 4;
            let mut edgex =
                i32::from(t.eperm_move[edge as usize][t.sym.sym_move_ud[esym as usize][m]]);
            let esymx = t.sym.sym_mult[(edgex & 0xf) as usize][esym as usize] as i32;
            edgex >>= 4;
            let edgei = t.perm_sym_inv(edgex, esymx, false);
            let corni = t.perm_sym_inv(cornx, csymx, true);

            let prun = get_pruning(
                &t.eperm_ccombp_prun,
                (edgei >> 4) as usize * N_COMB
                    + usize::from(
                        t.ccombp_conj[usize::from(t.perm2combp[(corni >> 4) as usize])]
                            [t.sym.sym_mult_inv[(edgei & 0xf) as usize][(corni & 0xf) as usize]],
                    ),
            );
            if prun > maxl + 1 {
                return maxl - prun + 1;
            } else if prun >= maxl {
                m += ((0x42 >> m & 3) & (maxl - prun)) as usize + 1;
                continue;
            }
            let prun = get_pruning(
                &t.mcperm_prun,
                cornx as usize * N_MPERM + usize::from(t.mperm_conj[midx as usize][csymx as usize]),
            )
            .max(get_pruning(
                &t.eperm_ccombp_prun,
                edgex as usize * N_COMB
                    + usize::from(
                        t.ccombp_conj[usize::from(t.perm2combp[cornx as usize])]
                            [t.sym.sym_mult_inv[esymx as usize][csymx as usize]],
                    ),
            ));
            if prun >= maxl {
                m += ((0x42 >> m & 3) & (maxl - prun)) as usize + 1;
                continue;
            }
            let ret = self.phase2(edgex, esymx, cornx, csymx, midx, maxl - 1, depth + 1, m);
            if ret >= 0 {
                self.moves[depth as usize] = UD2STD[m] as i32;
                return ret;
            }
            m += 1;
        }
        -1
    }

    fn solution_to_string(&self) -> String {
        let (names, separator) = if self.wca.is_some() {
            (&WCA_MOVE2STR, ".")
        } else {
            (&MOVE2STR, ".  ")
        };
        let use_separator = self.verbose & USE_SEPARATOR != 0;
        let urf = if self.verbose & INVERSE_SOLUTION != 0 {
            (self.urf_idx + 3) % 6
        } else {
            self.urf_idx
        };
        let mut sb = String::new();
        let push_move = |sb: &mut String, s: usize| {
            sb.push_str(names[URF_MOVE[urf][self.move_sol[s] as usize]]);
            sb.push(' ');
        };
        if urf < 3 {
            for s in 0..self.sol as usize {
                if use_separator && s as i32 == self.depth1 {
                    sb.push_str(separator);
                }
                push_move(&mut sb, s);
            }
        } else {
            for s in (0..self.sol as usize).rev() {
                push_move(&mut sb, s);
                if use_separator && s as i32 == self.depth1 {
                    sb.push_str(separator);
                }
            }
        }
        if self.verbose & APPEND_LENGTH != 0 {
            let _ = write!(sb, "({}f)", self.sol);
        }
        sb
    }
}

/// The WCA flavour of min2phase (`SearchWCA`), which supports restricting the axes of the
/// first and last moves and measures its limits in milliseconds instead of probes.
#[derive(Debug, Clone, Default)]
pub struct SearchWca {
    search: Search,
}

/// The axis (0 = U/D, 1 = R/L, 2 = F/B) of a move name, if it is one of the 18 face turns.
fn str2axis(s: &str) -> Option<usize> {
    WCA_MOVE2STR.iter().position(|&m| m == s).map(|i| i / 3 % 3)
}

impl SearchWca {
    /// A new WCA searcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Like [`Search::solution`], except that `probe_max` and `probe_min` are measured in
    /// milliseconds of wall-clock time, and the first and last moves of the solution can be
    /// forbidden from turning a given axis. A restriction must be one of the 18 face turn
    /// names (`"U"`, `"R2"`, `"F'"`, ...); anything else returns `Error 9`.
    pub fn solution(
        &mut self,
        facelets: &str,
        max_depth: i32,
        probe_max: i64,
        probe_min: i64,
        verbose: i32,
        first_axis_restriction: Option<&str>,
        last_axis_restriction: Option<&str>,
    ) -> String {
        let mut wca = WcaState {
            first_move_filter: [0; 6],
            last_move_filter: [0; 6],
            is_axis_restricted: false,
            start_time: Instant::now(),
        };
        if let Some(first) = first_axis_restriction {
            let Some(axis) = str2axis(first) else {
                return "Error 9".to_owned();
            };
            for i in 0..3 {
                let mask = 0xe07 << (URF_MOVE[(3 - i) % 3][axis * 3] / 3 * 3);
                wca.first_move_filter[i] |= mask;
                wca.last_move_filter[i + 3] |= mask;
            }
            wca.is_axis_restricted = true;
        }
        if let Some(last) = last_axis_restriction {
            let Some(axis) = str2axis(last) else {
                return "Error 9".to_owned();
            };
            for i in 0..3 {
                let mask = 0xe07 << (URF_MOVE[(3 - i) % 3][axis * 3] / 3 * 3);
                wca.last_move_filter[i] |= mask;
                wca.first_move_filter[i + 3] |= mask;
            }
            wca.is_axis_restricted = true;
        }
        Search::init();
        wca.start_time = Instant::now();
        self.search.wca = Some(wca);
        self.search
            .solution(facelets, max_depth, probe_max, probe_min, verbose)
    }

    /// The length of the last solution found.
    pub fn length(&self) -> i32 {
        self.search.length()
    }

    /// Checks a facelet string; see [`Search::verify`].
    pub fn verify(&mut self, facelets: &str) -> i32 {
        self.search.verify(facelets)
    }
}
