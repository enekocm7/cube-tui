//! The Pyraminx optimal solver (`PyraminxSolver`).
//!
//! The 4 corners are fixed and only twist; 6 edges move. Tips are handled separately.

use std::sync::LazyLock;

use crate::java::RandomSource;

const N_EDGE_PERM: usize = 720;
const N_EDGE_ORIENT: usize = 32;
const N_CORNER_ORIENT: usize = 81;
const N_ORIENT: usize = N_EDGE_ORIENT * N_CORNER_ORIENT;
const N_TIPS: i32 = 81;
const N_MOVES: usize = 8;
const MAX_LENGTH: usize = 20;
const MOVE_TO_STRING: [&str; 8] = ["U", "U'", "L", "L'", "R", "R'", "B", "B'"];
const INVERSE_MOVE_TO_STRING: [&str; 8] = ["U'", "U", "L'", "L", "R'", "R", "B'", "B"];
const TIP_TO_STRING: [&str; 8] = ["u", "u'", "l", "l'", "r", "r'", "b", "b'"];
const INVERSE_TIP_TO_STRING: [&str; 8] = ["u'", "u", "l'", "l", "r'", "r", "b'", "b"];
const FACT: [i32; 7] = [1, 1, 2, 6, 24, 120, 720];

/// The edge permutation index of six edges (`orientation << 3 | position`).
pub fn pack_edge_perm(edges: &[i32; 6]) -> i32 {
    let mut idx = 0;
    let mut val = 0x54_3210;
    for (i, &e) in edges.iter().enumerate().take(5) {
        let v = (e & 0x7) << 2;
        idx = (6 - i as i32) * idx + ((val >> v) & 0x7);
        val -= 0x11_1110 << v;
    }
    idx
}

fn unpack_edge_perm(mut perm: i32, edges: &mut [i32; 6]) {
    let mut val = 0x54_3210;
    for i in 0..5 {
        let p = FACT[5 - i];
        let mut v = perm / p;
        perm -= v * p;
        v <<= 2;
        edges[i] = (val >> v) & 0x7;
        let m = (1 << v) - 1;
        val = (val & m) + ((val >> 4) & !m);
    }
    edges[5] = val;
}

/// The edge orientation index of six edges (the sixth is implied).
pub fn pack_edge_orient(edges: &[i32; 6]) -> i32 {
    edges[..5].iter().fold(0, |ori, &e| 2 * ori + (e >> 3))
}

fn unpack_edge_orient(mut ori: i32, edges: &mut [i32; 6]) {
    let mut sum_ori = 0;
    for i in (0..5).rev() {
        edges[i] = (ori & 1) << 3;
        sum_ori ^= ori & 1;
        ori >>= 1;
    }
    edges[5] = sum_ori << 3;
}

/// The orientation index of four corners (also used for the tips).
pub fn pack_corner_orient(corners: &[i32; 4]) -> i32 {
    corners.iter().fold(0, |ori, &c| 3 * ori + c)
}

fn unpack_corner_orient(mut ori: i32, corners: &mut [i32; 4]) {
    for i in (0..4).rev() {
        corners[i] = ori % 3;
        ori /= 3;
    }
}

fn cycle_and_orient(edges: &mut [i32; 6], a: usize, b: usize, c: usize, times: usize) {
    for _ in 0..times {
        let temp = edges[c];
        edges[c] = (edges[b] + 8) % 16;
        edges[b] = (edges[a] + 8) % 16;
        edges[a] = temp;
    }
}

fn move_edges(edges: &mut [i32; 6], mv: usize) {
    let times = mv % 2 + 1;
    match mv / 2 {
        0 => cycle_and_orient(edges, 5, 3, 1, times),
        1 => cycle_and_orient(edges, 2, 1, 0, times),
        2 => cycle_and_orient(edges, 0, 3, 4, times),
        3 => cycle_and_orient(edges, 2, 4, 5, times),
        _ => {}
    }
}

fn move_corners(corners: &mut [i32; 4], mv: usize) {
    let face = mv / 2;
    let times = (mv % 2 + 1) as i32;
    corners[face] = (corners[face] + times) % 3;
}

/// The move and pruning tables.
pub(crate) struct Tables {
    pub(crate) move_edge_perm: Vec<[i32; N_MOVES]>,
    pub(crate) move_edge_orient: Vec<[i32; N_MOVES]>,
    pub(crate) move_corner_orient: Vec<[i32; N_MOVES]>,
    prun_perm: Vec<i32>,
    prun_orient: Vec<i32>,
}

pub(crate) static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    let mut edges1 = [0; 6];
    let mut move_edge_perm = vec![[0; N_MOVES]; N_EDGE_PERM];
    for (perm, row) in move_edge_perm.iter_mut().enumerate() {
        unpack_edge_perm(perm as i32, &mut edges1);
        for (mv, v) in row.iter_mut().enumerate() {
            let mut edges2 = edges1;
            move_edges(&mut edges2, mv);
            *v = pack_edge_perm(&edges2);
        }
    }
    let mut move_edge_orient = vec![[0; N_MOVES]; N_EDGE_ORIENT];
    for (orient, row) in move_edge_orient.iter_mut().enumerate() {
        unpack_edge_orient(orient as i32, &mut edges1);
        for (mv, v) in row.iter_mut().enumerate() {
            let mut edges2 = edges1;
            move_edges(&mut edges2, mv);
            *v = pack_edge_orient(&edges2);
        }
    }
    let mut corners1 = [0; 4];
    let mut move_corner_orient = vec![[0; N_MOVES]; N_CORNER_ORIENT];
    for (orient, row) in move_corner_orient.iter_mut().enumerate() {
        unpack_corner_orient(orient as i32, &mut corners1);
        for (mv, v) in row.iter_mut().enumerate() {
            let mut corners2 = corners1;
            move_corners(&mut corners2, mv);
            *v = pack_corner_orient(&corners2);
        }
    }

    let mut prun_perm = vec![-1; N_EDGE_PERM];
    prun_perm[0] = 0;
    let mut done = 1;
    let mut length = 0;
    // Only half of the permutations are reachable because of parity.
    while done < N_EDGE_PERM / 2 {
        for perm in 0..N_EDGE_PERM {
            if prun_perm[perm] == length {
                for &next in &move_edge_perm[perm] {
                    if prun_perm[next as usize] == -1 {
                        prun_perm[next as usize] = length + 1;
                        done += 1;
                    }
                }
            }
        }
        length += 1;
    }

    let mut prun_orient = vec![-1; N_ORIENT];
    prun_orient[0] = 0;
    done = 1;
    length = 0;
    while done < N_ORIENT {
        for orient in 0..N_ORIENT {
            if prun_orient[orient] == length {
                for mv in 0..N_MOVES {
                    let new_edge = move_edge_orient[orient % N_EDGE_ORIENT][mv];
                    let new_corner = move_corner_orient[orient / N_EDGE_ORIENT][mv];
                    let new_orient = new_corner as usize * N_EDGE_ORIENT + new_edge as usize;
                    if prun_orient[new_orient] == -1 {
                        prun_orient[new_orient] = length + 1;
                        done += 1;
                    }
                }
            }
        }
        length += 1;
    }

    Tables {
        move_edge_perm,
        move_edge_orient,
        move_corner_orient,
        prun_perm,
        prun_orient,
    }
});

/// A Pyraminx position as coordinates (`PyraminxSolverState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PyraminxSolverState {
    /// Edge permutation, `0..720` (only even permutations are reachable).
    pub edge_perm: i32,
    /// Edge orientation, `0..32`.
    pub edge_orient: i32,
    /// Corner orientation, `0..81`.
    pub corner_orient: i32,
    /// Tip orientation, `0..81`.
    pub tips: i32,
}

impl PyraminxSolverState {
    /// The number of tips that are not solved.
    pub fn unsolved_tips(&self) -> i32 {
        let mut n = 0;
        let mut t = self.tips;
        while t != 0 {
            if t % 3 > 0 {
                n += 1;
            }
            t /= 3;
        }
        n
    }
}

/// The Pyraminx solver. Searches visit moves in an order randomised by the given source,
/// so that equally short solutions are chosen at random.
#[derive(Debug, Clone, Copy, Default)]
pub struct PyraminxSolver;

impl PyraminxSolver {
    /// Creates a solver; the tables are built on first use.
    pub const fn new() -> Self {
        Self
    }

    /// A uniformly random Pyraminx position.
    pub fn random_state(&self, r: &mut dyn RandomSource) -> PyraminxSolverState {
        let t = &*TABLES;
        let edge_perm = loop {
            let p = r.next_int_bounded(N_EDGE_PERM as i32);
            // Odd permutations are unreachable.
            if t.prun_perm[p as usize] != -1 {
                break p;
            }
        };
        let edge_orient = r.next_int_bounded(N_EDGE_ORIENT as i32);
        let corner_orient = r.next_int_bounded(N_CORNER_ORIENT as i32);
        let tips = r.next_int_bounded(N_TIPS);
        PyraminxSolverState {
            edge_perm,
            edge_orient,
            corner_orient,
            tips,
        }
    }

    /// A solution of at most `length` moves (counting tip turns if `including_tips`).
    pub fn solve_in(
        &self,
        state: PyraminxSolverState,
        length: i32,
        including_tips: bool,
        r: &mut dyn RandomSource,
    ) -> Option<String> {
        Self::solve(state, length, false, false, including_tips, r)
    }

    /// A scramble of exactly `length` moves that generates `state`.
    pub fn generate_exactly(
        &self,
        state: PyraminxSolverState,
        length: i32,
        including_tips: bool,
        r: &mut dyn RandomSource,
    ) -> Option<String> {
        Self::solve(state, length, true, true, including_tips, r)
    }

    /// `PyraminxSolver.solve`, with the move order randomness made explicit.
    pub fn solve(
        state: PyraminxSolverState,
        desired_length: i32,
        exact_length: bool,
        inverse: bool,
        including_tips: bool,
        r: &mut dyn RandomSource,
    ) -> Option<String> {
        let mut solution = [0_usize; MAX_LENGTH];
        let mut desired_length = desired_length;
        if including_tips {
            desired_length -= state.unsolved_tips();
        }
        let mut length = if exact_length { desired_length } else { 0 };
        let mut found = false;
        while length <= desired_length {
            if search(
                state.edge_perm,
                state.edge_orient,
                state.corner_orient,
                0,
                length,
                42,
                &mut solution,
                r,
            ) {
                found = true;
                break;
            }
            length += 1;
        }
        if !found {
            return None;
        }

        let mut scramble = String::new();
        let length = length.max(0) as usize;
        if inverse {
            for &m in solution[..length].iter().rev() {
                scramble.push(' ');
                scramble.push_str(INVERSE_MOVE_TO_STRING[m]);
            }
        } else {
            for &m in &solution[..length] {
                scramble.push(' ');
                scramble.push_str(MOVE_TO_STRING[m]);
            }
        }
        // Scramble the tips.
        let mut tips = [0; 4];
        unpack_corner_orient(state.tips, &mut tips);
        for (tip, &dir) in tips.iter().enumerate() {
            if dir > 0 {
                let names = if inverse {
                    &TIP_TO_STRING
                } else {
                    &INVERSE_TIP_TO_STRING
                };
                scramble.push(' ');
                scramble.push_str(names[tip * 2 + dir as usize - 1]);
            }
        }
        Some(scramble.trim().to_owned())
    }
}

#[allow(clippy::too_many_arguments)]
fn search(
    edge_perm: i32,
    edge_orient: i32,
    corner_orient: i32,
    depth: usize,
    length: i32,
    last_move: usize,
    solution: &mut [usize; MAX_LENGTH],
    r: &mut dyn RandomSource,
) -> bool {
    let t = &*TABLES;
    if length == 0 {
        return edge_perm == 0 && edge_orient == 0 && corner_orient == 0;
    }
    if t.prun_perm[edge_perm as usize] > length
        || t.prun_orient[corner_orient as usize * N_EDGE_ORIENT + edge_orient as usize] > length
    {
        return false;
    }
    let random_offset = r.next_int_bounded(N_MOVES as i32) as usize;
    for mv in 0..N_MOVES {
        let random_move = (mv + random_offset) % N_MOVES;
        if random_move / 2 == last_move / 2 {
            continue;
        }
        if search(
            t.move_edge_perm[edge_perm as usize][random_move],
            t.move_edge_orient[edge_orient as usize][random_move],
            t.move_corner_orient[corner_orient as usize][random_move],
            depth + 1,
            length - 1,
            random_move,
            solution,
            r,
        ) {
            solution[depth] = random_move;
            return true;
        }
    }
    false
}

/// A move table: `table[coordinate][move]`.
pub type MoveTable = &'static [[i32; 8]];

/// The move tables, exposed for tests: `(edge_perm, edge_orient, corner_orient)`.
pub fn move_tables() -> (MoveTable, MoveTable, MoveTable) {
    let t = &*TABLES;
    (
        &t.move_edge_perm,
        &t.move_edge_orient,
        &t.move_corner_orient,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::java::JavaRandom;

    #[test]
    fn coordinates_round_trip() {
        let mut e = [0; 6];
        for p in 0..N_EDGE_PERM as i32 {
            unpack_edge_perm(p, &mut e);
            assert_eq!(pack_edge_perm(&e), p);
        }
        for o in 0..N_EDGE_ORIENT as i32 {
            unpack_edge_orient(o, &mut e);
            assert_eq!(pack_edge_orient(&e), o);
        }
        let mut c = [0; 4];
        for o in 0..N_CORNER_ORIENT as i32 {
            unpack_corner_orient(o, &mut c);
            assert_eq!(pack_corner_orient(&c), o);
        }
    }

    #[test]
    fn tips() {
        let s = PyraminxSolverState {
            tips: 1 + 2 * 27,
            ..PyraminxSolverState::default()
        };
        assert_eq!(s.unsolved_tips(), 2);
        let mut r = JavaRandom::new(0);
        assert_eq!(
            PyraminxSolver::new()
                .solve_in(s, 2, true, &mut r)
                .as_deref(),
            Some("u b'")
        );
        assert_eq!(PyraminxSolver::new().solve_in(s, 1, true, &mut r), None);
    }
}
