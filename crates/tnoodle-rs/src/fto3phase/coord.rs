//! Move and pruning tables of the FTO solver (`levigibson.fto3phase.FtoCoord`).

use std::sync::LazyLock;

use super::cubie::FtoCubie;
use super::search::{G1_MOVESET, G2_MOVESET, G3_MOVESET};
use super::util::{fact, n_cr, pow};

pub(crate) const ALL_EDGES_SIZE: i32 = fact(12) / 2;
pub(crate) const ALL_CORNER_PERMUTATION_SIZE: i32 = fact(6) / 2;
pub(crate) const ALL_CORNER_ORIENTATION_SIZE: i32 = pow(2, 5);
pub(crate) const ALL_TRIANGLE_SIZE: i32 = n_cr(12, 3) * n_cr(9, 3) * n_cr(6, 3);
pub(crate) const G1_TRIANGLES_SIZE: i32 = n_cr(12, 3);
pub(crate) const G1_EDGES_SIZE: i32 = n_cr(12, 3) * 2;
pub(crate) const G2_TRIANGLES_SIZE: i32 = n_cr(9, 3) * n_cr(6, 3);
pub(crate) const G2_EDGES_SIZE: i32 = n_cr(9, 3) * n_cr(6, 3) * 2 * 2;
pub(crate) const G2_TRIPLE_CORNER_SIZE: i32 = n_cr(6, 3) * pow(2, 3);
pub(crate) const G2_TRIPLE_TRIANGLE_SIZE: i32 = n_cr(12, 3);
pub(crate) const G2_TRIPLE_SIZE: i32 = G2_TRIPLE_CORNER_SIZE * G2_TRIPLE_TRIANGLE_SIZE;
pub(crate) const G3_CORNERS_SIZE: i32 = (fact(6) / 2) * pow(2, 5);
pub(crate) const G3_EDGE_SIZE: i32 = pow(3, 4);

/// Every FTO solver table.
pub(crate) struct Tables {
    pub(crate) g1_edge_moves: Vec<[i32; 16]>,
    pub(crate) g1_triangle_moves: Vec<[i32; 16]>,
    pub(crate) g2_triangle_moves: Vec<[i32; 10]>,
    pub(crate) g2_edge_moves: Vec<[i32; 10]>,
    pub(crate) g2_triple_moves: Vec<[i32; 10]>,
    pub(crate) g3_edge_moves: Vec<[i32; 10]>,
    pub(crate) g3_corner_moves: Vec<[i32; 10]>,
    pub(crate) g1_prun: Vec<i8>,
    pub(crate) g2_triple_prun: Vec<i8>,
    pub(crate) g2_txe_prun: Vec<i8>,
    pub(crate) g3_corner_prun: Vec<i8>,
    pub(crate) solved_g3_edges: i32,
    pub(crate) solved_g3_corners: i32,
}

pub(crate) static TABLES: LazyLock<Tables> = LazyLock::new(Tables::build);

/// Builds a move table: `set` puts index `idx` into a cubie, `pack` reads it back.
fn move_table<const N: usize>(
    size: i32,
    moveset: &[usize],
    set: impl Fn(&mut FtoCubie, i32),
    pack: impl Fn(&FtoCubie) -> i32,
) -> Vec<[i32; N]> {
    let mut table = vec![[0; N]; size as usize];
    let mut fto = FtoCubie::new();
    for (idx, row) in table.iter_mut().enumerate() {
        set(&mut fto, idx as i32);
        for &m in moveset {
            row[m] = pack(&fto.turned(m));
        }
    }
    table
}

/// Breadth first search from `start`, recording each state's depth in a new table.
fn bfs(
    size: usize,
    start: &[i32],
    moveset: &[usize],
    turn: impl Fn(usize, usize) -> usize,
) -> Vec<i8> {
    let mut prun = vec![-1_i8; size];
    for &s in start {
        prun[s as usize] = 0;
    }
    let mut frontier: Vec<usize> = start.iter().map(|&s| s as usize).collect();
    let mut depth = 0;
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for &idx in &frontier {
            for &m in moveset {
                let n = turn(idx, m);
                if prun[n] == -1 {
                    prun[n] = (depth + 1) as i8;
                    next.push(n);
                }
            }
        }
        frontier = next;
        depth += 1;
    }
    prun
}

impl Tables {
    fn build() -> Self {
        let g1_edge_moves = move_table::<16>(
            G1_EDGES_SIZE,
            &G1_MOVESET,
            FtoCubie::g1_set_edges,
            FtoCubie::g1_pack_edges,
        );
        let g1_triangle_moves = move_table::<16>(
            G1_TRIANGLES_SIZE,
            &G1_MOVESET,
            FtoCubie::g1_set_triangles,
            FtoCubie::g1_pack_triangles,
        );
        let solved = FtoCubie::new();
        let g1_size = G1_TRIANGLES_SIZE as usize;
        let g1_prun = bfs(
            G1_EDGES_SIZE as usize * g1_size,
            &[solved.g1_pack_edges() * G1_TRIANGLES_SIZE + solved.g1_pack_triangles()],
            &G1_MOVESET,
            |idx, m| {
                let edge = g1_edge_moves[idx / g1_size][m] as usize;
                let tri = g1_triangle_moves[idx % g1_size][m] as usize;
                edge * g1_size + tri
            },
        );

        let g2_triangle_moves = move_table::<10>(
            G2_TRIANGLES_SIZE,
            &G2_MOVESET,
            FtoCubie::g2_set_triangles,
            FtoCubie::g2_pack_triangles,
        );
        let g2_edge_moves = move_table::<10>(
            G2_EDGES_SIZE,
            &G2_MOVESET,
            FtoCubie::g2_set_edges,
            FtoCubie::g2_pack_edges,
        );
        let g2_triple_moves = move_table::<10>(
            G2_TRIPLE_SIZE,
            &G2_MOVESET,
            |c, idx| c.g2_set_triples(idx, 0),
            |c| c.g2_pack_triples(0),
        );

        let g2_size = G2_TRIANGLES_SIZE as usize;
        let g2_txe_prun = bfs(
            G2_EDGES_SIZE as usize * g2_size,
            &[solved.g2_pack_edges() * G2_TRIANGLES_SIZE + solved.g2_pack_triangles()],
            &G2_MOVESET,
            |idx, m| {
                let edge = g2_edge_moves[idx / g2_size][m] as usize;
                let tri = g2_triangle_moves[idx % g2_size][m] as usize;
                edge * g2_size + tri
            },
        );

        let triple_frontier = Self::g2_triple_frontier(&g2_triple_moves, &solved);
        let g2_triple_prun = bfs(
            G2_TRIPLE_SIZE as usize,
            &triple_frontier,
            &G2_MOVESET,
            |idx, m| g2_triple_moves[idx][m] as usize,
        );

        let g3_edge_moves = move_table::<10>(
            G3_EDGE_SIZE,
            &G3_MOVESET,
            FtoCubie::g3_set_edges,
            FtoCubie::g3_pack_edges,
        );
        let g3_corner_moves = move_table::<10>(
            G3_CORNERS_SIZE,
            &G3_MOVESET,
            FtoCubie::g3_set_corners,
            FtoCubie::g3_pack_corners,
        );
        let g3_corner_prun = bfs(
            G3_CORNERS_SIZE as usize,
            &[solved.g3_pack_corners()],
            &G3_MOVESET,
            |idx, m| g3_corner_moves[idx][m] as usize,
        );

        Self {
            g1_edge_moves,
            g1_triangle_moves,
            g2_triangle_moves,
            g2_edge_moves,
            g2_triple_moves,
            g3_edge_moves,
            g3_corner_moves,
            g1_prun,
            g2_triple_prun,
            g2_txe_prun,
            g3_corner_prun,
            solved_g3_edges: solved.g3_pack_edges(),
            solved_g3_corners: solved.g3_pack_corners(),
        }
    }

    /// All the triple states (of one colour) that are solved up to phase 3 moves
    /// (`g2GenerateTripleFrontier`). Like the Java code, the start state is not marked as
    /// visited, so it can appear more than once; this does not change the pruning table.
    fn g2_triple_frontier(g2_triple_moves: &[[i32; 10]], solved: &FtoCubie) -> Vec<i32> {
        let mut prun = vec![-1_i8; G2_TRIPLE_SIZE as usize];
        let mut all = Vec::with_capacity(161);
        let mut frontier = vec![solved.g2_pack_triples(0)];
        while !frontier.is_empty() {
            all.extend_from_slice(&frontier);
            let mut next = Vec::new();
            for &idx in &frontier {
                for &m in &G3_MOVESET {
                    let n = g2_triple_moves[idx as usize][m];
                    if prun[n as usize] == -1 {
                        prun[n as usize] = 1;
                        next.push(n);
                    }
                }
            }
            frontier = next;
        }
        all
    }

    pub(crate) fn g1_prun(&self, edge: i32, tris: i32) -> i32 {
        i32::from(self.g1_prun[(edge * G1_TRIANGLES_SIZE + tris) as usize])
    }

    pub(crate) fn g2_prun_triple(&self, idx: [i32; 4]) -> i32 {
        idx.iter()
            .map(|&i| i32::from(self.g2_triple_prun[i as usize]))
            .fold(0, i32::max)
    }

    pub(crate) fn g2_prun_txe(&self, edge: i32, tris: i32) -> i32 {
        i32::from(self.g2_txe_prun[(edge * G2_TRIANGLES_SIZE + tris) as usize])
    }

    pub(crate) fn g3_prun_corners(&self, corners: i32) -> i32 {
        i32::from(self.g3_corner_prun[corners as usize])
    }
}
