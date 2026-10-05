//! Facelet layout, permutation and combination coordinates shared by the min2phase solver.

use std::sync::LazyLock;

use super::cubie_cube::CubieCube;

// Facelet indices (`U1` .. `B9`).
pub(crate) const U1: u8 = 0;
pub(crate) const U2: u8 = 1;
pub(crate) const U3: u8 = 2;
pub(crate) const U4: u8 = 3;
pub(crate) const U5: u8 = 4;
pub(crate) const U6: u8 = 5;
pub(crate) const U7: u8 = 6;
pub(crate) const U8: u8 = 7;
pub(crate) const U9: u8 = 8;
pub(crate) const R1: u8 = 9;
pub(crate) const R2: u8 = 10;
pub(crate) const R3: u8 = 11;
pub(crate) const R4: u8 = 12;
pub(crate) const R5: u8 = 13;
pub(crate) const R6: u8 = 14;
pub(crate) const R7: u8 = 15;
pub(crate) const R8: u8 = 16;
pub(crate) const R9: u8 = 17;
pub(crate) const F1: u8 = 18;
pub(crate) const F2: u8 = 19;
pub(crate) const F3: u8 = 20;
pub(crate) const F4: u8 = 21;
pub(crate) const F5: u8 = 22;
pub(crate) const F6: u8 = 23;
pub(crate) const F7: u8 = 24;
pub(crate) const F8: u8 = 25;
pub(crate) const F9: u8 = 26;
pub(crate) const D1: u8 = 27;
pub(crate) const D2: u8 = 28;
pub(crate) const D3: u8 = 29;
pub(crate) const D4: u8 = 30;
pub(crate) const D5: u8 = 31;
pub(crate) const D6: u8 = 32;
pub(crate) const D7: u8 = 33;
pub(crate) const D8: u8 = 34;
pub(crate) const D9: u8 = 35;
pub(crate) const L1: u8 = 36;
pub(crate) const L2: u8 = 37;
pub(crate) const L3: u8 = 38;
pub(crate) const L4: u8 = 39;
pub(crate) const L5: u8 = 40;
pub(crate) const L6: u8 = 41;
pub(crate) const L7: u8 = 42;
pub(crate) const L8: u8 = 43;
pub(crate) const L9: u8 = 44;
pub(crate) const B1: u8 = 45;
pub(crate) const B2: u8 = 46;
pub(crate) const B3: u8 = 47;
pub(crate) const B4: u8 = 48;
pub(crate) const B5: u8 = 49;
pub(crate) const B6: u8 = 50;
pub(crate) const B7: u8 = 51;
pub(crate) const B8: u8 = 52;
pub(crate) const B9: u8 = 53;

// Face colours.
const U: u8 = 0;
const D: u8 = 3;

pub(crate) const CORNER_FACELET: [[u8; 3]; 8] = [
    [U9, R1, F3],
    [U7, F1, L3],
    [U1, L1, B3],
    [U3, B1, R3],
    [D3, F9, R7],
    [D1, L9, F7],
    [D7, B9, L7],
    [D9, R9, B7],
];

pub(crate) const EDGE_FACELET: [[u8; 2]; 12] = [
    [U6, R2],
    [U8, F2],
    [U4, L2],
    [U2, B2],
    [D6, R8],
    [D2, F8],
    [D4, L8],
    [D8, B8],
    [F6, R4],
    [F4, L6],
    [B6, L4],
    [B4, R6],
];

/// Move names padded to two characters, as used by the generic solver output.
pub(crate) const MOVE2STR: [&str; 18] = [
    "U ", "U2", "U'", "R ", "R2", "R'", "F ", "F2", "F'", "D ", "D2", "D'", "L ", "L2", "L'", "B ",
    "B2", "B'",
];

/// Maps the 10 phase 2 moves (`U U2 U' R2 F2 D D2 D' L2 B2`) followed by the 8 remaining
/// quarter turns to the standard move numbering.
pub(crate) const UD2STD: [usize; 18] =
    [0, 1, 2, 4, 7, 9, 10, 11, 13, 16, 3, 5, 6, 8, 12, 14, 15, 17];

/// Lookup tables computed once from the constants above.
pub(crate) struct UtilTables {
    pub(crate) cnk: [[i32; 13]; 13],
    pub(crate) std2ud: [usize; 18],
    pub(crate) ckmv2bit: [i32; 11],
}

pub(crate) static UTIL: LazyLock<UtilTables> = LazyLock::new(|| {
    let mut std2ud = [0; 18];
    for (i, &s) in UD2STD.iter().enumerate() {
        std2ud[s] = i;
    }
    let mut ckmv2bit = [0; 11];
    for (i, bits) in ckmv2bit.iter_mut().enumerate().take(10) {
        let ix = UD2STD[i] / 3;
        for j in 0..10 {
            let jx = UD2STD[j] / 3;
            if ix == jx || (ix % 3 == jx % 3 && ix >= jx) {
                *bits |= 1 << j;
            }
        }
    }
    let mut cnk = [[0; 13]; 13];
    for i in 0..13 {
        cnk[i][0] = 1;
        cnk[i][i] = 1;
        for j in 1..i {
            cnk[i][j] = cnk[i - 1][j - 1] + cnk[i - 1][j];
        }
    }
    UtilTables {
        cnk,
        std2ud,
        ckmv2bit,
    }
});

/// Converts a facelet colour array (values 0..6 in `URFDLB` order) into cubies.
pub(crate) fn to_cubie_cube(f: &[u8; 54], cc: &mut CubieCube) {
    cc.ca = [0; 8];
    cc.ea = [0; 12];
    for i in 0..8 {
        // Find the U/D coloured sticker of the corner at position i.
        let mut ori = 0;
        while ori < 3 {
            let col = f[CORNER_FACELET[i][ori] as usize];
            if col == U || col == D {
                break;
            }
            ori += 1;
        }
        let col1 = f[CORNER_FACELET[i][(ori + 1) % 3] as usize];
        let col2 = f[CORNER_FACELET[i][(ori + 2) % 3] as usize];
        for j in 0..8 {
            if col1 == CORNER_FACELET[j][1] / 9 && col2 == CORNER_FACELET[j][2] / 9 {
                cc.ca[i] = ((ori % 3) << 3 | j) as u8;
                break;
            }
        }
    }
    for i in 0..12 {
        for j in 0..12 {
            let a = f[EDGE_FACELET[i][0] as usize];
            let b = f[EDGE_FACELET[i][1] as usize];
            if a == EDGE_FACELET[j][0] / 9 && b == EDGE_FACELET[j][1] / 9 {
                cc.ea[i] = (j << 1) as u8;
                break;
            }
            if a == EDGE_FACELET[j][1] / 9 && b == EDGE_FACELET[j][0] / 9 {
                cc.ea[i] = (j << 1 | 1) as u8;
                break;
            }
        }
    }
}

/// Converts cubies into a 54 character facelet string.
pub(crate) fn to_face_cube(cc: &CubieCube) -> String {
    const TS: [u8; 6] = *b"URFDLB";
    let mut f = [0_u8; 54];
    for (i, c) in f.iter_mut().enumerate() {
        *c = TS[i / 9];
    }
    for c in 0..8 {
        let j = (cc.ca[c] & 0x7) as usize;
        let ori = (cc.ca[c] >> 3) as usize;
        for n in 0..3 {
            f[CORNER_FACELET[c][(n + ori) % 3] as usize] = TS[(CORNER_FACELET[j][n] / 9) as usize];
        }
    }
    for e in 0..12 {
        let j = (cc.ea[e] >> 1) as usize;
        let ori = (cc.ea[e] & 1) as usize;
        for n in 0..2 {
            f[EDGE_FACELET[e][(n + ori) % 2] as usize] = TS[(EDGE_FACELET[j][n] / 9) as usize];
        }
    }
    String::from_utf8(f.to_vec()).expect("facelets are ASCII")
}

/// The parity of the permutation with index `idx` among permutations of `n` elements.
pub(crate) fn get_n_parity(mut idx: i32, n: i32) -> i32 {
    let mut p = 0;
    let mut i = n - 2;
    while i >= 0 {
        p ^= idx % (n - i);
        idx /= n - i;
        i -= 1;
    }
    p & 1
}

fn set_val(val0: u8, val: i32, is_edge: bool) -> u8 {
    if is_edge {
        (val << 1 | i32::from(val0) & 1) as u8
    } else {
        (val | i32::from(val0) & 0xf8) as u8
    }
}

fn get_val(val0: u8, is_edge: bool) -> i32 {
    if is_edge {
        i32::from(val0 >> 1)
    } else {
        i32::from(val0 & 7)
    }
}

/// Sets the first `n` pieces of `arr` to the permutation with Lehmer index `idx`.
pub(crate) fn set_n_perm(arr: &mut [u8], mut idx: i32, n: usize, is_edge: bool) {
    let mut val: i64 = 0xFEDC_BA98_7654_3210_u64 as i64;
    let mut extract: i64 = 0;
    for p in 2..=n as i32 {
        extract = extract << 4 | i64::from(idx % p);
        idx /= p;
    }
    for item in arr.iter_mut().take(n - 1) {
        let v = ((extract as i32) & 0xf) << 2;
        extract >>= 4;
        *item = set_val(*item, (val >> v & 0xf) as i32, is_edge);
        let m = (1_i64 << v) - 1;
        val = val & m | val >> 4 & !m;
    }
    arr[n - 1] = set_val(arr[n - 1], (val & 0xf) as i32, is_edge);
}

/// The Lehmer index of the permutation of the first `n` pieces of `arr`.
pub(crate) fn get_n_perm(arr: &[u8], n: usize, is_edge: bool) -> i32 {
    let mut idx = 0_i32;
    let mut val: i64 = 0xFEDC_BA98_7654_3210_u64 as i64;
    for (i, &item) in arr.iter().enumerate().take(n - 1) {
        let v = get_val(item, is_edge) << 2;
        idx = (n - i) as i32 * idx + (val >> v & 0xf) as i32;
        val = val.wrapping_sub(0x1111_1111_1111_1110_i64 << v);
    }
    idx
}

/// The combination index of the positions of the pieces whose value matches `mask`.
pub(crate) fn get_comb(arr: &[u8], mask: i32, is_edge: bool) -> i32 {
    let cnk = &UTIL.cnk;
    let mut idx_c = 0;
    let mut r = 4;
    for i in (0..arr.len()).rev() {
        let perm = get_val(arr[i], is_edge);
        if perm & 0xc == mask {
            idx_c += cnk[i][r];
            r -= 1;
        }
    }
    idx_c
}

/// Inverse of [`get_comb`].
pub(crate) fn set_comb(arr: &mut [u8], mut idx_c: i32, mask: i32, is_edge: bool) {
    let cnk = &UTIL.cnk;
    let end = arr.len() as i32 - 1;
    let mut r = 4_usize;
    let mut fill = end;
    for i in (0..arr.len()).rev() {
        if idx_c >= cnk[i][r] {
            idx_c -= cnk[i][r];
            r -= 1;
            arr[i] = set_val(arr[i], r as i32 | mask, is_edge);
        } else {
            if fill & 0xc == mask {
                fill -= 4;
            }
            arr[i] = set_val(arr[i], fill, is_edge);
            fill -= 1;
        }
    }
}
