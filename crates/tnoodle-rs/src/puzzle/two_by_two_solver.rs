//! The 2x2x2 optimal solver (`TwoByTwoSolver`).
//!
//! Corner 7 (BLD) is considered fixed, so only U, R and F turns are needed. Each cubie is
//! encoded as `orientation << 3 | position`.

use std::sync::LazyLock;

use crate::java::RandomSource;

const N_PERM: usize = 5040;
const N_ORIENT: usize = 729;
const N_MOVES: usize = 9;
const MAX_LENGTH: usize = 20;
const MOVE_TO_STRING: [&str; 9] = ["U", "U2", "U'", "R", "R2", "R'", "F", "F2", "F'"];
const INVERSE_MOVE_TO_STRING: [&str; 9] = ["U'", "U2", "U", "R'", "R2", "R", "F'", "F2", "F"];
const FACT: [i32; 7] = [1, 1, 2, 6, 24, 120, 720];

/// A 2x2x2 position as permutation and orientation coordinates (`TwoByTwoState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TwoByTwoState {
    /// The permutation of the seven free corners, `0..5040`.
    pub permutation: i32,
    /// The orientation of six of the free corners, `0..729`.
    pub orientation: i32,
}

/// The permutation index of seven cubies.
pub fn pack_perm(cubies: &[i32; 7]) -> i32 {
    let mut idx = 0;
    let mut val = 0x654_3210;
    for (i, &c) in cubies.iter().enumerate().take(6) {
        let v = (c & 0x7) << 2;
        idx = (7 - i as i32) * idx + ((val >> v) & 0x7);
        val -= 0x111_1110 << v;
    }
    idx
}

fn unpack_perm(mut perm: i32, cubies: &mut [i32; 7]) {
    let mut val = 0x654_3210;
    for i in 0..6 {
        let p = FACT[6 - i];
        let mut v = perm / p;
        perm -= v * p;
        v <<= 2;
        cubies[i] = (val >> v) & 0x7;
        let m = (1 << v) - 1;
        val = (val & m) + ((val >> 4) & !m);
    }
    cubies[6] = val;
}

/// The orientation index of seven cubies (the seventh is implied).
pub fn pack_orient(cubies: &[i32; 7]) -> i32 {
    cubies[..6].iter().fold(0, |ori, &c| 3 * ori + (c >> 3))
}

fn unpack_orient(mut ori: i32, cubies: &mut [i32; 7]) {
    let mut sum_ori = 0;
    for i in (0..6).rev() {
        cubies[i] = (ori % 3) << 3;
        sum_ori += ori % 3;
        ori /= 3;
    }
    cubies[6] = ((42_424_242 - sum_ori) % 3) << 3;
}

fn cycle(cubies: &mut [i32; 7], a: usize, b: usize, c: usize, d: usize, times: usize) {
    for _ in 0..times {
        let temp = cubies[d];
        cubies[d] = cubies[c];
        cubies[c] = cubies[b];
        cubies[b] = cubies[a];
        cubies[a] = temp;
    }
}

fn cycle_and_orient(cubies: &mut [i32; 7], a: usize, b: usize, c: usize, d: usize, times: usize) {
    for _ in 0..times {
        let temp = cubies[d];
        cubies[d] = (cubies[c] + 8) % 24;
        cubies[c] = (cubies[b] + 16) % 24;
        cubies[b] = (cubies[a] + 8) % 24;
        cubies[a] = (temp + 16) % 24;
    }
}

fn move_cubies(cubies: &mut [i32; 7], mv: usize) {
    let times = mv % 3 + 1;
    match mv / 3 {
        0 => cycle(cubies, 1, 3, 2, 0, times),
        1 => cycle_and_orient(cubies, 0, 2, 6, 4, times),
        2 => cycle_and_orient(cubies, 1, 0, 4, 5, times),
        _ => {}
    }
}

/// The move and pruning tables.
pub(crate) struct Tables {
    /// `move_perm[perm][move]`.
    pub(crate) move_perm: Vec<[i32; N_MOVES]>,
    /// `move_orient[orient][move]`.
    pub(crate) move_orient: Vec<[i32; N_MOVES]>,
    prun_perm: Vec<i32>,
    prun_orient: Vec<i32>,
}

fn bfs(size: usize, moves: &[[i32; N_MOVES]]) -> Vec<i32> {
    let mut prun = vec![-1; size];
    prun[0] = 0;
    let mut done = 1;
    let mut length = 0;
    while done < size {
        for i in 0..size {
            if prun[i] == length {
                for &next in &moves[i] {
                    if prun[next as usize] == -1 {
                        prun[next as usize] = length + 1;
                        done += 1;
                    }
                }
            }
        }
        length += 1;
    }
    prun
}

pub(crate) static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    let mut cubies1 = [0; 7];
    let mut move_perm = vec![[0; N_MOVES]; N_PERM];
    for (perm, row) in move_perm.iter_mut().enumerate() {
        unpack_perm(perm as i32, &mut cubies1);
        for (mv, v) in row.iter_mut().enumerate() {
            let mut cubies2 = cubies1;
            move_cubies(&mut cubies2, mv);
            *v = pack_perm(&cubies2);
        }
    }
    let mut move_orient = vec![[0; N_MOVES]; N_ORIENT];
    for (orient, row) in move_orient.iter_mut().enumerate() {
        unpack_orient(orient as i32, &mut cubies1);
        for (mv, v) in row.iter_mut().enumerate() {
            let mut cubies2 = cubies1;
            move_cubies(&mut cubies2, mv);
            *v = pack_orient(&cubies2);
        }
    }
    let prun_perm = bfs(N_PERM, &move_perm);
    let prun_orient = bfs(N_ORIENT, &move_orient);
    Tables {
        move_perm,
        move_orient,
        prun_perm,
        prun_orient,
    }
});

/// The move tables, exposed for tests: `(move_perm, move_orient)`.
pub fn move_tables() -> (&'static [[i32; 9]], &'static [[i32; 9]]) {
    (&TABLES.move_perm, &TABLES.move_orient)
}

const COST_U: i32 = 8;
const COST_U_LOW: i32 = 20; // when grip = -1
const COST_U2: i32 = 10;
const COST_U3: i32 = 7;
const COST_R: i32 = 6;
const COST_R2: i32 = 10;
const COST_R3: i32 = 6;
const COST_F: i32 = 10;
const COST_F2: i32 = 30;
const COST_F3: i32 = 19;
const COST_REGRIP: i32 = 20;

/// An optimal 2x2x2 solver that prefers finger-trick friendly scrambles.
#[derive(Debug, Clone, Copy, Default)]
pub struct TwoByTwoSolver;

impl TwoByTwoSolver {
    /// Creates a solver; the tables are built on first use.
    pub const fn new() -> Self {
        Self
    }

    /// A uniformly random 2x2x2 position.
    pub fn random_state(&self, r: &mut dyn RandomSource) -> TwoByTwoState {
        let permutation = r.next_int_bounded(N_PERM as i32);
        let orientation = r.next_int_bounded(N_ORIENT as i32);
        TwoByTwoState {
            permutation,
            orientation,
        }
    }

    /// An optimal solution of at most `length` moves.
    pub fn solve_in(&self, state: TwoByTwoState, length: i32) -> Option<String> {
        Self::solve(state, length, false, false)
    }

    /// The easiest-to-execute scramble of exactly `length` moves that generates `state`.
    pub fn generate_exactly(&self, state: TwoByTwoState, length: i32) -> Option<String> {
        Self::solve(state, length, true, true)
    }

    fn solve(
        state: TwoByTwoState,
        desired_length: i32,
        exact_length: bool,
        inverse: bool,
    ) -> Option<String> {
        let mut solution = [0_usize; MAX_LENGTH];
        let mut best_solution = [0_i32; MAX_LENGTH + 1];
        let mut length = if exact_length { desired_length } else { 0 };
        let mut found = false;
        while length <= desired_length {
            best_solution[length as usize] = 42_424_242;
            if search(
                state.permutation,
                state.orientation,
                0,
                length,
                42,
                &mut solution,
                &mut best_solution,
            ) {
                found = true;
                break;
            }
            length += 1;
        }
        if !found {
            return None;
        }
        let length = length as usize;
        let names = if inverse {
            &INVERSE_MOVE_TO_STRING
        } else {
            &MOVE_TO_STRING
        };
        let mut moves: Vec<&str> = best_solution[..length]
            .iter()
            .map(|&m| names[m as usize])
            .collect();
        if inverse {
            moves.reverse();
        }
        Some(moves.join(" "))
    }
}

/// Depth-first search for solutions of exactly `length` more moves; records the cheapest
/// one (according to [`compute_cost`]) in `best_solution`.
fn search(
    perm: i32,
    orient: i32,
    depth: usize,
    length: i32,
    last_move: usize,
    solution: &mut [usize; MAX_LENGTH],
    best_solution: &mut [i32; MAX_LENGTH + 1],
) -> bool {
    let t = &*TABLES;
    if length == 0 {
        if perm == 0 && orient == 0 {
            // Found a solution: compute the cost of applying the reverse solution. Like the
            // Java code this starts one past the last move, at a slot that is always 0 (U'),
            // which adds the same constant to every candidate.
            let cost = compute_cost(solution, depth as i32, 0, 0);
            if cost < best_solution[depth] {
                for (b, &s) in best_solution.iter_mut().zip(&solution[..depth]) {
                    *b = s as i32;
                }
                best_solution[depth] = cost;
            }
            return true;
        }
        return false;
    }
    if t.prun_perm[perm as usize] > length || t.prun_orient[orient as usize] > length {
        return false;
    }
    let mut found = false;
    for mv in 0..N_MOVES {
        if mv / 3 == last_move / 3 {
            continue;
        }
        solution[depth] = mv;
        found |= search(
            t.move_perm[perm as usize][mv],
            t.move_orient[orient as usize][mv],
            depth + 1,
            length - 1,
            mv,
            solution,
            best_solution,
        );
    }
    found
}

/// The cost of executing the inverse of `solution[..=index]` (read backwards), given the
/// grip of the right hand: -1 thumb on D, 0 thumb on F, 1 thumb on U.
fn compute_cost(solution: &[usize; MAX_LENGTH], index: i32, current_cost: i32, grip: i32) -> i32 {
    if index < 0 {
        return current_cost;
    }
    let next = |cost: i32, grip: i32| compute_cost(solution, index - 1, current_cost + cost, grip);
    match solution[index as usize] {
        // U'
        0 => next(COST_U3, grip),
        // U2
        1 => next(COST_U2, grip),
        // U
        2 => match grip {
            0 => next(COST_U, 0),
            -1 => next(COST_REGRIP + COST_U, 0).min(next(COST_U_LOW, grip)),
            _ => next(COST_REGRIP + COST_U, 0),
        },
        // R'
        3 => {
            if grip > -1 {
                next(COST_R3, grip - 1)
            } else {
                next(COST_REGRIP + COST_R3, -1)
            }
        }
        // R2
        4 => {
            if grip != 0 {
                next(COST_R2, -grip)
            } else {
                next(COST_REGRIP + COST_R2, -1).min(next(COST_REGRIP + COST_R2, 1))
            }
        }
        // R
        5 => {
            if grip < 1 {
                next(COST_R, grip + 1)
            } else {
                next(COST_REGRIP + COST_R, 1)
            }
        }
        // F'
        6 => {
            if grip != 0 {
                next(COST_F3, grip)
            } else {
                next(COST_REGRIP + COST_F3, -1).min(next(COST_REGRIP + COST_F3, 1))
            }
        }
        // F2
        7 => {
            if grip == -1 {
                next(COST_F2, -1)
            } else {
                next(COST_REGRIP + COST_F2, -1)
            }
        }
        // F
        8 => {
            if grip == -1 {
                next(COST_F, -1)
            } else {
                next(COST_REGRIP + COST_F, -1)
            }
        }
        _ => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_round_trip() {
        let mut c = [0; 7];
        for p in 0..N_PERM as i32 {
            unpack_perm(p, &mut c);
            assert_eq!(pack_perm(&c), p);
        }
        for o in 0..N_ORIENT as i32 {
            unpack_orient(o, &mut c);
            assert_eq!(pack_orient(&c), o);
        }
    }

    #[test]
    fn solves_single_moves() {
        let solver = TwoByTwoSolver::new();
        let (perm, orient) = move_tables();
        for mv in 0..9 {
            let state = TwoByTwoState {
                permutation: perm[0][mv],
                orientation: orient[0][mv],
            };
            assert_eq!(
                solver.solve_in(state, 1).as_deref(),
                Some(INVERSE_MOVE_TO_STRING[mv])
            );
            assert_eq!(solver.solve_in(state, 0), None);
        }
        assert_eq!(
            solver.solve_in(TwoByTwoState::default(), 3).as_deref(),
            Some("")
        );
    }
}
