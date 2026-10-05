//! A port of `java.util.PriorityQueue`.
//!
//! The 4x4x4 solver keeps its best phase 1 candidates in a priority queue and later reads the
//! queue's backing array directly (`toArray`), so the exact heap layout matters.

use std::cmp::Ordering;

/// A binary min-heap (with respect to `cmp`) laid out exactly like Java's `PriorityQueue`.
#[derive(Debug, Clone)]
pub struct JavaPriorityQueue<T> {
    queue: Vec<T>,
    cmp: fn(&T, &T) -> Ordering,
}

impl<T> JavaPriorityQueue<T> {
    /// Equivalent of `new PriorityQueue<>(initialCapacity, comparator)`.
    pub fn with_capacity(capacity: usize, cmp: fn(&T, &T) -> Ordering) -> Self {
        Self {
            queue: Vec::with_capacity(capacity),
            cmp,
        }
    }

    /// The number of elements.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Equivalent of `PriorityQueue#clear`.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// The backing array, equivalent of `PriorityQueue#toArray`.
    pub fn as_slice(&self) -> &[T] {
        &self.queue
    }

    /// Equivalent of `PriorityQueue#add` (`siftUpUsingComparator`).
    pub fn push(&mut self, x: T) {
        self.queue.push(x);
        let mut k = self.queue.len() - 1;
        while k > 0 {
            let parent = (k - 1) >> 1;
            if (self.cmp)(&self.queue[k], &self.queue[parent]) != Ordering::Less {
                break;
            }
            self.queue.swap(k, parent);
            k = parent;
        }
    }

    /// Equivalent of `PriorityQueue#poll` (`siftDownUsingComparator`).
    pub fn pop(&mut self) -> Option<T> {
        let last = self.queue.pop()?;
        if self.queue.is_empty() {
            return Some(last);
        }
        let result = std::mem::replace(&mut self.queue[0], last);
        let n = self.queue.len();
        let half = n >> 1;
        let mut k = 0;
        while k < half {
            let mut child = (k << 1) + 1;
            let right = child + 1;
            if right < n && (self.cmp)(&self.queue[child], &self.queue[right]) == Ordering::Greater
            {
                child = right;
            }
            if (self.cmp)(&self.queue[k], &self.queue[child]) != Ordering::Greater {
                break;
            }
            self.queue.swap(k, child);
            k = child;
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaves_like_a_min_heap() {
        let mut q = JavaPriorityQueue::with_capacity(4, i32::cmp);
        for v in [5, 1, 4, 1, 3, 9, 2] {
            q.push(v);
        }
        assert_eq!(q.len(), 7);
        assert_eq!(q.as_slice()[0], 1);
        let mut out = Vec::new();
        while let Some(v) = q.pop() {
            out.push(v);
        }
        assert_eq!(out, [1, 1, 2, 3, 4, 5, 9]);
        assert!(q.is_empty());
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn heap_layout_matches_java() {
        // Java: PriorityQueue<Integer> with offers 5,1,4,1,3,9,2 has toArray [1, 1, 2, 5, 3, 9, 4].
        let mut q = JavaPriorityQueue::with_capacity(4, i32::cmp);
        for v in [5, 1, 4, 1, 3, 9, 2] {
            q.push(v);
        }
        assert_eq!(q.as_slice(), [1, 1, 2, 5, 3, 9, 4]);
        q.clear();
        assert!(q.is_empty());
    }
}
