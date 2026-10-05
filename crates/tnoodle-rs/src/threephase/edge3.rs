//! The phase 3 edge coordinate and its pruning table (`cs.threephase.Edge3`).

use std::sync::atomic::{AtomicUsize, Ordering};

use super::cubes::EdgeCube;

pub(crate) const N_SYM: usize = 1538;
pub(crate) const N_RAW: usize = 20160;
pub(crate) const N_EPRUN: usize = N_SYM * N_RAW;
pub(crate) const MAX_DEPTH: i32 = 10;
const N_RAW2SYM: usize = 11880;

const PRUN_VALUES: [usize; 14] = [
    1, 4, 16, 55, 324, 1922, 12275, 77640, 485_359, 2_778_197, 11_742_425, 27_492_416, 31_002_941,
    31_006_080,
];

const SYMINV: [usize; 8] = [0, 1, 6, 3, 4, 5, 2, 7];
const FACT_X: [i32; 13] = [
    1,
    1,
    1,
    3,
    12,
    60,
    360,
    2520,
    20160,
    181_440,
    1_814_400,
    19_958_400,
    239_500_800,
];
const FULL_EDGE_MAP: [usize; 12] = [0, 2, 4, 6, 1, 3, 7, 5, 8, 9, 10, 11];

/// Progress of the pruning table construction, for [`init_status`].
static DONE: AtomicUsize = AtomicUsize::new(0);

/// How far the (slow) edge pruning table construction has progressed, from 0 to 1
/// (`Edge3.initStatus`).
pub fn init_status() -> f64 {
    DONE.load(Ordering::Relaxed) as f64 / PRUN_VALUES[MAX_DEPTH as usize - 1] as f64
}

/// The 12 composite edges of the phase 3 reduced 4x4x4 (`Edge3` instances).
///
/// `edge` and `edgeo` hold a permutation in a split representation that rotations update
/// lazily; [`std`](Self::std) normalises it. A new value starts all zeroes (and "standard")
/// exactly like a freshly allocated Java `Edge3`, which matters for bit-exact results.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Edge3 {
    pub(crate) edge: [i32; 12],
    edgeo: [i32; 12],
    temp: [i32; 12],
    is_std: bool,
}

/// `Edge3`'s static tables.
pub(crate) struct Edge3Tables {
    pub(crate) eprun: Vec<i32>,
    pub(crate) sym2raw: Vec<i32>,
    pub(crate) symstate: Vec<u16>,
    pub(crate) raw2sym: Vec<i32>,
    pub(crate) mvrot: Vec<[i32; 12]>,
    pub(crate) mvroto: Vec<[i32; 12]>,
}

fn set_pruning(table: &mut [i32], index: usize, value: i32) {
    table[index >> 4] ^= (0x3 ^ value) << ((index & 0xf) << 1);
}

fn get_pruning(table: &[i32], index: usize) -> i32 {
    (table[index >> 4] >> ((index & 0xf) << 1)) & 0x3
}

impl Edge3 {
    pub(crate) fn new() -> Self {
        Self {
            is_std: true,
            ..Self::default()
        }
    }

    /// Sets the edges from a 4x4x4 edge cube; returns the parity of the composite edges.
    pub(crate) fn set_from_cube(&mut self, c: &EdgeCube) -> i32 {
        for i in 0..12 {
            self.temp[i] = i as i32;
            self.edge[i] = i32::from(c.ep[FULL_EDGE_MAP[i] + 12] % 12);
        }
        let mut parity = 1; // because of FULL_EDGE_MAP
        for i in 0..12 {
            while self.edge[i] != i as i32 {
                let t = self.edge[i] as usize;
                self.edge[i] = self.edge[t];
                self.edge[t] = t as i32;
                self.temp.swap(i, t);
                parity ^= 1;
            }
        }
        for i in 0..12 {
            self.edge[i] = self.temp[usize::from(c.ep[FULL_EDGE_MAP[i]] % 12)];
        }
        parity
    }

    /// Sets the edges from an index (`set(int)`).
    pub(crate) fn set_index(&mut self, mut idx: i32) {
        let mut val: i64 = 0xba98_7654_3210;
        let mut parity = 0;
        for i in 0..11 {
            let p = FACT_X[11 - i];
            let mut v = idx / p;
            idx %= p;
            parity ^= v;
            v <<= 2;
            self.edge[i] = ((val >> v) & 0xf) as i32;
            let m = (1_i64 << v) - 1;
            val = (val & m) + ((val >> 4) & !m);
        }
        if parity & 1 == 0 {
            self.edge[11] = val as i32;
        } else {
            self.edge[11] = self.edge[10];
            self.edge[10] = val as i32;
        }
        for (i, o) in self.edgeo.iter_mut().enumerate() {
            *o = i as i32;
        }
        self.is_std = true;
    }

    /// Copies the permutation of `e` (`set(Edge3)`); the scratch buffer is left alone.
    pub(crate) fn set_from(&mut self, e: &Self) {
        self.edge = e.edge;
        self.edgeo = e.edgeo;
        self.is_std = e.is_std;
    }

    fn std(&mut self) {
        for i in 0..12 {
            self.temp[self.edgeo[i] as usize] = i as i32;
        }
        for i in 0..12 {
            self.edge[i] = self.temp[self.edge[i] as usize];
            self.edgeo[i] = i as i32;
        }
        self.is_std = true;
    }

    /// The index of the first `end` edges.
    pub(crate) fn get(&mut self, end: usize) -> i32 {
        if !self.is_std {
            self.std();
        }
        let mut idx = 0_i32;
        let mut val: i64 = 0xba98_7654_3210;
        for i in 0..end {
            let v = self.edge[i] << 2;
            idx *= 12 - i as i32;
            idx += ((val >> v) & 0xf) as i32;
            val = val.wrapping_sub(0x1111_1111_1110_i64 << v);
        }
        idx
    }

    /// The symmetry reduced coordinate (rotates `self`).
    pub(crate) fn get_sym(&mut self, t: &Edge3Tables) -> i32 {
        let cord1x = self.get(4);
        let mut symcord1x = t.raw2sym[cord1x as usize];
        let symx = symcord1x & 0x7;
        symcord1x >>= 3;
        self.rotate(symx as usize);
        let cord2x = self.get(10) % N_RAW as i32;
        symcord1x * N_RAW as i32 + cord2x
    }

    pub(crate) fn do_move(&mut self, i: usize) {
        self.is_std = false;
        let (e, o) = (&mut self.edge, &mut self.edgeo);
        match i {
            0 => {
                circle(e, 0, 4, 1, 5);
                circle(o, 0, 4, 1, 5);
            }
            1 => {
                swap4(e, 0, 4, 1, 5);
                swap4(o, 0, 4, 1, 5);
            }
            2 => {
                circle(e, 0, 5, 1, 4);
                circle(o, 0, 5, 1, 4);
            }
            3 => {
                swap4(e, 5, 10, 6, 11);
                swap4(o, 5, 10, 6, 11);
            }
            4 => {
                circle(e, 0, 11, 3, 8);
                circle(o, 0, 11, 3, 8);
            }
            5 => {
                swap4(e, 0, 11, 3, 8);
                swap4(o, 0, 11, 3, 8);
            }
            6 => {
                circle(e, 0, 8, 3, 11);
                circle(o, 0, 8, 3, 11);
            }
            7 => {
                circle(e, 2, 7, 3, 6);
                circle(o, 2, 7, 3, 6);
            }
            8 => {
                swap4(e, 2, 7, 3, 6);
                swap4(o, 2, 7, 3, 6);
            }
            9 => {
                circle(e, 2, 6, 3, 7);
                circle(o, 2, 6, 3, 7);
            }
            10 => {
                swap4(e, 4, 8, 7, 9);
                swap4(o, 4, 8, 7, 9);
            }
            11 => {
                circle(e, 1, 9, 2, 10);
                circle(o, 1, 9, 2, 10);
            }
            12 => {
                swap4(e, 1, 9, 2, 10);
                swap4(o, 1, 9, 2, 10);
            }
            13 => {
                circle(e, 1, 10, 2, 9);
                circle(o, 1, 10, 2, 9);
            }
            14 => {
                swap4(e, 0, 4, 1, 5);
                swap4(o, 0, 4, 1, 5);
                e.swap(9, 11);
                o.swap(8, 10);
            }
            15 => {
                swap4(e, 5, 10, 6, 11);
                swap4(o, 5, 10, 6, 11);
                e.swap(1, 3);
                o.swap(0, 2);
            }
            16 => {
                swap4(e, 0, 11, 3, 8);
                swap4(o, 0, 11, 3, 8);
                e.swap(5, 7);
                o.swap(4, 6);
            }
            17 => {
                swap4(e, 2, 7, 3, 6);
                swap4(o, 2, 7, 3, 6);
                e.swap(8, 10);
                o.swap(9, 11);
            }
            18 => {
                swap4(e, 4, 8, 7, 9);
                swap4(o, 4, 8, 7, 9);
                e.swap(0, 2);
                o.swap(1, 3);
            }
            19 => {
                swap4(e, 1, 9, 2, 10);
                swap4(o, 1, 9, 2, 10);
                e.swap(4, 6);
                o.swap(5, 7);
            }
            _ => {}
        }
    }

    fn rot(&mut self, r: usize) {
        self.is_std = false;
        match r {
            0 => {
                self.do_move(14);
                self.do_move(17);
            }
            1 => {
                self.circlex(11, 5, 10, 6); // r
                self.circlex(5, 10, 6, 11);
                self.circlex(1, 2, 3, 0);
                self.circlex(4, 9, 7, 8); // l'
                self.circlex(8, 4, 9, 7);
                self.circlex(0, 1, 2, 3);
            }
            2 => {
                for (x, y) in [
                    (4, 5),
                    (5, 4),
                    (11, 8),
                    (8, 11),
                    (7, 6),
                    (6, 7),
                    (9, 10),
                    (10, 9),
                ] {
                    self.swapx(x, y);
                }
                for x in [1, 0, 3, 2] {
                    self.swapx(x, x);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn rotate(&mut self, mut r: usize) {
        while r >= 2 {
            r -= 2;
            self.rot(1);
            self.rot(2);
        }
        if r != 0 {
            self.rot(0);
        }
    }

    fn swapx(&mut self, x: usize, y: usize) {
        std::mem::swap(&mut self.edge[x], &mut self.edgeo[y]);
    }

    fn circlex(&mut self, a: usize, b: usize, c: usize, d: usize) {
        let temp = self.edgeo[d];
        self.edgeo[d] = self.edge[c];
        self.edge[c] = self.edgeo[b];
        self.edgeo[b] = self.edge[a];
        self.edge[a] = temp;
    }
}

fn circle(arr: &mut [i32; 12], a: usize, b: usize, c: usize, d: usize) {
    let temp = arr[d];
    arr[d] = arr[c];
    arr[c] = arr[b];
    arr[b] = arr[a];
    arr[a] = temp;
}

fn swap4(arr: &mut [i32; 12], a: usize, b: usize, c: usize, d: usize) {
    arr.swap(a, c);
    arr.swap(b, d);
}

impl Edge3Tables {
    pub(crate) fn build() -> Self {
        let mut t = Self {
            eprun: vec![0; N_EPRUN / 16],
            sym2raw: vec![0; N_SYM],
            symstate: vec![0; N_SYM],
            raw2sym: vec![0; N_RAW2SYM],
            mvrot: vec![[0; 12]; 20 * 8],
            mvroto: vec![[0; 12]; 20 * 8],
        };
        t.init_mvrot();
        t.init_raw2sym();
        t.create_prun();
        t
    }

    fn init_mvrot(&mut self) {
        let mut e = Edge3::new();
        for m in 0..20 {
            for r in 0..8 {
                e.set_index(0);
                e.do_move(m);
                e.rotate(r);
                self.mvrot[m << 3 | r] = e.edge;
                e.std();
                self.mvroto[m << 3 | r] = e.temp;
            }
        }
    }

    fn init_raw2sym(&mut self) {
        let mut e = Edge3::new();
        let mut occ = vec![0_u8; N_RAW2SYM / 8];
        let mut count = 0;
        for i in 0..N_RAW2SYM {
            if occ[i >> 3] & (1 << (i & 7)) == 0 {
                e.set_index(i as i32 * FACT_X[8]);
                for j in 0..8 {
                    let idx = e.get(4) as usize;
                    if idx == i {
                        self.symstate[count] |= 1 << j;
                    }
                    occ[idx >> 3] |= 1 << (idx & 7);
                    self.raw2sym[idx] = (count << 3 | SYMINV[j]) as i32;
                    e.rot(0);
                    if j % 2 == 1 {
                        e.rot(1);
                        e.rot(2);
                    }
                }
                self.sym2raw[count] = i as i32;
                count += 1;
            }
        }
        debug_assert_eq!(count, N_SYM);
    }

    /// `Edge3.getmvrot`: the index of the first `end` edges after applying the move and
    /// rotation `mr_idx` to the permutation `ep`.
    pub(crate) fn mvrot_index(&self, ep: &[i32; 12], mr_idx: usize, end: usize) -> i32 {
        let movo = &self.mvroto[mr_idx];
        let mov = &self.mvrot[mr_idx];
        let mut idx = 0_i32;
        let mut val: i64 = 0xba98_7654_3210;
        for i in 0..end {
            let v = movo[ep[mov[i] as usize] as usize] << 2;
            idx *= 12 - i as i32;
            idx += ((val >> v) & 0xf) as i32;
            val = val.wrapping_sub(0x1111_1111_1110_i64 << v);
        }
        idx
    }

    /// A lower bound of the distance of `edge` from solved, given the bound `prun` of a
    /// neighbouring state (`getprun(int, int)`).
    pub(crate) fn prun_relative(&self, edge: usize, prun: i32) -> i32 {
        let depm3 = get_pruning(&self.eprun, edge);
        if depm3 == 0x3 {
            return MAX_DEPTH;
        }
        (depm3 - prun + 16) % 3 + prun - 1
    }

    /// The exact (capped) distance of `edge` from solved (`getprun(int)`).
    pub(crate) fn prun(&self, mut edge: usize) -> i32 {
        let mut e = Edge3::new();
        let mut depth = 0;
        let mut depm3 = get_pruning(&self.eprun, edge);
        if depm3 == 0x3 {
            return MAX_DEPTH;
        }
        while edge != 0 {
            depm3 = if depm3 == 0 { 2 } else { depm3 - 1 };
            let symcord1 = edge / N_RAW;
            let cord1 = self.sym2raw[symcord1];
            let cord2 = (edge % N_RAW) as i32;
            e.set_index(cord1 * N_RAW as i32 + cord2);
            for m in 0..17 {
                let cord1x = self.mvrot_index(&e.edge, m << 3, 4);
                let mut symcord1x = self.raw2sym[cord1x as usize];
                let symx = (symcord1x & 0x7) as usize;
                symcord1x >>= 3;
                let cord2x = self.mvrot_index(&e.edge, m << 3 | symx, 10) % N_RAW as i32;
                let idx = symcord1x as usize * N_RAW + cord2x as usize;
                if get_pruning(&self.eprun, idx) == depm3 {
                    depth += 1;
                    edge = idx;
                    break;
                }
            }
        }
        depth
    }

    fn create_prun(&mut self) {
        let mut e = Edge3::new();
        let mut f = Edge3::new();
        let mut g = Edge3::new();
        self.eprun.fill(-1);
        let mut depth = 0;
        let mut done = 1;
        DONE.store(done, Ordering::Relaxed);
        set_pruning(&mut self.eprun, 0, 0);

        while done != N_EPRUN {
            let inv = depth > 9;
            let depm3 = depth % 3;
            let dep1m3 = (depth + 1) % 3;
            let find = if inv { 0x3 } else { depm3 };
            let chk = if inv { depm3 } else { 0x3 };

            if depth >= MAX_DEPTH - 1 {
                break;
            }

            for block in 0..N_EPRUN / 16 {
                let mut val = self.eprun[block];
                if !inv && val == -1 {
                    continue;
                }
                for i in block * 16..block * 16 + 16 {
                    let entry = val;
                    val >>= 2;
                    if entry & 0x3 != find {
                        continue;
                    }
                    let symcord1 = i / N_RAW;
                    let cord1 = self.sym2raw[symcord1];
                    let cord2 = (i % N_RAW) as i32;
                    e.set_index(cord1 * N_RAW as i32 + cord2);

                    for m in 0..17 {
                        let cord1x = self.mvrot_index(&e.edge, m << 3, 4);
                        let mut symcord1x = self.raw2sym[cord1x as usize];
                        let symx = (symcord1x & 0x7) as usize;
                        symcord1x >>= 3;
                        let cord2x = self.mvrot_index(&e.edge, m << 3 | symx, 10) % N_RAW as i32;
                        let idx = symcord1x as usize * N_RAW + cord2x as usize;
                        if get_pruning(&self.eprun, idx) != chk {
                            continue;
                        }
                        set_pruning(&mut self.eprun, if inv { i } else { idx }, dep1m3);
                        done += 1;
                        if inv {
                            break;
                        }
                        let mut sym_state = self.symstate[symcord1x as usize];
                        if sym_state == 1 {
                            continue;
                        }
                        f.set_from(&e);
                        f.do_move(m);
                        f.rotate(symx);
                        let mut j = 1;
                        loop {
                            sym_state >>= 1;
                            if sym_state == 0 {
                                break;
                            }
                            if sym_state & 1 == 1 {
                                g.set_from(&f);
                                g.rotate(j);
                                let idxx = symcord1x as usize * N_RAW
                                    + (g.get(10) % N_RAW as i32) as usize;
                                if get_pruning(&self.eprun, idxx) == chk {
                                    set_pruning(&mut self.eprun, idxx, dep1m3);
                                    done += 1;
                                }
                            }
                            j += 1;
                        }
                    }
                }
            }
            depth += 1;
            DONE.store(done, Ordering::Relaxed);
        }
    }
}
