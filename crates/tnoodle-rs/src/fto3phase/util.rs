//! Permutation and subset indexing helpers (`levigibson.fto3phase.Util`).

use super::cubie::FtoCubie;
use super::cubie::moves::{B, BL, BLP, BP, BR, BRP, D, DP, F, FP, L, LP, R, RP, U, UP};

const FACTORIAL: [i32; 13] = [
    1,
    1,
    2,
    6,
    24,
    120,
    720,
    5040,
    40320,
    362_880,
    3_628_800,
    39_916_800,
    479_001_600,
];

const CNK: [[i32; 13]; 13] = {
    let mut c = [[0; 13]; 13];
    let mut n = 0;
    while n < 13 {
        c[n][0] = 1;
        let mut k = 1;
        while k <= n {
            c[n][k] = c[n - 1][k - 1] + c[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    c
};

/// `n choose k` for `n <= 12`.
pub(crate) const fn n_cr(n: usize, k: usize) -> i32 {
    CNK[n][k]
}

/// `n!` for `n <= 12`.
pub(crate) const fn fact(n: usize) -> i32 {
    FACTORIAL[n]
}

/// `a^b` for small non-negative integers.
pub(crate) const fn pow(a: i32, b: u32) -> i32 {
    a.pow(b)
}

/// Whether a permutation of `0..n` is odd.
///
/// # Panics
///
/// Panics if `perm` is not a permutation of `0..n`, like Java throws
/// `IllegalArgumentException`.
pub(crate) fn is_parity(perm: &[i32]) -> bool {
    let mut perm = perm.to_vec();
    let mut swaps = 0;
    for i in 0..perm.len() {
        while perm[i] != i as i32 {
            let target = perm[i];
            assert!(
                (0..perm.len() as i32).contains(&target),
                "perm is not a sequential array {perm:?}"
            );
            perm.swap(i, target as usize);
            swaps += 1;
        }
    }
    swaps % 2 == 1
}

/// The Lehmer index of the first `size` elements of `arr`, halved if `parity`.
pub(crate) fn pack_perm(arr: &[i32], parity: bool, size: usize) -> i32 {
    let size = size as i32;
    let mut index = 0;
    let mut seen = 0_i32;
    for i in 0..size {
        let e = arr[i as usize];
        seen |= 1 << ((size - 1) - e);
        let lehmer_digit = e - (seen >> (size - e)).count_ones() as i32;
        index += FACTORIAL[((size - 1) - i) as usize] * lehmer_digit;
    }
    if parity { index / 2 } else { index }
}

/// Inverse of [`pack_perm`]: fills the first `size` elements of `arr`.
pub(crate) fn unpack_perm(arr: &mut [i32], mut idx: i32, size: usize, parity: bool) {
    if parity {
        idx *= 2;
    }
    let mut used = [false; 12];
    for i in 0..size {
        let f = FACTORIAL[(size - 1) - i];
        let lehmer_digit = idx / f;
        idx %= f;
        let mut count = 0;
        let mut e = usize::MAX;
        for (v, &u) in used.iter().enumerate().take(size) {
            if !u {
                if count == lehmer_digit {
                    e = v;
                    break;
                }
                count += 1;
            }
        }
        arr[i] = e as i32;
        used[e] = true;
    }
    if parity && is_parity(&arr[..size]) {
        arr.swap(size - 2, size - 1);
    }
}

/// The colex index of a strictly increasing set of positions.
///
/// # Panics
///
/// Panics if `idx` is not strictly increasing.
pub(crate) fn pack_subset(idx: &[i32]) -> i32 {
    assert!(
        idx.windows(2).all(|w| w[0] < w[1]),
        "idx must be in ascending order"
    );
    let mut index = 0;
    for i in (0..idx.len()).rev() {
        index += n_cr(idx[i] as usize, i + 1);
    }
    index
}

/// Inverse of [`pack_subset`].
pub(crate) fn unpack_subset(arr: &mut [i32], idx: i32) {
    let subset_size = arr.len();
    let mut k = subset_size;
    let mut remaining = idx;
    for pos in 0..subset_size {
        let mut c = k - 1;
        while n_cr(c + 1, k) <= remaining {
            c += 1;
        }
        arr[subset_size - 1 - pos] = c as i32;
        remaining -= n_cr(c, k);
        k -= 1;
    }
}

const FACE_NAMES: [&str; 8] = ["R", "L", "B", "U", "D", "F", "BR", "BL"];
const CW_MOVES: [usize; 8] = [R, L, B, U, D, F, BR, BL];
const CCW_MOVES: [usize; 8] = [RP, LP, BP, UP, DP, FP, BRP, BLP];

fn parse_move(s: &str) -> Option<usize> {
    let (face, prime) = s.strip_suffix('\'').map_or((s, false), |f| (f, true));
    let i = FACE_NAMES.iter().position(|&n| n == face)?;
    Some(if prime { CCW_MOVES[i] } else { CW_MOVES[i] })
}

/// The cubie obtained by applying a move string such as `"R U' BL"` (`fromAlg`).
///
/// # Panics
///
/// Panics on an unrecognised move, like Java throws `IllegalArgumentException`.
pub(crate) fn from_alg(alg: &str) -> FtoCubie {
    let mut fto = FtoCubie::new();
    for token in alg.split_whitespace() {
        let m = parse_move(token).unwrap_or_else(|| panic!("Unrecognized move: {token}"));
        fto.turn(m);
    }
    fto
}

const INVERSE_MOVE_NAMES: [&str; 16] = [
    "R'", "R", "L'", "L", "B'", "B", "U'", "U", "D'", "D", "F'", "F", "BR'", "BR", "BL'", "BL",
];

/// Inverts a sequence of moves and renders it, turning a solution into a scramble.
pub(crate) fn move_array_to_inverted_string(moves: &[usize]) -> String {
    moves
        .iter()
        .rev()
        .map(|&m| INVERSE_MOVE_NAMES[m])
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsets_round_trip() {
        for idx in 0..n_cr(12, 3) {
            let mut arr = [0; 3];
            unpack_subset(&mut arr, idx);
            assert_eq!(pack_subset(&arr), idx);
        }
    }

    #[test]
    fn permutations_round_trip() {
        for parity in [false, true] {
            let n = if parity { 360 } else { 720 };
            for idx in 0..n {
                let mut arr = [0; 6];
                unpack_perm(&mut arr, idx, 6, parity);
                assert_eq!(pack_perm(&arr, parity, 6), idx);
                if parity {
                    assert!(!is_parity(&arr));
                }
            }
        }
    }

    #[test]
    #[should_panic(expected = "ascending")]
    fn pack_subset_rejects_unsorted() {
        pack_subset(&[2, 1, 3]);
    }

    #[test]
    fn helpers() {
        assert_eq!(fact(5), 120);
        assert_eq!(pow(2, 5), 32);
        assert_eq!(parse_move("BR'"), Some(BRP));
        assert_eq!(parse_move("X"), None);
        assert_eq!(move_array_to_inverted_string(&[R, UP]), "U R'");
    }
}
