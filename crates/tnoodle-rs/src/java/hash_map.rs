//! A map that iterates in exactly the order `java.util.HashMap` would.
//!
//! Java's `HashMap` iterates its buckets in index order and each bucket along its `next`
//! links. The bucket of a key depends on its `hashCode()`, the current table capacity, and
//! the history of resizes, all of which are reproduced here. Long buckets are converted to
//! red-black trees ("treeified"), which reorders their `next` links; that is reproduced
//! too. TNoodle's scrambles, solutions and SVG attribute orders all depend on this order.

use std::cmp::Ordering;

use super::hash::JavaHash;

/// `HashMap.TREEIFY_THRESHOLD`: bins longer than this are converted to trees.
const TREEIFY_THRESHOLD: usize = 8;
/// `HashMap.UNTREEIFY_THRESHOLD`: trees this small are converted back to lists on resize.
const UNTREEIFY_THRESHOLD: usize = 6;
/// `HashMap.MIN_TREEIFY_CAPACITY`: smaller tables are resized instead of treeified.
const MIN_TREEIFY_CAPACITY: usize = 64;
const DEFAULT_INITIAL_CAPACITY: usize = 16;
const LOAD_FACTOR: f32 = 0.75;

#[derive(Debug, Clone)]
struct Entry<K, V> {
    hash: i32,
    key: K,
    value: V,
    // `HashMap.TreeNode` links, only meaningful while the entry is in a treeified bin.
    parent: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    red: bool,
}

/// A bucket: its entries in `next` order. A tree bin's root is always first.
#[derive(Debug, Clone, Default)]
struct Bin {
    order: Vec<usize>,
    tree: bool,
}

/// An insertion-history aware map that reproduces `java.util.HashMap`'s iteration order.
///
/// Lookups are linear within a bucket. Treeified bins keep Java's red-black tree only to
/// reproduce the iteration order it implies.
///
/// Java breaks ties between distinct, non-comparable keys with equal hash codes using
/// `System.identityHashCode`, which is nondeterministic. This map always picks the order
/// Java picks when the new key's identity hash is the smaller one.
#[derive(Debug, Clone)]
pub struct JavaHashMap<K, V> {
    entries: Vec<Option<Entry<K, V>>>,
    free: Vec<usize>,
    table: Vec<Bin>,
    threshold: usize,
    len: usize,
}

impl<K, V> Default for JavaHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// `HashMap.hash`: spreads the high bits of the hash code downwards.
fn spread(h: i32) -> i32 {
    h ^ ((h as u32) >> 16) as i32
}

/// `HashMap.tableSizeFor`: the smallest power of two that is at least `cap` (and at least 1).
fn table_size_for(cap: usize) -> usize {
    cap.max(1).next_power_of_two()
}

impl<K, V> JavaHashMap<K, V> {
    /// Equivalent of `new HashMap<>()`.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            free: Vec::new(),
            table: Vec::new(),
            threshold: 0,
            len: 0,
        }
    }

    /// Equivalent of `new HashMap<>(initialCapacity)`.
    pub fn with_capacity(initial_capacity: usize) -> Self {
        let mut map = Self::new();
        map.threshold = table_size_for(initial_capacity);
        map
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterates over the entries in Java's iteration order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.table.iter().flat_map(|bin| &bin.order).map(|&i| {
            let e = self.node(i);
            (&e.key, &e.value)
        })
    }

    /// Iterates over the keys in Java's iteration order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(k, _)| k)
    }

    /// Iterates over the values in Java's iteration order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }

    /// Consumes the map, yielding its entries in Java's iteration order.
    pub fn into_entries(mut self) -> Vec<(K, V)> {
        let order: Vec<usize> = self.table.iter().flat_map(|b| &b.order).copied().collect();
        order
            .into_iter()
            .map(|i| {
                let e = self.entries[i]
                    .take()
                    .expect("bucket points at a live entry");
                (e.key, e.value)
            })
            .collect()
    }

    fn node(&self, i: usize) -> &Entry<K, V> {
        self.entries[i]
            .as_ref()
            .expect("bucket points at a live entry")
    }

    fn node_mut(&mut self, i: usize) -> &mut Entry<K, V> {
        self.entries[i]
            .as_mut()
            .expect("bucket points at a live entry")
    }

    fn bucket_index(&self, hash: i32) -> usize {
        (hash as u32 as usize) & (self.table.len() - 1)
    }

    fn is_red(&self, i: Option<usize>) -> bool {
        i.is_some_and(|i| self.node(i).red)
    }

    fn set_red(&mut self, i: usize, red: bool) {
        self.node_mut(i).red = red;
    }

    /// Points `parent`'s link to `old` at `new` instead (`pp.left == p ? left : right`).
    fn replace_child(&mut self, parent: usize, old: usize, new: Option<usize>) {
        if self.node(parent).left == Some(old) {
            self.node_mut(parent).left = new;
        } else {
            self.node_mut(parent).right = new;
        }
    }

    /// `TreeNode.root`.
    fn tree_root(&self, mut i: usize) -> usize {
        while let Some(p) = self.node(i).parent {
            i = p;
        }
        i
    }

    /// `TreeNode.rotateLeft`.
    fn rotate_left(&mut self, mut root: usize, p: usize) -> usize {
        let Some(r) = self.node(p).right else {
            return root;
        };
        let rl = self.node(r).left;
        self.node_mut(p).right = rl;
        if let Some(rl) = rl {
            self.node_mut(rl).parent = Some(p);
        }
        let pp = self.node(p).parent;
        self.node_mut(r).parent = pp;
        match pp {
            None => {
                root = r;
                self.set_red(r, false);
            }
            Some(pp) => self.replace_child(pp, p, Some(r)),
        }
        self.node_mut(r).left = Some(p);
        self.node_mut(p).parent = Some(r);
        root
    }

    /// `TreeNode.rotateRight`.
    fn rotate_right(&mut self, mut root: usize, p: usize) -> usize {
        let Some(l) = self.node(p).left else {
            return root;
        };
        let lr = self.node(l).right;
        self.node_mut(p).left = lr;
        if let Some(lr) = lr {
            self.node_mut(lr).parent = Some(p);
        }
        let pp = self.node(p).parent;
        self.node_mut(l).parent = pp;
        match pp {
            None => {
                root = l;
                self.set_red(l, false);
            }
            Some(pp) => {
                // Java tests `pp.right == p` here, the mirror image of `rotateLeft`.
                if self.node(pp).right == Some(p) {
                    self.node_mut(pp).right = Some(l);
                } else {
                    self.node_mut(pp).left = Some(l);
                }
            }
        }
        self.node_mut(l).right = Some(p);
        self.node_mut(p).parent = Some(l);
        root
    }

    /// `TreeNode.balanceInsertion`: returns the new root.
    fn balance_insertion(&mut self, mut root: usize, mut x: usize) -> usize {
        self.set_red(x, true);
        loop {
            let Some(xp) = self.node(x).parent else {
                self.set_red(x, false);
                return x;
            };
            if !self.node(xp).red {
                return root;
            }
            let Some(xpp) = self.node(xp).parent else {
                return root;
            };
            let xppl = self.node(xpp).left;
            if xppl == Some(xp) {
                let xppr = self.node(xpp).right;
                if let Some(xppr) = xppr.filter(|&n| self.node(n).red) {
                    self.set_red(xppr, false);
                    self.set_red(xp, false);
                    self.set_red(xpp, true);
                    x = xpp;
                } else {
                    let (mut xp, mut xpp) = (Some(xp), Some(xpp));
                    if self.node(xp.expect("parent")).right == Some(x) {
                        x = xp.expect("parent");
                        root = self.rotate_left(root, x);
                        xp = self.node(x).parent;
                        xpp = xp.and_then(|p| self.node(p).parent);
                    }
                    if let Some(xp) = xp {
                        self.set_red(xp, false);
                        if let Some(xpp) = xpp {
                            self.set_red(xpp, true);
                            root = self.rotate_right(root, xpp);
                        }
                    }
                }
            } else if let Some(xppl) = xppl.filter(|&n| self.node(n).red) {
                self.set_red(xppl, false);
                self.set_red(xp, false);
                self.set_red(xpp, true);
                x = xpp;
            } else {
                let (mut xp, mut xpp) = (Some(xp), Some(xpp));
                if self.node(xp.expect("parent")).left == Some(x) {
                    x = xp.expect("parent");
                    root = self.rotate_right(root, x);
                    xp = self.node(x).parent;
                    xpp = xp.and_then(|p| self.node(p).parent);
                }
                if let Some(xp) = xp {
                    self.set_red(xp, false);
                    if let Some(xpp) = xpp {
                        self.set_red(xpp, true);
                        root = self.rotate_left(root, xpp);
                    }
                }
            }
        }
    }

    /// `TreeNode.balanceDeletion`: returns the new root.
    fn balance_deletion(&mut self, mut root: usize, mut x: Option<usize>) -> usize {
        loop {
            let Some(xi) = x.filter(|&xi| xi != root) else {
                return root;
            };
            let Some(xp0) = self.node(xi).parent else {
                self.set_red(xi, false);
                return xi;
            };
            if self.node(xi).red {
                self.set_red(xi, false);
                return root;
            }
            let mut xp = Some(xp0);
            if self.node(xp0).left == Some(xi) {
                let mut xpr = self.node(xp0).right;
                if let Some(s) = xpr.filter(|&s| self.node(s).red) {
                    self.set_red(s, false);
                    self.set_red(xp0, true);
                    root = self.rotate_left(root, xp0);
                    xp = self.node(xi).parent;
                    xpr = xp.and_then(|p| self.node(p).right);
                }
                let Some(s) = xpr else {
                    x = xp;
                    continue;
                };
                let (sl, sr) = (self.node(s).left, self.node(s).right);
                if !self.is_red(sr) && !self.is_red(sl) {
                    self.set_red(s, true);
                    x = xp;
                    continue;
                }
                let mut xpr = Some(s);
                if !self.is_red(sr) {
                    if let Some(sl) = sl {
                        self.set_red(sl, false);
                    }
                    self.set_red(s, true);
                    root = self.rotate_right(root, s);
                    xp = self.node(xi).parent;
                    xpr = xp.and_then(|p| self.node(p).right);
                }
                if let Some(s) = xpr {
                    let red = self.is_red(xp);
                    self.set_red(s, red);
                    if let Some(sr) = self.node(s).right {
                        self.set_red(sr, false);
                    }
                }
                if let Some(xp) = xp {
                    self.set_red(xp, false);
                    root = self.rotate_left(root, xp);
                }
                x = Some(root);
            } else {
                let mut xpl = self.node(xp0).left;
                if let Some(s) = xpl.filter(|&s| self.node(s).red) {
                    self.set_red(s, false);
                    self.set_red(xp0, true);
                    root = self.rotate_right(root, xp0);
                    xp = self.node(xi).parent;
                    xpl = xp.and_then(|p| self.node(p).left);
                }
                let Some(s) = xpl else {
                    x = xp;
                    continue;
                };
                let (sl, sr) = (self.node(s).left, self.node(s).right);
                if !self.is_red(sl) && !self.is_red(sr) {
                    self.set_red(s, true);
                    x = xp;
                    continue;
                }
                let mut xpl = Some(s);
                if !self.is_red(sl) {
                    if let Some(sr) = sr {
                        self.set_red(sr, false);
                    }
                    self.set_red(s, true);
                    root = self.rotate_left(root, s);
                    xp = self.node(xi).parent;
                    xpl = xp.and_then(|p| self.node(p).left);
                }
                if let Some(s) = xpl {
                    let red = self.is_red(xp);
                    self.set_red(s, red);
                    if let Some(sl) = self.node(s).left {
                        self.set_red(sl, false);
                    }
                }
                if let Some(xp) = xp {
                    self.set_red(xp, false);
                    root = self.rotate_right(root, xp);
                }
                x = Some(root);
            }
        }
    }

    /// `TreeNode.moveRootToFront`.
    fn move_root_to_front(&mut self, idx: usize, root: usize) {
        let order = &mut self.table[idx].order;
        if order.first() != Some(&root) {
            let pos = order
                .iter()
                .position(|&i| i == root)
                .expect("the root is in its bin");
            order.remove(pos);
            order.insert(0, root);
        }
    }

    /// `TreeNode.removeTreeNode` (with `movable == true`) for entry `p` of tree bin `idx`.
    fn remove_tree_node(&mut self, idx: usize, p: usize) {
        let first = self.table[idx].order[0];
        let order = &mut self.table[idx].order;
        let pos = order
            .iter()
            .position(|&i| i == p)
            .expect("entry in its bin");
        order.remove(pos);
        if order.is_empty() {
            self.table[idx].tree = false;
            return;
        }
        let mut root = self.tree_root(first);
        let too_small = {
            let r = self.node(root);
            r.right.is_none() || r.left.is_none_or(|rl| self.node(rl).left.is_none())
        };
        if too_small {
            // `untreeify`: the remaining entries form a list in `next` order.
            self.table[idx].tree = false;
            return;
        }
        let (pl, pr) = (self.node(p).left, self.node(p).right);
        let replacement = if let (Some(pl), Some(pr)) = (pl, pr) {
            // Swap `p` with its successor `s` in the tree.
            let mut s = pr;
            while let Some(sl) = self.node(s).left {
                s = sl;
            }
            let (s_red, p_red) = (self.node(s).red, self.node(p).red);
            self.set_red(s, p_red);
            self.set_red(p, s_red);
            let sr = self.node(s).right;
            let pp = self.node(p).parent;
            if s == pr {
                self.node_mut(p).parent = Some(s);
                self.node_mut(s).right = Some(p);
            } else {
                let sp = self.node(s).parent;
                self.node_mut(p).parent = sp;
                if let Some(sp) = sp {
                    if self.node(sp).left == Some(s) {
                        self.node_mut(sp).left = Some(p);
                    } else {
                        self.node_mut(sp).right = Some(p);
                    }
                }
                self.node_mut(s).right = Some(pr);
                self.node_mut(pr).parent = Some(s);
            }
            self.node_mut(p).left = None;
            self.node_mut(p).right = sr;
            if let Some(sr) = sr {
                self.node_mut(sr).parent = Some(p);
            }
            self.node_mut(s).left = Some(pl);
            self.node_mut(pl).parent = Some(s);
            self.node_mut(s).parent = pp;
            match pp {
                None => root = s,
                Some(pp) => self.replace_child(pp, p, Some(s)),
            }
            sr.unwrap_or(p)
        } else {
            pl.or(pr).unwrap_or(p)
        };
        if replacement != p {
            let pp = self.node(p).parent;
            self.node_mut(replacement).parent = pp;
            match pp {
                None => {
                    root = replacement;
                    self.set_red(replacement, false);
                }
                Some(pp) => self.replace_child(pp, p, Some(replacement)),
            }
            let node = self.node_mut(p);
            node.left = None;
            node.right = None;
            node.parent = None;
        }
        let r = if self.node(p).red {
            root
        } else {
            self.balance_deletion(root, Some(replacement))
        };
        if replacement == p {
            // Detach `p`.
            if let Some(pp) = self.node_mut(p).parent.take() {
                if self.node(pp).left == Some(p) {
                    self.node_mut(pp).left = None;
                } else if self.node(pp).right == Some(p) {
                    self.node_mut(pp).right = None;
                }
            }
        }
        self.move_root_to_front(idx, r);
    }
}

impl<K: JavaHash, V> JavaHashMap<K, V> {
    /// Whether entry `x` goes to the left of entry `p` in a tree: the `dir <= 0` of
    /// `treeify` and `putTreeVal`, comparing hashes, then `compareTo`, then identity hashes.
    fn goes_left(&self, x: usize, p: usize) -> bool {
        let (x, p) = (self.node(x), self.node(p));
        match p.hash.cmp(&x.hash) {
            Ordering::Greater => true,
            Ordering::Less => false,
            // `tieBreakOrder` returns -1 when the new key's identity hash is not larger.
            Ordering::Equal => x.key.java_compare(&p.key) != Some(Ordering::Greater),
        }
    }

    /// Walks down from `root` and links `x` below the node where its search ends, returning
    /// that parent.
    fn link_into_tree(&mut self, root: usize, x: usize) -> usize {
        let mut p = root;
        loop {
            let left = self.goes_left(x, p);
            let child = if left {
                self.node(p).left
            } else {
                self.node(p).right
            };
            if let Some(c) = child {
                p = c;
                continue;
            }
            self.node_mut(x).parent = Some(p);
            if left {
                self.node_mut(p).left = Some(x);
            } else {
                self.node_mut(p).right = Some(x);
            }
            return p;
        }
    }

    /// `TreeNode.treeify`: builds a tree of bin `idx` from its entries in `next` order.
    fn treeify(&mut self, idx: usize) {
        let order = self.table[idx].order.clone();
        let mut root: Option<usize> = None;
        for x in order {
            let node = self.node_mut(x);
            node.left = None;
            node.right = None;
            root = Some(match root {
                None => {
                    node.parent = None;
                    node.red = false;
                    x
                }
                Some(r) => {
                    self.link_into_tree(r, x);
                    self.balance_insertion(r, x)
                }
            });
        }
        self.table[idx].tree = true;
        self.move_root_to_front(idx, root.expect("treeified bins are not empty"));
    }

    /// `TreeNode.putTreeVal` for a new entry `x`: links it into the tree and inserts it
    /// right after its tree parent in `next` order.
    fn put_tree_val(&mut self, idx: usize, x: usize) {
        let root = self.tree_root(self.table[idx].order[0]);
        let parent = self.link_into_tree(root, x);
        let order = &mut self.table[idx].order;
        let pos = order
            .iter()
            .position(|&i| i == parent)
            .expect("the parent is in the bin");
        order.insert(pos + 1, x);
        let root = self.balance_insertion(root, x);
        self.move_root_to_front(idx, root);
    }

    /// `HashMap.resize`: allocates the table or doubles its capacity, splitting every bin
    /// into a low and a high half that keep their relative `next` order.
    fn resize(&mut self) {
        let old_cap = self.table.len();
        let old_thr = self.threshold;
        let mut new_thr = 0;
        let new_cap = if old_cap > 0 {
            let new_cap = old_cap << 1;
            if old_cap >= DEFAULT_INITIAL_CAPACITY {
                new_thr = old_thr << 1;
            }
            new_cap
        } else if old_thr > 0 {
            old_thr
        } else {
            new_thr = (DEFAULT_INITIAL_CAPACITY as f32 * LOAD_FACTOR) as usize;
            DEFAULT_INITIAL_CAPACITY
        };
        if new_thr == 0 {
            new_thr = (new_cap as f32 * LOAD_FACTOR) as usize;
        }
        self.threshold = new_thr;

        let old_table = std::mem::replace(&mut self.table, vec![Bin::default(); new_cap]);
        for (j, bin) in old_table.into_iter().enumerate() {
            let (lo, hi): (Vec<usize>, Vec<usize>) = bin
                .order
                .into_iter()
                .partition(|&i| (self.node(i).hash as u32 as usize) & old_cap == 0);
            if !bin.tree {
                self.table[j].order = lo;
                self.table[j + old_cap].order = hi;
                continue;
            }
            // `TreeNode.split`.
            let (lo_empty, hi_empty) = (lo.is_empty(), hi.is_empty());
            for (dest, half, other_empty) in [(j, lo, hi_empty), (j + old_cap, hi, lo_empty)] {
                if half.is_empty() {
                    continue;
                }
                let keep_tree = half.len() > UNTREEIFY_THRESHOLD;
                self.table[dest] = Bin {
                    order: half,
                    tree: keep_tree,
                };
                // A half that holds the whole bin keeps the existing tree.
                if keep_tree && !other_empty {
                    self.treeify(dest);
                }
            }
        }
    }
}

impl<K: Eq + JavaHash, V> JavaHashMap<K, V> {
    fn find(&self, hash: i32, key: &K) -> Option<usize> {
        if self.table.is_empty() {
            return None;
        }
        self.table[self.bucket_index(hash)]
            .order
            .iter()
            .copied()
            .find(|&i| {
                let e = self.node(i);
                e.hash == hash && e.key == *key
            })
    }

    /// Equivalent of `HashMap#put`: replaces the value of an existing key in place,
    /// otherwise adds the key to its bucket.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = spread(key.java_hash());
        if self.table.is_empty() {
            self.resize();
        }
        if let Some(i) = self.find(hash, &key) {
            return Some(std::mem::replace(&mut self.node_mut(i).value, value));
        }
        let entry = Some(Entry {
            hash,
            key,
            value,
            parent: None,
            left: None,
            right: None,
            red: false,
        });
        let slot = if let Some(slot) = self.free.pop() {
            self.entries[slot] = entry;
            slot
        } else {
            self.entries.push(entry);
            self.entries.len() - 1
        };
        let idx = self.bucket_index(hash);
        if self.table[idx].tree {
            self.put_tree_val(idx, slot);
        } else {
            self.table[idx].order.push(slot);
            if self.table[idx].order.len() > TREEIFY_THRESHOLD {
                // `treeifyBin` resizes small tables instead of building a tree.
                if self.table.len() < MIN_TREEIFY_CAPACITY {
                    self.resize();
                } else {
                    self.treeify(idx);
                }
            }
        }
        self.len += 1;
        if self.len > self.threshold {
            self.resize();
        }
        None
    }

    /// Equivalent of `HashMap#remove`.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hash = spread(key.java_hash());
        let i = self.find(hash, key)?;
        let idx = self.bucket_index(hash);
        if self.table[idx].tree {
            self.remove_tree_node(idx, i);
        } else {
            self.table[idx].order.retain(|&j| j != i);
        }
        self.free.push(i);
        self.len -= 1;
        self.entries[i].take().map(|e| e.value)
    }

    /// Equivalent of `HashMap#get`.
    pub fn get(&self, key: &K) -> Option<&V> {
        let i = self.find(spread(key.java_hash()), key)?;
        Some(&self.node(i).value)
    }

    /// Equivalent of `HashMap#containsKey`.
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Equivalent of `new HashMap<>(other)`: pre-sizes the table for `other.len()` entries
    /// and inserts them in `other`'s iteration order.
    pub fn copy_of(other: &Self) -> Self
    where
        K: Clone,
        V: Clone,
    {
        let mut map = Self::new();
        let s = other.len();
        if s > 0 {
            let t = (s as f64 / f64::from(LOAD_FACTOR)).ceil() as usize;
            map.threshold = table_size_for(t);
            for (k, v) in other.iter() {
                map.insert(k.clone(), v.clone());
            }
        }
        map
    }
}

impl<K: Eq + JavaHash, V> FromIterator<(K, V)> for JavaHashMap<K, V> {
    /// Inserts the pairs into a `new HashMap<>()` one by one.
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

/// Reorders `items` into the iteration order of a `new HashMap<>()` into which the items'
/// keys were inserted in the given order. Later duplicates replace the value but keep the
/// position of the first occurrence, just like `HashMap#put`.
pub fn java_hash_order<K: Eq + JavaHash, V>(
    items: impl IntoIterator<Item = (K, V)>,
) -> Vec<(K, V)> {
    items
        .into_iter()
        .collect::<JavaHashMap<K, V>>()
        .into_entries()
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<K, V> JavaHashMap<K, V> {
        /// Checks the red-black and linkage invariants of every tree bin, returning the
        /// number of tree bins.
        fn check_trees(&self) -> usize {
            let mut trees = 0;
            for bin in self.table.iter().filter(|b| b.tree) {
                trees += 1;
                let root = bin.order[0];
                assert_eq!(self.node(root).parent, None, "the root is first");
                assert!(!self.node(root).red, "the root is black");
                let mut count = 0;
                self.check_subtree(Some(root), None, &mut count);
                assert_eq!(count, bin.order.len(), "the tree holds the whole bin");
            }
            trees
        }

        /// Returns the black height of the subtree.
        fn check_subtree(
            &self,
            i: Option<usize>,
            parent: Option<usize>,
            count: &mut usize,
        ) -> usize {
            let Some(i) = i else { return 1 };
            *count += 1;
            let n = self.node(i);
            assert_eq!(n.parent, parent);
            if n.red {
                assert!(!self.is_red(n.left) && !self.is_red(n.right), "no red-red");
            }
            let l = self.check_subtree(n.left, Some(i), count);
            let r = self.check_subtree(n.right, Some(i), count);
            assert_eq!(l, r, "equal black heights");
            l + usize::from(!n.red)
        }
    }

    #[test]
    fn small_string_maps_follow_bucket_order() {
        let map: JavaHashMap<String, i32> = ["R", "D", "U"]
            .iter()
            .map(|s| ((*s).to_owned(), 0))
            .collect();
        // "D" = 68 -> bucket 4, "R" = 82 -> bucket 2, "U" = 85 -> bucket 5.
        let keys: Vec<&str> = map.keys().map(String::as_str).collect();
        assert_eq!(keys, ["R", "D", "U"]);
    }

    #[test]
    fn insert_replaces_in_place() {
        let mut map = JavaHashMap::new();
        assert_eq!(map.insert(1, "a"), None);
        assert_eq!(map.insert(17, "b"), None);
        assert_eq!(map.insert(1, "c"), Some("a"));
        assert_eq!(map.iter().collect::<Vec<_>>(), [(&1, &"c"), (&17, &"b")]);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn remove_and_reuse_slots() {
        let mut map = JavaHashMap::new();
        for i in 0..40 {
            map.insert(i, i * 2);
        }
        for i in (0..40).step_by(3) {
            assert_eq!(map.remove(&i), Some(i * 2));
        }
        assert_eq!(map.remove(&0), None);
        map.insert(100, 1);
        assert_eq!(map.get(&100), Some(&1));
        assert!(!map.contains_key(&3));
        assert_eq!(map.len(), 40 - 14 + 1);
    }

    #[test]
    fn resizes_keep_integer_keys_sorted_modulo_capacity() {
        let map: JavaHashMap<i32, ()> = (0..100).rev().map(|i| (i, ())).collect();
        let keys: Vec<i32> = map.keys().copied().collect();
        assert_eq!(keys, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn hash_order_dedups() {
        let items = vec![
            ("a".to_owned(), 1),
            ("b".to_owned(), 2),
            ("a".to_owned(), 3),
        ];
        assert_eq!(
            java_hash_order(items),
            [("a".to_owned(), 3), ("b".to_owned(), 2)]
        );
    }

    #[test]
    fn copy_constructor_sizing() {
        // Three entries are copied into a table of capacity 4 (JDK 19+ sizing), which can
        // change the iteration order relative to the original 16-bucket table.
        let map: JavaHashMap<i32, ()> = [3, 5, 16].into_iter().map(|i| (i, ())).collect();
        assert_eq!(map.keys().copied().collect::<Vec<_>>(), [16, 3, 5]);
        let copy = JavaHashMap::copy_of(&map);
        assert_eq!(copy.keys().copied().collect::<Vec<_>>(), [16, 5, 3]);
        assert!(JavaHashMap::<i32, ()>::copy_of(&JavaHashMap::new()).is_empty());
    }

    #[test]
    fn colliding_keys_are_treeified() {
        // All keys share bucket 0 of a 64-entry table: the 9th key grows the table to 32,
        // the 10th to 64, and the 11th treeifies the bin.
        let mut map = JavaHashMap::new();
        for i in 0..11 {
            map.insert(i * 64, i);
        }
        assert_eq!(map.table.len(), 64);
        assert_eq!(map.check_trees(), 1);
        // The root moved to the front of the bin; everything else kept insertion order.
        let keys: Vec<i32> = map.keys().copied().collect();
        assert_eq!(keys, [192, 0, 64, 128, 256, 320, 384, 448, 512, 576, 640]);
        for i in 0..11 {
            assert_eq!(map.get(&(i * 64)), Some(&i));
        }
    }

    #[test]
    fn tree_bins_survive_inserts_and_removals() {
        let mut map = JavaHashMap::new();
        let mut state = 12345_u32;
        let mut next = || {
            state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
            state >> 8
        };
        for _ in 0..3000 {
            let key = ((next() % 500) * 128) as i32;
            if next() % 3 == 0 {
                map.remove(&key);
            } else {
                map.insert(key, ());
            }
            map.check_trees();
        }
    }
}
