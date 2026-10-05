//! The two phase Square-1 search (`cs.sq12phase.Search`).

use std::fmt::Write as _;

use super::full_cube::FullCube;
use super::tables::{TABLES, Tables};

/// Return the inverse of the solution, i.e. a scramble that generates the input state.
pub const INVERSE_SOLUTION: i32 = 0x2;

/// The optimal solver counts a simultaneous top and bottom turn as one move.
const PRUN_INC: i32 = 2;

/// Chen Shuang's two phase Square-1 solver: phase 1 solves the shape, phase 2 the
/// permutation.
#[derive(Clone)]
pub struct Search {
    t: &'static Tables,
    moves: [i32; 100],
    c: FullCube,
    length1: i32,
    movelen1: usize,
    maxlen2: i32,
    verbose: i32,
    sol_string: Option<String>,
}

impl std::fmt::Debug for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Search")
            .field("solution", &self.sol_string)
            .finish_non_exhaustive()
    }
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

/// The number of nibbles of `val` that are `0xf`.
fn count0xf(mut val: i32) -> i32 {
    val &= val >> 1;
    val &= val >> 2;
    (val & 0x1111_1111).count_ones() as i32
}

impl Search {
    /// A new searcher; the lookup tables are built on first use.
    pub fn new() -> Self {
        Self {
            t: &TABLES,
            moves: [0; 100],
            c: FullCube::default(),
            length1: 0,
            movelen1: 0,
            maxlen2: 0,
            verbose: 0,
            sol_string: None,
        }
    }

    /// Builds the lookup tables now instead of on the first solve.
    pub fn init() {
        let _ = &*TABLES;
    }

    /// Finds a (not necessarily optimal) solution in TNoodle's notation, e.g.
    /// `"(1,0) / (-3,3) / ..."`. `verbose` may contain [`INVERSE_SOLUTION`].
    pub fn solution(&mut self, c: &FullCube, verbose: i32) -> Option<String> {
        let t = self.t;
        self.c = *c;
        self.verbose = verbose;
        self.sol_string = None;
        let shape = c.shape_idx() as usize;
        self.length1 = t.shape.shape_prun[shape];
        while self.length1 < 100 {
            self.maxlen2 = (31 - self.length1).min(17);
            if self.ida_phase1(shape, t.shape.shape_prun[shape], self.length1, 0, -1) {
                break;
            }
            self.length1 += 1;
        }
        self.sol_string.clone()
    }

    /// Finds an optimal solution (in WCA metric) of at most `maxl` moves, if any.
    pub fn solution_opt(&mut self, c: &FullCube, maxl: i32, verbose: i32) -> Option<String> {
        let t = self.t;
        self.c = *c;
        self.verbose = verbose;
        self.sol_string = None;
        let shape = c.shape_idx() as usize;
        self.length1 = t.shape.shape_prun_opt[shape] * PRUN_INC;
        while self.length1 <= maxl * PRUN_INC {
            if self.phase1_opt(shape, self.length1, 0, -1, 0) {
                break;
            }
            self.length1 += PRUN_INC;
        }
        self.sol_string.clone()
    }

    fn phase1_opt(
        &mut self,
        shape: usize,
        maxl: i32,
        depth: usize,
        lm: i32,
        last_turns: i32,
    ) -> bool {
        let t = self.t;
        let i = count0xf((last_turns ^ !0) & 0xff_00ff)
            - count0xf((last_turns ^ !0x66_6666) & 0xff_00ff);
        if i < 0 || i == 0 && (last_turns >> 20 & 0xf) >= 6 {
            return false;
        }
        if maxl / PRUN_INC == 0 {
            self.movelen1 = depth;
            if self.is_solved_in_phase1() {
                return true;
            }
            if maxl == 0 {
                return false;
            }
        }
        // Try each possible move, starting with the twist.
        if lm != 0 {
            let shapex = t.shape.twist_move[shape] as usize;
            let prun = t.shape.shape_prun_opt[shapex];
            if prun < maxl / PRUN_INC {
                self.moves[depth] = 0;
                let next_maxl = (maxl / PRUN_INC - 1) * PRUN_INC;
                if self.phase1_opt(shapex, next_maxl, depth + 1, 0, last_turns << 8) {
                    return true;
                }
            }
        }
        // Try the top layer.
        if lm <= 0 {
            let mut m = 0;
            let mut shapex = shape as i32;
            loop {
                m += t.shape.top_move[shapex as usize];
                shapex = m >> 4;
                m &= 0xf;
                if m >= 12 {
                    break;
                }
                let prun = t.shape.shape_prun_opt[shapex as usize];
                if prun * PRUN_INC > maxl + PRUN_INC - 1 {
                    break;
                } else if prun * PRUN_INC < maxl + PRUN_INC - 1 {
                    self.moves[depth] = m;
                    if self.phase1_opt(shapex as usize, maxl - 1, depth + 1, 1, last_turns | m << 4)
                    {
                        return true;
                    }
                }
            }
        }
        // Try the bottom layer.
        if lm <= 1 {
            let mut m = 0;
            let mut shapex = shape as i32;
            loop {
                m += t.shape.bottom_move[shapex as usize];
                shapex = m >> 4;
                m &= 0xf;
                if m >= 12 {
                    break;
                }
                let prun = t.shape.shape_prun_opt[shapex as usize];
                if prun * PRUN_INC > maxl + PRUN_INC - 1 {
                    break;
                } else if prun * PRUN_INC < maxl + PRUN_INC - 1 {
                    self.moves[depth] = -m;
                    if self.phase1_opt(shapex as usize, maxl - 1, depth + 1, 2, last_turns | m) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn ida_phase1(
        &mut self,
        shape: usize,
        prunvalue: i32,
        maxl: i32,
        depth: usize,
        lm: i32,
    ) -> bool {
        let t = self.t;
        if prunvalue == 0 && maxl < 4 {
            self.movelen1 = depth;
            return maxl == 0 && self.init_phase2();
        }
        // Try each possible move, starting with the twist.
        if lm != 0 {
            let shapex = t.shape.twist_move[shape] as usize;
            let prun = t.shape.shape_prun[shapex];
            if prun < maxl {
                self.moves[depth] = 0;
                if self.ida_phase1(shapex, prun, maxl - 1, depth + 1, 0) {
                    return true;
                }
            }
        }
        // Try the top layer.
        if lm <= 0 {
            let mut m = 0;
            let mut shapex = shape as i32;
            loop {
                m += t.shape.top_move[shapex as usize];
                shapex = m >> 4;
                m &= 0xf;
                if m >= 12 {
                    break;
                }
                let prun = t.shape.shape_prun[shapex as usize];
                if prun > maxl {
                    break;
                } else if prun < maxl {
                    self.moves[depth] = m;
                    if self.ida_phase1(shapex as usize, prun, maxl - 1, depth + 1, 1) {
                        return true;
                    }
                }
            }
        }
        // Try the bottom layer.
        if lm <= 1 {
            let mut m = 0;
            let mut shapex = shape as i32;
            loop {
                m += t.shape.bottom_move[shapex as usize];
                shapex = m >> 4;
                m &= 0xf;
                if m >= 6 {
                    break;
                }
                let prun = t.shape.shape_prun[shapex as usize];
                if prun > maxl {
                    break;
                } else if prun < maxl {
                    self.moves[depth] = -m;
                    if self.ida_phase1(shapex as usize, prun, maxl - 1, depth + 1, 2) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn after_phase1(&self) -> FullCube {
        let mut d = self.c;
        for &m in &self.moves[..self.movelen1] {
            d.do_move(m);
        }
        d
    }

    fn is_solved_in_phase1(&mut self) -> bool {
        let solved = self.after_phase1().is_solved();
        if solved {
            self.sol_string = Some(self.move2string(self.movelen1));
        }
        solved
    }

    fn init_phase2(&mut self) -> bool {
        let t = &self.t.square;
        let sq = self.after_phase1().square();
        let edge = sq.edgeperm;
        let corner = sq.cornperm;
        let ml = sq.ml;
        let prun = i32::from(t.square_prun[(sq.edgeperm << 1 | ml) as usize])
            .max(i32::from(t.square_prun[(sq.cornperm << 1 | ml) as usize]));
        for i in prun..self.maxlen2 {
            if self.ida_phase2(
                edge,
                corner,
                sq.top_edge_first,
                sq.bot_edge_first,
                ml,
                i,
                self.movelen1,
                0,
            ) {
                self.sol_string = Some(self.move2string(i as usize + self.movelen1));
                return true;
            }
        }
        false
    }

    fn move2string(&self, len: usize) -> String {
        let output_moves: Vec<i32> = if self.verbose & INVERSE_SOLUTION != 0 {
            (0..len)
                .rev()
                .map(|i| {
                    let m = self.moves[i];
                    match m.cmp(&0) {
                        std::cmp::Ordering::Greater => 12 - m,
                        std::cmp::Ordering::Less => -12 - m,
                        std::cmp::Ordering::Equal => m,
                    }
                })
                .collect()
        } else {
            self.moves[..len].to_vec()
        };
        let mut s = String::new();
        let mut top = 0;
        let mut bottom = 0;
        for &val in &output_moves {
            match val.cmp(&0) {
                std::cmp::Ordering::Greater => top = if val > 6 { val - 12 } else { val },
                std::cmp::Ordering::Less => bottom = if -val > 6 { -val - 12 } else { -val },
                std::cmp::Ordering::Equal => {
                    if top == 0 && bottom == 0 {
                        s.push_str(" / ");
                    } else {
                        let _ = write!(s, "({top},{bottom}) / ");
                    }
                    top = 0;
                    bottom = 0;
                }
            }
        }
        if top != 0 || bottom != 0 {
            let _ = write!(s, "({top},{bottom})");
        }
        s
    }

    fn ida_phase2(
        &mut self,
        edge: i32,
        corner: i32,
        top_edge_first: bool,
        bot_edge_first: bool,
        ml: i32,
        maxl: i32,
        depth: usize,
        lm: i32,
    ) -> bool {
        if maxl == 0 && !top_edge_first && bot_edge_first {
            return true;
        }
        let t = &self.t.square;
        let prun = |perm: i32, ml: i32| i32::from(t.square_prun[(perm << 1 | ml) as usize]);

        // Try each possible move, starting with the twist.
        if lm != 0 && top_edge_first == bot_edge_first {
            let edgex = i32::from(t.twist_move[edge as usize]);
            let cornerx = i32::from(t.twist_move[corner as usize]);
            if prun(edgex, 1 - ml) < maxl && prun(cornerx, 1 - ml) < maxl {
                self.moves[depth] = 0;
                if self.ida_phase2(
                    edgex,
                    cornerx,
                    top_edge_first,
                    bot_edge_first,
                    1 - ml,
                    maxl - 1,
                    depth + 1,
                    0,
                ) {
                    return true;
                }
            }
        }

        // Try the top layer.
        if lm <= 0 {
            let mut top_edge_firstx = !top_edge_first;
            let mut edgex = if top_edge_firstx {
                i32::from(t.top_move[edge as usize])
            } else {
                edge
            };
            let mut cornerx = if top_edge_firstx {
                corner
            } else {
                i32::from(t.top_move[corner as usize])
            };
            let mut m = if top_edge_firstx { 1 } else { 2 };
            let mut prun1 = prun(edgex, ml);
            let mut prun2 = prun(cornerx, ml);
            // The Java source checks `prun1 <= maxl` twice; the condition is preserved.
            while m < 12 && prun1 <= maxl {
                if prun1 < maxl && prun2 < maxl {
                    self.moves[depth] = m;
                    if self.ida_phase2(
                        edgex,
                        cornerx,
                        top_edge_firstx,
                        bot_edge_first,
                        ml,
                        maxl - 1,
                        depth + 1,
                        1,
                    ) {
                        return true;
                    }
                }
                top_edge_firstx = !top_edge_firstx;
                if top_edge_firstx {
                    edgex = i32::from(t.top_move[edgex as usize]);
                    prun1 = prun(edgex, ml);
                    m += 1;
                } else {
                    cornerx = i32::from(t.top_move[cornerx as usize]);
                    prun2 = prun(cornerx, ml);
                    m += 2;
                }
            }
        }

        // Try the bottom layer.
        if lm <= 1 {
            let mut bot_edge_firstx = !bot_edge_first;
            let mut edgex = if bot_edge_firstx {
                i32::from(t.bottom_move[edge as usize])
            } else {
                edge
            };
            let mut cornerx = if bot_edge_firstx {
                corner
            } else {
                i32::from(t.bottom_move[corner as usize])
            };
            let mut m = if bot_edge_firstx { 1 } else { 2 };
            let mut prun1 = prun(edgex, ml);
            let mut prun2 = prun(cornerx, ml);
            let limit = if maxl > 6 { 6 } else { 12 };
            while m < limit && prun1 <= maxl {
                if prun1 < maxl && prun2 < maxl {
                    self.moves[depth] = -m;
                    if self.ida_phase2(
                        edgex,
                        cornerx,
                        top_edge_first,
                        bot_edge_firstx,
                        ml,
                        maxl - 1,
                        depth + 1,
                        2,
                    ) {
                        return true;
                    }
                }
                bot_edge_firstx = !bot_edge_firstx;
                if bot_edge_firstx {
                    edgex = i32::from(t.bottom_move[edgex as usize]);
                    prun1 = prun(edgex, ml);
                    m += 1;
                } else {
                    cornerx = i32::from(t.bottom_move[cornerx as usize]);
                    prun2 = prun(cornerx, ml);
                    m += 2;
                }
            }
        }
        false
    }
}
