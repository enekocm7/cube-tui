//! The Skewb optimal solver (`SkewbSolver`), working in Jaap's notation internally.

use std::sync::LazyLock;

use crate::java::RandomSource;

const N_MOVES: usize = 4;
/// `FACT[x] = x! / 2`.
const FACT: [i32; 7] = [1, 1, 1, 3, 12, 60, 360];
const TWIST_ORIENTATIONS: usize = 2187;
/// The number of (even) permutations of the free corners.
pub const FREE_CORNER_PERM: i32 = FACT[4];
const CENTER_PERM: i32 = FACT[6];
const SKEWB_PERMUTATIONS: usize = (FREE_CORNER_PERM * CENTER_PERM) as usize;
const MAX_SOLUTION_LENGTH: usize = 12;

const CORNER_PERM_MV: [[u8; 4]; 12] = [
    [6, 5, 10, 1],
    [9, 7, 4, 2],
    [3, 11, 8, 0],
    [10, 1, 6, 5],
    [0, 8, 11, 3],
    [7, 9, 2, 4],
    [4, 2, 9, 7],
    [11, 3, 0, 8],
    [1, 10, 5, 6],
    [8, 0, 3, 11],
    [2, 4, 7, 9],
    [5, 6, 1, 10],
];
const ORI: [i32; 12] = [0, 1, 2, 0, 2, 1, 1, 2, 0, 2, 1, 0];

/// The (even) permutation index of six centers.
pub fn pack_center_perm(centers: &[i32; 6]) -> i32 {
    let mut idx = 0;
    let mut val = 0x54_3210;
    // The last two elements are implied by the first four and the parity.
    for (i, &c) in centers.iter().enumerate().take(4) {
        let v = c << 2;
        idx = (6 - i as i32) * idx + ((val >> v) & 0xf);
        val -= 0x11_1110 << v;
    }
    idx
}

/// The (even) permutation index of the four free corners.
pub fn pack_corner_perm(free_corners: &[i32; 4]) -> i32 {
    let mut idx = 0;
    let mut val = 0x3210;
    for (i, &c) in free_corners.iter().enumerate().take(2) {
        let v = c << 2;
        idx = (4 - i as i32) * idx + ((val >> v) & 0xf);
        val -= 0x1110 << v;
    }
    idx
}

/// The orientation index of the free and fixed corners.
pub fn pack_corner_orient(free_corners: &[i32; 4], fixed_corners: &[i32; 4]) -> i32 {
    let mut idx = 0;
    for i in (0..3).rev() {
        idx = idx * 3 + free_corners[i] % 3;
    }
    for i in (0..4).rev() {
        idx = idx * 3 + fixed_corners[i];
    }
    idx
}

fn permmv(idx: usize, mv: usize) -> i32 {
    let mut centerindex = idx as i32 / FREE_CORNER_PERM;
    let cornerindex = idx % FREE_CORNER_PERM as usize;
    let mut val = 0x54_3210;
    let mut parity = 0;
    let mut centerperm = [0; 6];
    for i in 0..5 {
        let p = FACT[5 - i];
        let mut v = centerindex / p;
        centerindex -= v * p;
        parity ^= v;
        v <<= 2;
        centerperm[i] = (val >> v) & 0xf;
        let m = (1 << v) - 1;
        val = (val & m) + ((val >> 4) & !m);
    }
    if parity & 1 == 0 {
        centerperm[5] = val;
    } else {
        centerperm[5] = centerperm[4];
        centerperm[4] = val;
    }
    let cycle = |c: &mut [i32; 6], a: usize, b: usize, d: usize| {
        let t = c[a];
        c[a] = c[b];
        c[b] = c[d];
        c[d] = t;
    };
    match mv {
        0 => cycle(&mut centerperm, 0, 1, 3),
        1 => cycle(&mut centerperm, 0, 4, 2),
        2 => cycle(&mut centerperm, 1, 2, 5),
        3 => cycle(&mut centerperm, 3, 5, 4),
        _ => {}
    }
    pack_center_perm(&centerperm) * FREE_CORNER_PERM + i32::from(CORNER_PERM_MV[cornerindex][mv])
}

fn twstmv(mut idx: i32, mv: usize) -> i32 {
    let mut fixedtwst = [0; 4];
    let mut twst = [0; 4];
    for t in &mut fixedtwst {
        *t = idx % 3;
        idx /= 3;
    }
    for t in twst.iter_mut().take(3) {
        *t = idx % 3;
        idx /= 3;
    }
    twst[3] = (6 - twst[0] - twst[1] - twst[2]) % 3;
    fixedtwst[mv] = (fixedtwst[mv] + 1) % 3;
    let cycle = |t: &mut [i32; 4], a: usize, b: usize, c: usize| {
        let tmp = t[a];
        t[a] = t[b] + 2;
        t[b] = t[c] + 2;
        t[c] = tmp + 2;
    };
    match mv {
        0 => cycle(&mut twst, 0, 2, 1),
        1 => cycle(&mut twst, 0, 1, 3),
        2 => cycle(&mut twst, 0, 3, 2),
        3 => cycle(&mut twst, 1, 2, 3),
        _ => {}
    }
    pack_corner_orient(&twst, &fixedtwst)
}

struct Tables {
    permmv: Vec<[u16; N_MOVES]>,
    twstmv: Vec<[u16; N_MOVES]>,
    permprun: Vec<i8>,
    twstprun: Vec<i8>,
}

fn prun_bfs(moves: &[[u16; N_MOVES]]) -> Vec<i8> {
    let mut prun = vec![-1_i8; moves.len()];
    prun[0] = 0;
    for l in 0..6 {
        for p in 0..moves.len() {
            if prun[p] == l {
                for m in 0..N_MOVES {
                    let mut q = p;
                    for _ in 0..2 {
                        q = usize::from(moves[q][m]);
                        if prun[q] == -1 {
                            prun[q] = l + 1;
                        }
                    }
                }
            }
        }
    }
    prun
}

static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    let permmv: Vec<[u16; N_MOVES]> = (0..SKEWB_PERMUTATIONS)
        .map(|i| std::array::from_fn(|j| permmv(i, j) as u16))
        .collect();
    let twstmv: Vec<[u16; N_MOVES]> = (0..TWIST_ORIENTATIONS)
        .map(|i| std::array::from_fn(|j| twstmv(i as i32, j) as u16))
        .collect();
    let permprun = prun_bfs(&permmv);
    let twstprun = prun_bfs(&twstmv);
    Tables {
        permmv,
        twstmv,
        permprun,
        twstprun,
    }
});

/// A Skewb position as coordinates (`SkewbSolverState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SkewbSolverState {
    /// Center and free corner permutation.
    pub perm: i32,
    /// Corner orientations.
    pub twst: i32,
}

impl SkewbSolverState {
    /// Whether the twist is consistent with the permutation.
    pub fn is_solvable(&self) -> bool {
        ORI[(self.perm % 12) as usize]
            == (self.twst + self.twst / 3 + self.twst / 9 + self.twst / 27) % 3
    }
}

/// The Skewb solver. Searches visit moves in an order randomised by the given source.
#[derive(Debug, Clone, Copy, Default)]
pub struct SkewbSolver;

impl SkewbSolver {
    /// Creates a solver; the tables are built on first use.
    pub const fn new() -> Self {
        Self
    }

    /// A uniformly random Skewb position.
    pub fn random_state(&self, r: &mut dyn RandomSource) -> SkewbSolverState {
        let perm = r.next_int_bounded(SKEWB_PERMUTATIONS as i32);
        loop {
            let state = SkewbSolverState {
                perm,
                twst: r.next_int_bounded(TWIST_ORIENTATIONS as i32),
            };
            if state.is_solvable() {
                return state;
            }
        }
    }

    /// A solution (in fixed corner notation) of at most `length` moves.
    pub fn solve_in(
        &self,
        state: SkewbSolverState,
        length: i32,
        r: &mut dyn RandomSource,
    ) -> Option<String> {
        Self::solve(state, length, false, false, r)
    }

    /// A scramble of exactly `length` moves that generates `state`.
    pub fn generate_exactly(
        &self,
        state: SkewbSolverState,
        length: i32,
        r: &mut dyn RandomSource,
    ) -> Option<String> {
        Self::solve(state, length, true, true, r)
    }

    /// `SkewbSolver.solve`, with the move order randomness made explicit.
    pub fn solve(
        state: SkewbSolverState,
        desired_length: i32,
        exact_length: bool,
        inverse: bool,
        r: &mut dyn RandomSource,
    ) -> Option<String> {
        let mut sol = [0_usize; MAX_SOLUTION_LENGTH];
        let mut solution_length = None;
        let mut length = if exact_length { desired_length } else { 0 };
        while length <= desired_length {
            solution_length = search(
                0,
                state.perm as usize,
                state.twst as usize,
                length,
                None,
                &mut sol,
                r,
            );
            if solution_length.is_some() {
                break;
            }
            length += 1;
        }
        let solution_length = solution_length?;
        if solution_length == 0 {
            return Some(String::new());
        }
        let fcn_solution = solution_to_fcn(&sol[..solution_length]);
        if inverse {
            let scramble: Vec<String> = fcn_solution
                .split(' ')
                .rev()
                .map(|mv| {
                    // Skewb turns are always 120 or -120 degrees.
                    mv.strip_suffix('\'')
                        .map_or_else(|| format!("{mv}'"), str::to_owned)
                })
                .collect();
            return Some(scramble.join(" "));
        }
        Some(fcn_solution)
    }
}

fn search(
    depth: usize,
    perm: usize,
    twst: usize,
    maxl: i32,
    lm: Option<usize>,
    sol: &mut [usize; MAX_SOLUTION_LENGTH],
    r: &mut dyn RandomSource,
) -> Option<usize> {
    let t = &*TABLES;
    if maxl == 0 {
        return (perm == 0 && twst == 0).then_some(depth);
    }
    if i32::from(t.permprun[perm]) > maxl || i32::from(t.twstprun[twst]) > maxl {
        return None;
    }
    let random_offset = r.next_int_bounded(N_MOVES as i32) as usize;
    for m in 0..N_MOVES {
        let random_move = (m + random_offset) % N_MOVES;
        if Some(random_move) == lm {
            continue;
        }
        let mut p = perm;
        let mut s = twst;
        for a in 0..2 {
            p = usize::from(t.permmv[p][random_move]);
            s = usize::from(t.twstmv[s][random_move]);
            if let Some(len) = search(depth + 1, p, s, maxl - 1, Some(random_move), sol, r) {
                sol[depth] = random_move * 2 + a;
                return Some(len);
            }
        }
    }
    None
}

/// Converts a solution in Jaap's notation to WCA fixed corner notation: rotate by z2 (which
/// maps Jaap's `R L D B` to `L R F U`), then replace every `F` by `B` and relabel the
/// following moves accordingly (`F R` becomes `B U`).
fn solution_to_fcn(sol: &[usize]) -> String {
    let mut move2str = ["L", "R", "B", "U"];
    let mut moves = Vec::with_capacity(sol.len());
    for &m in sol {
        let axis = m >> 1;
        let pow = m & 1;
        if axis == 2 {
            for _ in 0..=pow {
                let temp = move2str[0];
                move2str[0] = move2str[1];
                move2str[1] = move2str[3];
                move2str[3] = temp;
            }
        }
        moves.push(format!(
            "{}{}",
            move2str[axis],
            if pow == 1 { "'" } else { "" }
        ));
    }
    moves.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::java::JavaRandom;

    #[test]
    fn solved_state_needs_no_moves() {
        let mut r = JavaRandom::new(0);
        assert_eq!(
            SkewbSolver::new()
                .solve_in(SkewbSolverState::default(), 3, &mut r)
                .as_deref(),
            Some("")
        );
        assert!(SkewbSolverState::default().is_solvable());
    }
}
