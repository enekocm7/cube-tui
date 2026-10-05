//! Faithful re-implementations of the Java platform behaviour that TNoodle's output depends on.
//!
//! TNoodle is specified by its Java implementation. Reproducing its scrambles and drawings
//! bit for bit requires reproducing `java.util.Random`, the `SHA1PRNG` secure random,
//! `HashMap` iteration order, `String.hashCode`, `PriorityQueue` heap layout and
//! `Double.toString` formatting. They live here so the rest of the crate can stay idiomatic.

mod hash;
mod hash_map;
mod math;
mod number;
mod priority_queue;
mod random;

pub use hash::{JavaHash, array_hash, deep_hash_2d, string_compare, string_hash};
pub use hash_map::{JavaHashMap, java_hash_order};
pub use math::{acos, cos, sin};
pub use number::{double_to_string, round};
pub use priority_queue::JavaPriorityQueue;
pub use random::{JavaRandom, RandomSource, Sha1Prng};

/// Equivalent of `Puzzle.choose(Random, Iterable)`: reservoir sampling that consumes one
/// `nextInt` per element, so the result depends on the iteration order of `items`.
///
/// Returns `None` if `items` is empty.
pub fn choose<T>(r: &mut dyn RandomSource, items: impl IntoIterator<Item = T>) -> Option<T> {
    let mut chosen = None;
    let mut count = 0;
    for item in items {
        count += 1;
        if r.next_int_bounded(count) == 0 {
            chosen = Some(item);
        }
    }
    chosen
}

/// Equivalent of Java's `String#split(regex)` for a literal, non-empty separator: trailing
/// empty strings are removed.
pub(crate) fn split_dropping_trailing_empty<'a>(s: &'a str, separator: &str) -> Vec<&'a str> {
    if s.is_empty() {
        return vec![""];
    }
    let mut parts: Vec<&str> = s.split(separator).collect();
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_is_uniform_reservoir_sampling() {
        let mut r = JavaRandom::new(1);
        assert_eq!(choose(&mut r, Vec::<i32>::new()), None);
        assert_eq!(choose(&mut r, [7]), Some(7));
        let mut counts = [0; 3];
        for _ in 0..3000 {
            counts[choose(&mut r, 0..3).unwrap()] += 1;
        }
        assert!(
            counts.iter().all(|&c| (800..1200).contains(&c)),
            "{counts:?}"
        );
    }

    #[test]
    fn split_like_java() {
        assert_eq!(split_dropping_trailing_empty("a,b,,", ","), ["a", "b"]);
        assert_eq!(split_dropping_trailing_empty(",a", ","), ["", "a"]);
        assert_eq!(split_dropping_trailing_empty("", ","), [""]);
        assert_eq!(split_dropping_trailing_empty(",,", ","), [] as [&str; 0]);
    }
}
