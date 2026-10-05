//! Move numbering, names and move-pruning tables of the 4x4x4 solver (`cs.threephase.Moves`
//! and `cs.threephase.Util`).

use std::sync::LazyLock;

/// Facelets of the reduced 3x3x3 (`U1` .. `B9`, 9 per face in `URFDLB` order).
#[allow(dead_code)]
pub(crate) mod f3 {
    macro_rules! facelets {
        ($($name:ident = $value:expr),* $(,)?) => { $(pub(crate) const $name: u8 = $value;)* };
    }
    facelets!(
        U1 = 0,
        U2 = 1,
        U3 = 2,
        U4 = 3,
        U5 = 4,
        U6 = 5,
        U7 = 6,
        U8 = 7,
        U9 = 8,
        R1 = 9,
        R2 = 10,
        R3 = 11,
        R4 = 12,
        R5 = 13,
        R6 = 14,
        R7 = 15,
        R8 = 16,
        R9 = 17,
        F1 = 18,
        F2 = 19,
        F3 = 20,
        F4 = 21,
        F5 = 22,
        F6 = 23,
        F7 = 24,
        F8 = 25,
        F9 = 26,
        D1 = 27,
        D2 = 28,
        D3 = 29,
        D4 = 30,
        D5 = 31,
        D6 = 32,
        D7 = 33,
        D8 = 34,
        D9 = 35,
        L1 = 36,
        L2 = 37,
        L3 = 38,
        L4 = 39,
        L5 = 40,
        L6 = 41,
        L7 = 42,
        L8 = 43,
        L9 = 44,
        B1 = 45,
        B2 = 46,
        B3 = 47,
        B4 = 48,
        B5 = 49,
        B6 = 50,
        B7 = 51,
        B8 = 52,
        B9 = 53,
    );
}

/// Facelets of the 4x4x4 (`u0` .. `bf`, 16 per face in `URFDLB` order).
///
/// The names follow the Java source; they live in the value namespace, so `f4::u8` does not
/// clash with the `u8` type.
#[allow(non_upper_case_globals, dead_code)]
pub(crate) mod f4 {
    macro_rules! facelets {
        ($($name:ident = $value:expr),* $(,)?) => { $(pub(crate) const $name: u8 = $value;)* };
    }
    facelets!(
        u0 = 0x00,
        u1 = 0x01,
        u2 = 0x02,
        u3 = 0x03,
        u4 = 0x04,
        u5 = 0x05,
        u6 = 0x06,
        u7 = 0x07,
        u8 = 0x08,
        u9 = 0x09,
        ua = 0x0a,
        ub = 0x0b,
        uc = 0x0c,
        ud = 0x0d,
        ue = 0x0e,
        uf = 0x0f,
        r0 = 0x10,
        r1 = 0x11,
        r2 = 0x12,
        r3 = 0x13,
        r4 = 0x14,
        r5 = 0x15,
        r6 = 0x16,
        r7 = 0x17,
        r8 = 0x18,
        r9 = 0x19,
        ra = 0x1a,
        rb = 0x1b,
        rc = 0x1c,
        rd = 0x1d,
        re = 0x1e,
        rf = 0x1f,
        f0 = 0x20,
        f1 = 0x21,
        f2 = 0x22,
        f3 = 0x23,
        f4 = 0x24,
        f5 = 0x25,
        f6 = 0x26,
        f7 = 0x27,
        f8 = 0x28,
        f9 = 0x29,
        fa = 0x2a,
        fb = 0x2b,
        fc = 0x2c,
        fd = 0x2d,
        fe = 0x2e,
        ff = 0x2f,
        d0 = 0x30,
        d1 = 0x31,
        d2 = 0x32,
        d3 = 0x33,
        d4 = 0x34,
        d5 = 0x35,
        d6 = 0x36,
        d7 = 0x37,
        d8 = 0x38,
        d9 = 0x39,
        da = 0x3a,
        db = 0x3b,
        dc = 0x3c,
        dd = 0x3d,
        de = 0x3e,
        df = 0x3f,
        l0 = 0x40,
        l1 = 0x41,
        l2 = 0x42,
        l3 = 0x43,
        l4 = 0x44,
        l5 = 0x45,
        l6 = 0x46,
        l7 = 0x47,
        l8 = 0x48,
        l9 = 0x49,
        la = 0x4a,
        lb = 0x4b,
        lc = 0x4c,
        ld = 0x4d,
        le = 0x4e,
        lf = 0x4f,
        b0 = 0x50,
        b1 = 0x51,
        b2 = 0x52,
        b3 = 0x53,
        b4 = 0x54,
        b5 = 0x55,
        b6 = 0x56,
        b7 = 0x57,
        b8 = 0x58,
        b9 = 0x59,
        ba = 0x5a,
        bb = 0x5b,
        bc = 0x5c,
        bd = 0x5d,
        be = 0x5e,
        bf = 0x5f,
    );
}

// Moves: outer turns `U U2 U' R .. B'` are 0..18, wide turns `Uw Uw2 Uw' .. Bw'` are 18..36.
// Java names them `Ux1 .. Bx3` and `ux1 .. bx3`; here they are `O*` (outer) and `W*` (wide).
#[allow(dead_code)]
mod names {
    pub(crate) const OU1: usize = 0;
    pub(crate) const OU2: usize = 1;
    pub(crate) const OU3: usize = 2;
    pub(crate) const OR1: usize = 3;
    pub(crate) const OR2: usize = 4;
    pub(crate) const OR3: usize = 5;
    pub(crate) const OF1: usize = 6;
    pub(crate) const OF2: usize = 7;
    pub(crate) const OF3: usize = 8;
    pub(crate) const OD1: usize = 9;
    pub(crate) const OD2: usize = 10;
    pub(crate) const OD3: usize = 11;
    pub(crate) const OL1: usize = 12;
    pub(crate) const OL2: usize = 13;
    pub(crate) const OL3: usize = 14;
    pub(crate) const OB1: usize = 15;
    pub(crate) const OB2: usize = 16;
    pub(crate) const OB3: usize = 17;
    pub(crate) const WU1: usize = 18;
    pub(crate) const WU2: usize = 19;
    pub(crate) const WU3: usize = 20;
    pub(crate) const WR1: usize = 21;
    pub(crate) const WR2: usize = 22;
    pub(crate) const WR3: usize = 23;
    pub(crate) const WF1: usize = 24;
    pub(crate) const WF2: usize = 25;
    pub(crate) const WF3: usize = 26;
    pub(crate) const WD1: usize = 27;
    pub(crate) const WD2: usize = 28;
    pub(crate) const WD3: usize = 29;
    pub(crate) const WL1: usize = 30;
    pub(crate) const WL2: usize = 31;
    pub(crate) const WL3: usize = 32;
    pub(crate) const WB1: usize = 33;
    pub(crate) const WB2: usize = 34;
    pub(crate) const WB3: usize = 35;
    /// End of moves.
    pub(crate) const EOM: usize = 36;
}
pub(crate) use names::*;

/// Move names, padded to three characters.
pub(crate) const MOVE2STR: [&str; 36] = [
    "U  ", "U2 ", "U' ", "R  ", "R2 ", "R' ", "F  ", "F2 ", "F' ", "D  ", "D2 ", "D' ", "L  ",
    "L2 ", "L' ", "B  ", "B2 ", "B' ", "Uw ", "Uw2", "Uw'", "Rw ", "Rw2", "Rw'", "Fw ", "Fw2",
    "Fw'", "Dw ", "Dw2", "Dw'", "Lw ", "Lw2", "Lw'", "Bw ", "Bw2", "Bw'",
];

/// The 28 phase 2 moves (plus [`EOM`]) in the standard numbering.
pub(crate) const MOVE2STD: [usize; 29] = [
    OU1, OU2, OU3, OR1, OR2, OR3, OF1, OF2, OF3, OD1, OD2, OD3, OL1, OL2, OL3, OB1, OB2, OB3, WU2,
    WR1, WR2, WR3, WF2, WD2, WL1, WL2, WL3, WB2, EOM,
];

/// The 20 phase 3 moves (plus [`EOM`]) in the standard numbering.
pub(crate) const MOVE3STD: [usize; 21] = [
    OU1, OU2, OU3, OR2, OF1, OF2, OF3, OD1, OD2, OD3, OL2, OB1, OB2, OB3, WU2, WR2, WF2, WD2, WL2,
    WB2, EOM,
];

/// Move-sequence pruning tables derived from the move numbering.
pub(crate) struct MoveTables {
    pub(crate) ckmv: [[bool; 36]; 37],
    pub(crate) ckmv2: [[bool; 28]; 29],
    pub(crate) ckmv3: [[bool; 20]; 21],
    pub(crate) skip_axis2: [usize; 28],
    pub(crate) skip_axis3: [usize; 20],
    pub(crate) cnk: [[i32; 25]; 25],
    pub(crate) fact: [i32; 13],
}

pub(crate) static MOVES: LazyLock<MoveTables> = LazyLock::new(|| {
    let mut ckmv = [[false; 36]; 37];
    for (i, row) in ckmv.iter_mut().enumerate().take(36) {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (i / 3 == j / 3) || ((i / 3 % 3 == j / 3 % 3) && (i > j));
        }
    }
    let mut ckmv2 = [[false; 28]; 29];
    for (i, row) in ckmv2.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = ckmv[MOVE2STD[i]][MOVE2STD[j]];
        }
    }
    let mut ckmv3 = [[false; 20]; 21];
    for (i, row) in ckmv3.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = ckmv[MOVE3STD[i]][MOVE3STD[j]];
        }
    }
    let mut skip_axis2 = [28; 28];
    for (i, skip) in skip_axis2.iter_mut().enumerate() {
        if let Some(j) = (i..28).find(|&j| !ckmv2[i][j]) {
            *skip = j - 1;
        }
    }
    let mut skip_axis3 = [20; 20];
    for (i, skip) in skip_axis3.iter_mut().enumerate() {
        if let Some(j) = (i..20).find(|&j| !ckmv3[i][j]) {
            *skip = j - 1;
        }
    }
    let mut cnk = [[0; 25]; 25];
    for (i, row) in cnk.iter_mut().enumerate() {
        row[i] = 1;
        row[0] = 1;
    }
    for i in 1..25 {
        for j in 1..=i {
            cnk[i][j] = cnk[i - 1][j] + cnk[i - 1][j - 1];
        }
    }
    let mut fact = [1; 13];
    for i in 0..12 {
        fact[i + 1] = fact[i] * (i as i32 + 1);
    }
    MoveTables {
        ckmv,
        ckmv2,
        ckmv3,
        skip_axis2,
        skip_axis3,
        cnk,
        fact,
    }
});

/// Parses a move sequence such as `"R U Rw' F2 l"` (`Util.tomove`). Unknown characters are
/// skipped; lowercase letters are wide turns.
pub(crate) fn to_moves(s: &str) -> Vec<usize> {
    let mut moves = Vec::new();
    let mut axis: i32 = -1;
    for c in s.chars() {
        match c {
            'U' => axis = 0,
            'R' => axis = 3,
            'F' => axis = 6,
            'D' => axis = 9,
            'L' => axis = 12,
            'B' => axis = 15,
            'u' => axis = 18,
            'r' => axis = 21,
            'f' => axis = 24,
            'd' => axis = 27,
            'l' => axis = 30,
            'b' => axis = 33,
            ' ' => {
                if axis != -1 {
                    moves.push(axis as usize);
                }
                axis = -1;
            }
            '2' => axis += 1,
            '\'' => axis += 2,
            'w' => axis += 18,
            _ => {}
        }
    }
    if axis != -1 {
        moves.push(axis as usize);
    }
    moves
}

/// `Util.swap` with a 4-cycle `key`: 0 cycles `a -> b -> c -> d -> a`, 1 swaps `a <-> c`
/// and `b <-> d`, 2 cycles the other way.
pub(crate) fn swap4<T: Copy>(arr: &mut [T], a: usize, b: usize, c: usize, d: usize, key: usize) {
    match key {
        0 => {
            let temp = arr[d];
            arr[d] = arr[c];
            arr[c] = arr[b];
            arr[b] = arr[a];
            arr[a] = temp;
        }
        1 => {
            arr.swap(a, c);
            arr.swap(b, d);
        }
        2 => {
            let temp = arr[a];
            arr[a] = arr[b];
            arr[b] = arr[c];
            arr[c] = arr[d];
            arr[d] = temp;
        }
        _ => {}
    }
}

/// `Util.set8Perm`: sets `arr` to the permutation of 8 elements with Lehmer index `idx`.
pub(crate) fn set8_perm(arr: &mut [u8; 8], mut idx: i32) {
    let fact = &MOVES.fact;
    let mut val: i32 = 0x7654_3210;
    for i in 0..7 {
        let p = fact[7 - i];
        let mut v = idx / p;
        idx -= v * p;
        v <<= 2;
        arr[i] = ((val >> v) & 0xf) as u8;
        let m = (1 << v) - 1;
        val = (val & m) + ((val >> 4) & !m);
    }
    arr[7] = val as u8;
}

/// `Util.parity`: the parity of the number of inversions.
pub(crate) fn parity(arr: &[u8]) -> i32 {
    let mut parity = 0;
    for i in 0..arr.len() {
        for j in i..arr.len() {
            if arr[i] > arr[j] {
                parity ^= 1;
            }
        }
    }
    parity
}
