//! The three phase FTO search (`levigibson.fto3phase.Search`).

use std::sync::LazyLock;

use super::coord::{TABLES, Tables};
use super::cubie::FtoCubie;
use super::cubie::moves::{B, BL, BLP, BP, BR, BRP, D, DP, F, FP, L, LP, R, RP, U, UP};
use super::util::{from_alg, move_array_to_inverted_string};

/// The moves allowed in phase 1 (all of them).
pub const G1_MOVESET: [usize; 16] = [R, RP, L, LP, B, BP, U, UP, D, DP, F, FP, BR, BRP, BL, BLP];
/// The moves allowed in phase 2.
pub const G2_MOVESET: [usize; 10] = [R, RP, L, LP, B, BP, U, UP, D, DP];
/// The moves allowed in phase 3.
pub const G3_MOVESET: [usize; 8] = [R, RP, L, LP, B, BP, D, DP];

const MIN_G1_CANDIDATES: usize = 500;

/// For each axis, the axes that must not follow it (the same axis, and commuting axes in
/// one canonical order).
static INVALID_MOVES: LazyLock<[i32; 8]> = LazyLock::new(|| {
    let mut invalid = [0; 8];
    for a1 in 0..8 {
        invalid[a1] = 1 << a1;
        for a2 in 0..a1 {
            let fto1 = FtoCubie::new().turned(a1 * 2).turned(a2 * 2);
            let fto2 = FtoCubie::new().turned(a2 * 2).turned(a1 * 2);
            if fto1 == fto2 {
                invalid[a1] |= 1 << a2;
            }
        }
    }
    invalid
});

fn is_valid_move(last_move: usize, mv: usize) -> bool {
    ((INVALID_MOVES[last_move / 2] >> (mv / 2)) & 1) != 1
}

/// Levi Gibson's three phase FTO solver.
#[derive(Clone, Copy)]
pub struct Search {
    t: &'static Tables,
}

impl std::fmt::Debug for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Search").finish_non_exhaustive()
    }
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

impl Search {
    /// A new searcher; the lookup tables are built on first use.
    pub fn new() -> Self {
        Self { t: &TABLES }
    }

    /// Builds the lookup tables now instead of on the first solve.
    pub fn init() {
        let _ = &*TABLES;
    }

    /// A scramble that generates `random_state` from a solved FTO.
    ///
    /// # Panics
    ///
    /// Panics if the solution fails its self check (the Java code throws).
    pub fn solution(self, random_state: &FtoCubie) -> String {
        let candidates = self.g1_iterate(random_state);
        let g1_and_g2 = self.g2_iterate(random_state, &candidates);
        let g2_fto = random_state.with_moves(&g1_and_g2);
        let g3 = self.g3_iterate(&g2_fto);
        let full: Vec<usize> = g1_and_g2.into_iter().chain(g3).collect();
        let full_str = move_array_to_inverted_string(&full);
        assert!(
            Self::validate_solution(&full_str, random_state),
            "CRITICAL: Found solution does not match random state"
        );
        full_str
    }

    /// Whether `moves` applied to a solved FTO gives `random_state`.
    pub fn validate_solution(moves: &str, random_state: &FtoCubie) -> bool {
        from_alg(moves) == *random_state
    }

    fn g1_iterate(self, cubie: &FtoCubie) -> Vec<Vec<usize>> {
        let mut candidates = Vec::new();
        let mut moves = Vec::new();
        let edge = cubie.g1_pack_edges();
        let tri = cubie.g1_pack_triangles();
        for depth in 0.. {
            self.g1_search(depth, edge, tri, &mut candidates, &mut moves);
            if candidates.len() >= MIN_G1_CANDIDATES {
                break;
            }
        }
        candidates
    }

    fn g2_iterate(self, cubie: &FtoCubie, candidates: &[Vec<usize>]) -> Vec<usize> {
        let states: Vec<[i32; 6]> = candidates
            .iter()
            .map(|moves| {
                let c = cubie.with_moves(moves);
                [
                    c.g2_pack_edges(),
                    c.g2_pack_triangles(),
                    c.g2_pack_triples(0),
                    c.g2_pack_triples(1),
                    c.g2_pack_triples(2),
                    c.g2_pack_triples(3),
                ]
            })
            .collect();
        let mut moves = Vec::new();
        for depth in 0_i32.. {
            for (c, s) in states.iter().enumerate() {
                let search_depth = depth - candidates[c].len() as i32;
                if search_depth < 0 {
                    continue;
                }
                if let Some(g2) = self.g2_search(
                    search_depth,
                    s[0],
                    s[1],
                    [s[2], s[3], s[4], s[5]],
                    &mut moves,
                ) {
                    return candidates[c].iter().copied().chain(g2).collect();
                }
            }
        }
        unreachable!("Could not find Phase 2 solution")
    }

    fn g3_iterate(self, cubie: &FtoCubie) -> Vec<usize> {
        let edges = cubie.g3_pack_edges();
        let corners = cubie.g3_pack_corners();
        let mut moves = Vec::new();
        (0..)
            .find_map(|depth| self.g3_search(depth, edges, corners, &mut moves))
            .expect("phase 3 always has a solution")
    }

    fn g1_search(
        self,
        depth: i32,
        edge: i32,
        tri: i32,
        candidates: &mut Vec<Vec<usize>>,
        moves: &mut Vec<usize>,
    ) {
        let t = self.t;
        let prun = t.g1_prun(edge, tri);
        if depth < prun {
            return;
        }
        if prun == 0 && depth == 0 {
            candidates.push(moves.clone());
            return;
        }
        if depth <= 0 {
            return;
        }
        for mv in G1_MOVESET {
            if let Some(&last) = moves.last()
                && !is_valid_move(last, mv)
            {
                continue;
            }
            moves.push(mv);
            self.g1_search(
                depth - 1,
                t.g1_edge_moves[edge as usize][mv],
                t.g1_triangle_moves[tri as usize][mv],
                candidates,
                moves,
            );
            moves.pop();
        }
    }

    fn g2_search(
        self,
        depth: i32,
        edge: i32,
        tri: i32,
        triples: [i32; 4],
        moves: &mut Vec<usize>,
    ) -> Option<Vec<usize>> {
        let t = self.t;
        let triple_prun = t.g2_prun_triple(triples);
        if depth < triple_prun {
            return None;
        }
        let txe_prun = t.g2_prun_txe(edge, tri);
        if depth < txe_prun {
            return None;
        }
        if triple_prun == 0 && txe_prun == 0 {
            return Some(moves.clone());
        }
        if depth == 0 {
            return None;
        }
        for mv in G2_MOVESET {
            if let Some(&last) = moves.last()
                && !is_valid_move(last, mv)
            {
                continue;
            }
            moves.push(mv);
            let res = self.g2_search(
                depth - 1,
                t.g2_edge_moves[edge as usize][mv],
                t.g2_triangle_moves[tri as usize][mv],
                triples.map(|tp| t.g2_triple_moves[tp as usize][mv]),
                moves,
            );
            moves.pop();
            if res.is_some() {
                return res;
            }
        }
        None
    }

    fn g3_search(
        self,
        depth: i32,
        edges: i32,
        corners: i32,
        moves: &mut Vec<usize>,
    ) -> Option<Vec<usize>> {
        let t = self.t;
        if depth < t.g3_prun_corners(corners) {
            return None;
        }
        if corners == t.solved_g3_corners && edges == t.solved_g3_edges {
            return Some(moves.clone());
        }
        if depth == 0 {
            return None;
        }
        for mv in G3_MOVESET {
            if let Some(&last) = moves.last()
                && !is_valid_move(last, mv)
            {
                continue;
            }
            moves.push(mv);
            let res = self.g3_search(
                depth - 1,
                t.g3_edge_moves[edges as usize][mv],
                t.g3_corner_moves[corners as usize][mv],
                moves,
            );
            moves.pop();
            if res.is_some() {
                return res;
            }
        }
        None
    }
}
