//! Java's `hashCode` contracts for the types TNoodle hashes.
//!
//! Hash codes matter for output because TNoodle iterates `HashMap`s keyed by strings and
//! puzzle states, and Java's iteration order is a function of the keys' hash codes.

use std::cmp::Ordering;

/// A value with a Java `hashCode()`.
pub trait JavaHash {
    /// The value `hashCode()` would return in Java.
    fn java_hash(&self) -> i32;

    /// `compareTo` if the Java type is `Comparable`, `None` otherwise. `HashMap` uses it to
    /// arrange keys with equal hash codes in treeified bins.
    fn java_compare(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

/// `String#compareTo`: lexicographic over UTF-16 code units.
pub fn string_compare(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// `String#hashCode()`: `s[0]*31^(n-1) + ... + s[n-1]` over UTF-16 code units.
pub fn string_hash(s: &str) -> i32 {
    s.encode_utf16()
        .fold(0_i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

/// `java.util.Arrays#hashCode(int[])` and friends: `31 * h + e` starting from `1`.
///
/// The same recurrence implements `Arrays.deepHashCode` (feed it the hashes of the
/// sub-arrays) and `Objects.hash` (feed it the hashes of the arguments).
pub fn array_hash<I>(values: I) -> i32
where
    I: IntoIterator,
    I::Item: Into<i32>,
{
    values
        .into_iter()
        .fold(1_i32, |h, v| h.wrapping_mul(31).wrapping_add(v.into()))
}

/// `Arrays.deepHashCode` of a rectangular `int[rows][cols]` stored row-major.
pub fn deep_hash_2d(values: &[u8], cols: usize) -> i32 {
    array_hash(
        values
            .chunks(cols)
            .map(|row| array_hash(row.iter().map(|&v| i32::from(v)))),
    )
}

impl JavaHash for str {
    fn java_hash(&self) -> i32 {
        string_hash(self)
    }

    fn java_compare(&self, other: &Self) -> Option<Ordering> {
        Some(string_compare(self, other))
    }
}

impl JavaHash for String {
    fn java_hash(&self) -> i32 {
        string_hash(self)
    }

    fn java_compare(&self, other: &Self) -> Option<Ordering> {
        Some(string_compare(self, other))
    }
}

impl JavaHash for i32 {
    /// `Integer#hashCode()` is the value itself.
    fn java_hash(&self) -> i32 {
        *self
    }

    fn java_compare(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: JavaHash + ?Sized> JavaHash for &T {
    fn java_hash(&self) -> i32 {
        (**self).java_hash()
    }

    fn java_compare(&self, other: &Self) -> Option<Ordering> {
        (**self).java_compare(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_hashes() {
        assert_eq!(string_hash(""), 0);
        assert_eq!(string_hash("a"), 97);
        assert_eq!(string_hash("Aa"), string_hash("BB"));
        // Non-BMP characters hash as their surrogate pairs.
        assert_eq!(string_hash("😀"), 0xD83D * 31 + 0xDE00);
    }

    #[test]
    fn string_comparison_uses_utf16() {
        assert_eq!(string_compare("a", "b"), Ordering::Less);
        assert_eq!(string_compare("ab", "a"), Ordering::Greater);
        // U+FF01 sorts before U+1F600 in UTF-8, but after its high surrogate in UTF-16.
        assert_eq!(string_compare("\u{FF01}", "\u{1F600}"), Ordering::Greater);
        assert_eq!("x".java_compare("x"), Some(Ordering::Equal));
        assert_eq!(3.java_compare(&4), Some(Ordering::Less));
    }

    #[test]
    fn array_hashes() {
        assert_eq!(array_hash(Vec::<i32>::new()), 1);
        assert_eq!(array_hash([1, 2]), (31 + 1) * 31 + 2);
        assert_eq!(
            deep_hash_2d(&[1, 2, 3, 4], 2),
            array_hash([array_hash([1, 2]), array_hash([3, 4])])
        );
    }
}
