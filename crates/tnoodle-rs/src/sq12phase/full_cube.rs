//! The Square-1 piece representation (`cs.sq12phase.FullCube`).

use super::tables::{ShapeTables, TABLES};
use crate::java::RandomSource;

/// A Square-1: four half layers of six nibbles each (a corner occupies two nibbles), and
/// the middle layer (`0` = solved).
///
/// Pieces are numbered 0..16: even numbers below 8 and odd numbers from 8 are corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FullCube {
    pub(crate) ul: i32,
    pub(crate) ur: i32,
    pub(crate) dl: i32,
    pub(crate) dr: i32,
    pub(crate) ml: i32,
}

impl Default for FullCube {
    fn default() -> Self {
        Self {
            ul: 0x01_1233,
            ur: 0x45_5677,
            dl: 0x99_8bba,
            dr: 0xdd_cffe,
            ml: 0,
        }
    }
}

/// The phase 2 coordinates of a cube in a square shape (`cs.sq12phase.Square`).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Square {
    pub(crate) edgeperm: i32,
    pub(crate) cornperm: i32,
    pub(crate) top_edge_first: bool,
    pub(crate) bot_edge_first: bool,
    pub(crate) ml: i32,
}

impl FullCube {
    /// A solved Square-1.
    pub fn new() -> Self {
        Self::default()
    }

    /// The raw representation: the four half layers (`ul`, `ur`, `dl`, `dr`) and the
    /// middle layer.
    pub fn raw(&self) -> [i32; 5] {
        [self.ul, self.ur, self.dl, self.dr, self.ml]
    }

    /// A uniformly random Square-1 (`randomCube(Random)`).
    pub fn random(r: &mut dyn RandomSource) -> Self {
        let t = &TABLES.shape;
        let shape = t.shape_idx[r.next_int_bounded(3678) as usize];
        let mut f = Self::default();
        let mut corner: i32 = 0x0123_4567 << 1 | 0x1111_1111;
        let mut edge: i32 = 0x0123_4567 << 1;
        let mut n_corner = 8;
        let mut n_edge = 8;
        let mut i = 0;
        while i < 24 {
            if (shape >> i) & 1 == 0 {
                // edge
                let rnd = r.next_int_bounded(n_edge) << 2;
                f.set_piece(23 - i, (edge >> rnd) & 0xf);
                let m = (1 << rnd) - 1;
                edge = (edge & m) + ((edge >> 4) & !m);
                n_edge -= 1;
            } else {
                // corner
                let rnd = r.next_int_bounded(n_corner) << 2;
                f.set_piece(23 - i, (corner >> rnd) & 0xf);
                f.set_piece(22 - i, (corner >> rnd) & 0xf);
                let m = (1 << rnd) - 1;
                corner = (corner & m) + ((corner >> 4) & !m);
                n_corner -= 1;
                i += 1;
            }
            i += 1;
        }
        f.ml = r.next_int_bounded(2);
        f
    }

    pub(crate) fn is_solved(&self) -> bool {
        *self == Self::default()
    }

    /// Applies a move: `0` is a twist (`/`), `1..=11` turn the top layer and `-1..=-11`
    /// the bottom layer, e.g. `6` is `(6,0)`, `9` is `(-3,0)` and `-4` is `(0,4)`.
    pub(crate) fn do_move(&mut self, mv: i32) {
        let mut mv = mv << 2;
        if mv > 24 {
            mv = 48 - mv;
            let temp = self.ul;
            self.ul = (self.ul >> mv | self.ur.wrapping_shl((24 - mv) as u32)) & 0xff_ffff;
            self.ur = (self.ur >> mv | temp.wrapping_shl((24 - mv) as u32)) & 0xff_ffff;
        } else if mv > 0 {
            let temp = self.ul;
            self.ul = (self.ul.wrapping_shl(mv as u32) | self.ur >> (24 - mv)) & 0xff_ffff;
            self.ur = (self.ur.wrapping_shl(mv as u32) | temp >> (24 - mv)) & 0xff_ffff;
        } else if mv == 0 {
            std::mem::swap(&mut self.ur, &mut self.dl);
            self.ml = 1 - self.ml;
        } else if mv >= -24 {
            mv = -mv;
            let temp = self.dl;
            self.dl = (self.dl.wrapping_shl(mv as u32) | self.dr >> (24 - mv)) & 0xff_ffff;
            self.dr = (self.dr.wrapping_shl(mv as u32) | temp >> (24 - mv)) & 0xff_ffff;
        } else {
            mv += 48;
            let temp = self.dl;
            self.dl = (self.dl >> mv | self.dr.wrapping_shl((24 - mv) as u32)) & 0xff_ffff;
            self.dr = (self.dr >> mv | temp.wrapping_shl((24 - mv) as u32)) & 0xff_ffff;
        }
    }

    fn piece_at(&self, idx: usize) -> i32 {
        let ret = if idx < 6 {
            self.ul >> ((5 - idx) << 2)
        } else if idx < 12 {
            self.ur >> ((11 - idx) << 2)
        } else if idx < 18 {
            self.dl >> ((17 - idx) << 2)
        } else {
            self.dr >> ((23 - idx) << 2)
        };
        ret & 0x0f
    }

    /// Sets slot `idx` (0..24, or 24 for the middle layer) to `value`.
    pub fn set_piece(&mut self, idx: usize, value: i32) {
        let (layer, shift) = match idx {
            0..6 => (&mut self.ul, (5 - idx) << 2),
            6..12 => (&mut self.ur, (11 - idx) << 2),
            12..18 => (&mut self.dl, (17 - idx) << 2),
            18..24 => (&mut self.dr, (23 - idx) << 2),
            _ => {
                self.ml = value;
                return;
            }
        };
        *layer &= !(0xf << shift);
        *layer |= value << shift;
    }

    fn parity(&self) -> i32 {
        let mut arr = [0; 16];
        let mut cnt = 0;
        arr[0] = self.piece_at(0);
        for i in 1..24 {
            if self.piece_at(i) != arr[cnt] {
                cnt += 1;
                arr[cnt] = self.piece_at(i);
            }
        }
        let mut p = 0;
        for a in 0..16 {
            for b in a + 1..16 {
                if arr[a] > arr[b] {
                    p ^= 1;
                }
            }
        }
        p
    }

    fn layer_shape(layer: i32) -> i32 {
        let mut x = layer & 0x11_1111;
        x |= x >> 3;
        x |= x >> 6;
        (x & 0xf) | ((x >> 12) & 0x30)
    }

    pub(crate) fn shape_idx_with(&self, t: &ShapeTables) -> i32 {
        let urx = Self::layer_shape(self.ur);
        let ulx = Self::layer_shape(self.ul);
        let drx = Self::layer_shape(self.dr);
        let dlx = Self::layer_shape(self.dl);
        t.shape2idx(self.parity() << 24 | ulx << 18 | urx << 12 | dlx << 6 | drx)
    }

    pub(crate) fn shape_idx(&self) -> i32 {
        self.shape_idx_with(&TABLES.shape)
    }

    pub(crate) fn square(&self) -> Square {
        let mut sq = Square::default();
        let mut prm = [0_u8; 8];
        for (a, p) in prm.iter_mut().enumerate() {
            *p = (self.piece_at(a * 3 + 1) >> 1) as u8;
        }
        sq.cornperm = super::tables::get8_perm(prm);
        sq.top_edge_first = self.piece_at(0) == self.piece_at(1);
        let mut a = if sq.top_edge_first { 2 } else { 0 };
        let mut b = 0;
        while b < 4 {
            prm[b] = (self.piece_at(a) >> 1) as u8;
            a += 3;
            b += 1;
        }
        sq.bot_edge_first = self.piece_at(12) == self.piece_at(13);
        a = if sq.bot_edge_first { 14 } else { 12 };
        while b < 8 {
            prm[b] = (self.piece_at(a) >> 1) as u8;
            a += 3;
            b += 1;
        }
        sq.edgeperm = super::tables::get8_perm(prm);
        sq.ml = self.ml;
        sq
    }
}
