//! Shape and permutation tables of the Square-1 solver (`Shape` and `Square`).

use std::sync::LazyLock;

use super::full_cube::FullCube;

pub(crate) const N_SHAPE: usize = 3678;

/// One half layer per value: 1 = corner, 0 = edge.
const HALF_LAYER: [i32; 13] = [
    0x00, 0x03, 0x06, 0x0c, 0x0f, 0x18, 0x1b, 0x1e, 0x30, 0x33, 0x36, 0x3c, 0x3f,
];

/// The shape of a Square-1 (which slots hold corners), with the permutation parity.
#[derive(Debug, Clone, Copy, Default)]
struct Shape {
    top: i32,
    bottom: i32,
    parity: i32,
}

/// The shape tables (`Shape`'s statics).
pub(crate) struct ShapeTables {
    pub(crate) shape_idx: Vec<i32>,
    pub(crate) shape_prun: Vec<i32>,
    pub(crate) shape_prun_opt: Vec<i32>,
    pub(crate) top_move: Vec<i32>,
    pub(crate) bottom_move: Vec<i32>,
    pub(crate) twist_move: Vec<i32>,
}

/// Java's `Arrays.binarySearch`: the index of `key`, or `-(insertion point) - 1`.
fn java_binary_search(a: &[i32], key: i32) -> i32 {
    match a.binary_search(&key) {
        Ok(i) => i as i32,
        Err(i) => -(i as i32) - 1,
    }
}

impl ShapeTables {
    /// `Shape.getShape2Idx`.
    pub(crate) fn shape2idx(&self, shp: i32) -> i32 {
        java_binary_search(&self.shape_idx, shp & 0xff_ffff) << 1 | shp >> 24
    }
}

const FACE_TURN_METRIC: i32 = 0;
const WCA_TURN_METRIC: i32 = 1;

impl Shape {
    fn idx(&self, t: &[i32]) -> i32 {
        java_binary_search(t, self.top << 12 | self.bottom) << 1 | self.parity
    }

    fn set_idx(&mut self, t: &[i32], idx: usize) {
        self.parity = (idx & 1) as i32;
        self.top = t[idx >> 1];
        self.bottom = self.top & 0xfff;
        self.top >>= 12;
    }

    fn turn_layer(layer: &mut i32, parity: &mut i32) -> i32 {
        let mut mv = 0;
        let mut move_parity = 0;
        loop {
            if *layer & 0x800 == 0 {
                mv += 1;
                *layer <<= 1;
            } else {
                mv += 2;
                *layer = (*layer << 2) ^ 0x3003;
            }
            move_parity = 1 - move_parity;
            if (*layer & 0x3f).count_ones() & 1 == 0 {
                break;
            }
        }
        if layer.count_ones() & 2 == 0 {
            *parity ^= move_parity;
        }
        mv
    }

    fn top_move(&mut self) -> i32 {
        Self::turn_layer(&mut self.top, &mut self.parity)
    }

    fn bottom_move(&mut self) -> i32 {
        Self::turn_layer(&mut self.bottom, &mut self.parity)
    }

    fn twist_move(&mut self) {
        let temp = self.top & 0x3f;
        let p1 = temp.count_ones() as i32;
        let p3 = (self.bottom & 0xfc0).count_ones() as i32;
        self.parity ^= 1 & ((p1 & p3) >> 1);
        self.top = (self.top & 0xfc0) | ((self.bottom >> 6) & 0x3f);
        self.bottom = (self.bottom & 0x3f) | temp << 6;
    }
}

fn init_pruning(t: &ShapeTables, prun: &mut [i32], mut done: usize, metric: i32) {
    let mut done0 = 0;
    let mut depth = -1;
    while done != done0 {
        done0 = done;
        depth += 1;
        for i in 0..N_SHAPE * 2 {
            if prun[i] != depth {
                continue;
            }
            // Try a twist.
            let idx = t.twist_move[i] as usize;
            if prun[idx] == -1 {
                done += 1;
                prun[idx] = depth + 1;
            }
            if metric == FACE_TURN_METRIC {
                for moves in [&t.top_move, &t.bottom_move] {
                    let mut m = 0;
                    let mut idx = i as i32;
                    while m != 12 {
                        idx = moves[idx as usize];
                        let inc = idx & 0xf;
                        idx >>= 4;
                        if prun[idx as usize] == -1 {
                            done += 1;
                            prun[idx as usize] = depth + 1;
                        }
                        m += inc;
                    }
                }
            } else if metric == WCA_TURN_METRIC {
                // A top and a bottom turn together count as one move.
                let mut m = 0;
                let mut idx = i as i32;
                while m != 12 {
                    idx = t.top_move[idx as usize];
                    let inc = idx & 0xf;
                    idx >>= 4;
                    let mut m2 = 0;
                    let mut idx2 = idx;
                    while m2 != 12 {
                        idx2 = t.bottom_move[idx2 as usize];
                        let inc2 = idx2 & 0xf;
                        idx2 >>= 4;
                        if prun[idx2 as usize] == -1 {
                            done += 1;
                            prun[idx2 as usize] = depth + 1;
                        }
                        m2 += inc2;
                    }
                    m += inc;
                }
            }
        }
    }
}

impl ShapeTables {
    fn build() -> Self {
        let mut shape_idx = Vec::with_capacity(N_SHAPE);
        for i in 0..13 * 13 * 13 * 13 {
            let dr = HALF_LAYER[i % 13];
            let dl = HALF_LAYER[i / 13 % 13];
            let ur = HALF_LAYER[i / 13 / 13 % 13];
            let ul = HALF_LAYER[i / 13 / 13 / 13];
            let value = ul << 18 | ur << 12 | dl << 6 | dr;
            if value.count_ones() == 16 {
                shape_idx.push(value);
            }
        }
        debug_assert_eq!(shape_idx.len(), N_SHAPE);

        let n = N_SHAPE * 2;
        let mut t = Self {
            shape_idx,
            shape_prun: vec![-1; n],
            shape_prun_opt: vec![-1; n],
            top_move: vec![0; n],
            bottom_move: vec![0; n],
            twist_move: vec![0; n],
        };
        let mut s = Shape::default();
        for i in 0..n {
            s.set_idx(&t.shape_idx, i);
            t.top_move[i] = s.top_move();
            t.top_move[i] |= s.idx(&t.shape_idx) << 4;
            s.set_idx(&t.shape_idx, i);
            t.bottom_move[i] = s.bottom_move();
            t.bottom_move[i] |= s.idx(&t.shape_idx) << 4;
            s.set_idx(&t.shape_idx, i);
            s.twist_move();
            t.twist_move[i] = s.idx(&t.shape_idx);
        }
        for shp in [0x0db_66db, 0x1db_6db6, 0x16d_b6db, 0x06d_bdb6] {
            let idx = t.shape2idx(shp) as usize;
            t.shape_prun[idx] = 0;
        }
        let solved = FullCube::default().shape_idx_with(&t) as usize;
        t.shape_prun_opt[solved] = 0;

        let mut prun = std::mem::take(&mut t.shape_prun);
        init_pruning(&t, &mut prun, 4, FACE_TURN_METRIC);
        t.shape_prun = prun;
        let mut prun = std::mem::take(&mut t.shape_prun_opt);
        init_pruning(&t, &mut prun, 1, WCA_TURN_METRIC);
        t.shape_prun_opt = prun;
        t
    }
}

const FACT: [i32; 8] = [1, 1, 2, 6, 24, 120, 720, 5040];

/// `Square.set8Perm`.
pub(crate) fn set8_perm(arr: &mut [u8; 8], mut idx: i32) {
    let mut val = 0x7654_3210_i32;
    for i in 0..7 {
        let p = FACT[7 - i];
        let mut v = idx / p;
        idx -= v * p;
        v <<= 2;
        arr[i] = ((val >> v) & 0o7) as u8;
        let m = (1 << v) - 1;
        val = (val & m) + ((val >> 4) & !m);
    }
    arr[7] = val as u8;
}

/// `Square.get8Perm`.
pub(crate) fn get8_perm(arr: [u8; 8]) -> i32 {
    let mut idx = 0;
    let mut val = 0x7654_3210_i32;
    for i in 0..7 {
        let v = i32::from(arr[i]) << 2;
        idx = (8 - i as i32) * idx + ((val >> v) & 0o7);
        val = val.wrapping_sub(0x1111_1110 << v);
    }
    idx
}

/// The phase 2 permutation tables (`Square`'s statics).
pub(crate) struct SquareTables {
    pub(crate) square_prun: Vec<i8>,
    pub(crate) twist_move: Vec<u16>,
    pub(crate) top_move: Vec<u16>,
    pub(crate) bottom_move: Vec<u16>,
}

impl SquareTables {
    fn build() -> Self {
        const N: usize = 40320;
        let mut t = Self {
            square_prun: vec![-1; N * 2],
            twist_move: vec![0; N],
            top_move: vec![0; N],
            bottom_move: vec![0; N],
        };
        let mut pos = [0_u8; 8];
        for i in 0..N {
            // Twist.
            set8_perm(&mut pos, i as i32);
            pos.swap(2, 4);
            pos.swap(3, 5);
            t.twist_move[i] = get8_perm(pos) as u16;
            // Top layer turn.
            set8_perm(&mut pos, i as i32);
            pos[..4].rotate_left(1);
            t.top_move[i] = get8_perm(pos) as u16;
            // Bottom layer turn.
            set8_perm(&mut pos, i as i32);
            pos[4..].rotate_left(1);
            t.bottom_move[i] = get8_perm(pos) as u16;
        }

        t.square_prun[0] = 0;
        let mut depth = 0;
        let mut done = 1;
        while done < N * 2 {
            let inv = depth >= 11;
            let find = if inv { -1 } else { depth };
            let check = if inv { depth } else { -1 };
            depth += 1;
            'out: for i in 0..N * 2 {
                if i32::from(t.square_prun[i]) != find {
                    continue;
                }
                let mut perm = i >> 1;
                let ml = i & 1;

                // Try a twist.
                let idx = usize::from(t.twist_move[perm]) << 1 | (1 - ml);
                if i32::from(t.square_prun[idx]) == check {
                    done += 1;
                    t.square_prun[if inv { i } else { idx }] = depth as i8;
                    if inv {
                        continue 'out;
                    }
                }
                // Try turning the top layer.
                for _ in 0..4 {
                    perm = usize::from(t.top_move[perm]);
                    let idx = perm << 1 | ml;
                    if i32::from(t.square_prun[idx]) == check {
                        done += 1;
                        t.square_prun[if inv { i } else { idx }] = depth as i8;
                        if inv {
                            continue 'out;
                        }
                    }
                }
                // Try turning the bottom layer.
                for _ in 0..4 {
                    perm = usize::from(t.bottom_move[perm]);
                    let idx = perm << 1 | ml;
                    if i32::from(t.square_prun[idx]) == check {
                        done += 1;
                        t.square_prun[if inv { i } else { idx }] = depth as i8;
                        if inv {
                            continue 'out;
                        }
                    }
                }
            }
        }
        t
    }
}

/// All Square-1 solver tables.
pub(crate) struct Tables {
    pub(crate) shape: ShapeTables,
    pub(crate) square: SquareTables,
}

pub(crate) static TABLES: LazyLock<Tables> = LazyLock::new(|| Tables {
    shape: ShapeTables::build(),
    square: SquareTables::build(),
});
