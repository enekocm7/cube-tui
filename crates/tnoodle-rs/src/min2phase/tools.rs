//! Random cube generation and facelet helpers (`cs.min2phase.Tools`).

use super::coord_cube::TABLES;
use super::cubie_cube::CubieCube;
use super::search::Search;
use super::util;
use crate::java::RandomSource;

/// How a set of pieces (corner/edge permutation or orientation) should be randomised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PieceSpec {
    /// Every piece is random (`STATE_RANDOM`).
    Random,
    /// Every piece is solved (`STATE_SOLVED`).
    Solved,
    /// Fixed values, with `-1` marking the pieces to randomise.
    Partial(Vec<i8>),
}

fn resolve_ori(arr: &mut [i8], base: i32, gen_: &mut dyn RandomSource) -> i32 {
    let mut sum = 0_i32;
    let mut last_unknown = None;
    for (i, v) in arr.iter_mut().enumerate() {
        if *v == -1 {
            *v = gen_.next_int_bounded(base) as i8;
            last_unknown = Some(i);
        }
        sum += i32::from(*v);
    }
    if sum % base != 0
        && let Some(last) = last_unknown
    {
        arr[last] = ((30 + i32::from(arr[last]) - sum) % base) as i8;
    }
    let mut idx = 0;
    for &v in &arr[..arr.len() - 1] {
        idx *= base;
        idx += i32::from(v);
    }
    idx
}

fn count_unknown(spec: &PieceSpec) -> usize {
    match spec {
        PieceSpec::Random | PieceSpec::Solved => 0,
        PieceSpec::Partial(arr) => arr.iter().filter(|&&v| v == -1).count(),
    }
}

/// `Tools.getNPerm`: the Lehmer index of a permutation of `n` values.
fn get_n_perm(arr: &[i8], n: usize) -> i32 {
    let mut idx = 0;
    for i in 0..n {
        idx *= (n - i) as i32;
        for j in i + 1..n {
            if arr[j] < arr[i] {
                idx += 1;
            }
        }
    }
    idx
}

fn resolve_perm(
    spec: &mut PieceSpec,
    mut cnt_u: usize,
    parity: i32,
    gen_: &mut dyn RandomSource,
) -> i32 {
    let arr = match spec {
        PieceSpec::Solved => return 0,
        PieceSpec::Random => {
            return if parity == -1 {
                gen_.next_int_bounded(2)
            } else {
                parity
            };
        }
        PieceSpec::Partial(arr) => arr,
    };
    let mut val: [i8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    for &v in arr.iter() {
        if v != -1 {
            val[v as usize] = -1;
        }
    }
    let mut idx = 0;
    for i in 0..arr.len() {
        if val[i] != -1 {
            let j = gen_.next_int_bounded(idx as i32 + 1) as usize;
            let temp = val[i];
            val[idx] = val[j];
            idx += 1;
            val[j] = temp;
        }
    }
    let mut last = None;
    idx = 0;
    while idx < arr.len() && cnt_u > 0 {
        if arr[idx] == -1 {
            if cnt_u == 2 {
                last = Some(idx);
            }
            cnt_u -= 1;
            arr[idx] = val[cnt_u];
        }
        idx += 1;
    }
    let p = util::get_n_parity(get_n_perm(arr, arr.len()), arr.len() as i32);
    if p == 1 - parity
        && let Some(last) = last
    {
        arr.swap(idx - 1, last);
    }
    p
}

/// `Tools.randomState`: a random facelet string respecting the given constraints.
pub fn random_state(
    mut cp: PieceSpec,
    co: PieceSpec,
    mut ep: PieceSpec,
    eo: PieceSpec,
    gen_: &mut dyn RandomSource,
) -> String {
    let cnt_ue = if ep == PieceSpec::Random {
        12
    } else {
        count_unknown(&ep)
    };
    let cnt_uc = if cp == PieceSpec::Random {
        8
    } else {
        count_unknown(&cp)
    };
    let parity;
    let cp_val;
    let ep_val;
    if cnt_ue < 2 {
        if ep == PieceSpec::Solved {
            ep_val = 0;
            parity = 0;
        } else {
            parity = resolve_perm(&mut ep, cnt_ue, -1, gen_);
            ep_val = partial_perm(&ep, 12);
        }
        cp_val = match cp {
            PieceSpec::Solved => 0,
            PieceSpec::Random => loop {
                let v = gen_.next_int_bounded(40320);
                if util::get_n_parity(v, 8) == parity {
                    break v;
                }
            },
            PieceSpec::Partial(_) => {
                resolve_perm(&mut cp, cnt_uc, parity, gen_);
                partial_perm(&cp, 8)
            }
        };
    } else {
        match cp {
            PieceSpec::Solved => {
                cp_val = 0;
                parity = 0;
            }
            PieceSpec::Random => {
                cp_val = gen_.next_int_bounded(40320);
                parity = util::get_n_parity(cp_val, 8);
            }
            PieceSpec::Partial(_) => {
                parity = resolve_perm(&mut cp, cnt_uc, -1, gen_);
                cp_val = partial_perm(&cp, 8);
            }
        }
        ep_val = if ep == PieceSpec::Random {
            loop {
                let v = gen_.next_int_bounded(479_001_600);
                if util::get_n_parity(v, 12) == parity {
                    break v;
                }
            }
        } else {
            resolve_perm(&mut ep, cnt_ue, parity, gen_);
            partial_perm(&ep, 12)
        };
    }
    let co_val = match co {
        PieceSpec::Random => gen_.next_int_bounded(2187),
        PieceSpec::Solved => 0,
        PieceSpec::Partial(mut arr) => resolve_ori(&mut arr, 3, gen_),
    };
    let eo_val = match eo {
        PieceSpec::Random => gen_.next_int_bounded(2048),
        PieceSpec::Solved => 0,
        PieceSpec::Partial(mut arr) => resolve_ori(&mut arr, 2, gen_),
    };
    util::to_face_cube(&CubieCube::from_coords(cp_val, co_val, ep_val, eo_val))
}

fn partial_perm(spec: &PieceSpec, n: usize) -> i32 {
    match spec {
        PieceSpec::Partial(arr) => get_n_perm(arr, n),
        _ => 0,
    }
}

/// A uniformly random cube (`Tools.randomCube(Random)`).
pub fn random_cube(gen_: &mut dyn RandomSource) -> String {
    random_state(
        PieceSpec::Random,
        PieceSpec::Random,
        PieceSpec::Random,
        PieceSpec::Random,
        gen_,
    )
}

fn partial(values: &[i8]) -> PieceSpec {
    PieceSpec::Partial(values.to_vec())
}

/// A random last layer (`Tools.randomLastLayer`).
pub fn random_last_layer(gen_: &mut dyn RandomSource) -> String {
    random_state(
        partial(&[-1, -1, -1, -1, 4, 5, 6, 7]),
        partial(&[-1, -1, -1, -1, 0, 0, 0, 0]),
        partial(&[-1, -1, -1, -1, 4, 5, 6, 7, 8, 9, 10, 11]),
        partial(&[-1, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0]),
        gen_,
    )
}

/// A random last slot and last layer (`Tools.randomLastSlot`).
pub fn random_last_slot(gen_: &mut dyn RandomSource) -> String {
    random_state(
        partial(&[-1, -1, -1, -1, -1, 5, 6, 7]),
        partial(&[-1, -1, -1, -1, -1, 0, 0, 0]),
        partial(&[-1, -1, -1, -1, 4, 5, 6, 7, -1, 9, 10, 11]),
        partial(&[-1, -1, -1, -1, 0, 0, 0, 0, -1, 0, 0, 0]),
        gen_,
    )
}

/// A random ZBLL case (`Tools.randomZBLastLayer`).
pub fn random_zb_last_layer(gen_: &mut dyn RandomSource) -> String {
    random_state(
        partial(&[-1, -1, -1, -1, 4, 5, 6, 7]),
        partial(&[-1, -1, -1, -1, 0, 0, 0, 0]),
        partial(&[-1, -1, -1, -1, 4, 5, 6, 7, 8, 9, 10, 11]),
        PieceSpec::Solved,
        gen_,
    )
}

/// A random last layer with solved edges (`Tools.randomCornerOfLastLayer`).
pub fn random_corner_of_last_layer(gen_: &mut dyn RandomSource) -> String {
    random_state(
        partial(&[-1, -1, -1, -1, 4, 5, 6, 7]),
        partial(&[-1, -1, -1, -1, 0, 0, 0, 0]),
        PieceSpec::Solved,
        PieceSpec::Solved,
        gen_,
    )
}

/// A random last layer with solved corners (`Tools.randomEdgeOfLastLayer`).
pub fn random_edge_of_last_layer(gen_: &mut dyn RandomSource) -> String {
    random_state(
        PieceSpec::Solved,
        PieceSpec::Solved,
        partial(&[-1, -1, -1, -1, 4, 5, 6, 7, 8, 9, 10, 11]),
        partial(&[-1, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0]),
        gen_,
    )
}

/// A random cube with a solved D cross (`Tools.randomCrossSolved`).
pub fn random_cross_solved(gen_: &mut dyn RandomSource) -> String {
    random_state(
        PieceSpec::Random,
        PieceSpec::Random,
        partial(&[-1, -1, -1, -1, 4, 5, 6, 7, -1, -1, -1, -1]),
        partial(&[-1, -1, -1, -1, 0, 0, 0, 0, -1, -1, -1, -1]),
        gen_,
    )
}

/// A random cube with solved edges (`Tools.randomEdgeSolved`).
pub fn random_edge_solved(gen_: &mut dyn RandomSource) -> String {
    random_state(
        PieceSpec::Random,
        PieceSpec::Random,
        PieceSpec::Solved,
        PieceSpec::Solved,
        gen_,
    )
}

/// A random cube with solved corners (`Tools.randomCornerSolved`).
pub fn random_corner_solved(gen_: &mut dyn RandomSource) -> String {
    random_state(
        PieceSpec::Solved,
        PieceSpec::Solved,
        PieceSpec::Random,
        PieceSpec::Random,
        gen_,
    )
}

/// The superflip (`Tools.superFlip`).
pub fn super_flip() -> String {
    util::to_face_cube(&CubieCube::from_coords(0, 0, 0, 2047))
}

/// Applies a sequence of moves (0..18, `U U2 U' R ...`) to a solved cube.
pub fn from_moves(scramble: &[usize]) -> String {
    let t = &*TABLES;
    let mut c1 = CubieCube::SOLVED;
    let mut c2 = CubieCube::SOLVED;
    for &m in scramble {
        CubieCube::mult(&c1, &t.sym.move_cube[m], &mut c2);
        std::mem::swap(&mut c1, &mut c2);
    }
    util::to_face_cube(&c1)
}

/// Converts a scramble of outer face turns (`U`, `R2`, `F'`, ...) into facelets.
///
/// Like Java, unknown characters are skipped, so `"x R"` is read as `"R"`.
pub fn from_scramble(s: &str) -> String {
    let mut moves = Vec::new();
    let mut axis: i32 = -1;
    for c in s.encode_utf16() {
        match c {
            0x55 => axis = 0,  // U
            0x52 => axis = 3,  // R
            0x46 => axis = 6,  // F
            0x44 => axis = 9,  // D
            0x4C => axis = 12, // L
            0x42 => axis = 15, // B
            0x20 => {
                if axis != -1 {
                    moves.push(axis as usize);
                }
                axis = -1;
            }
            0x32 => axis += 1, // 2
            0x27 => axis += 2, // '
            _ => {}
        }
    }
    if axis != -1 {
        moves.push(axis as usize);
    }
    from_moves(&moves)
}

/// Checks whether a facelet string describes a solvable cube; `0` means solvable,
/// otherwise the negated min2phase error code.
pub fn verify(facelets: &str) -> i32 {
    Search::new().verify(facelets)
}
