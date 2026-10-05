//! Bit-exact ports of `java.util.Random` and the `SHA1PRNG` `SecureRandom`.
//!
//! TNoodle derives every scramble from a `java.util.Random`, so reproducing a Java scramble
//! from the same seed requires reproducing the exact stream of numbers Java hands out.

use sha1::{Digest, Sha1};

/// A source of randomness with exactly the semantics of `java.util.Random`'s public methods.
///
/// Implementors only provide [`next_bits`](RandomSource::next_bits) (Java's protected
/// `next(int)`); the derived methods replicate the JDK algorithms, so any implementor that
/// matches a JDK generator's `next(int)` also matches all of its derived values.
pub trait RandomSource {
    /// Equivalent of `java.util.Random#next(int bits)`: the next `bits` (1..=32) random bits.
    fn next_bits(&mut self, bits: u32) -> i32;

    /// Equivalent of `java.util.Random#nextInt()`.
    fn next_int(&mut self) -> i32 {
        self.next_bits(32)
    }

    /// Equivalent of `java.util.Random#nextInt(int bound)`: uniform in `0..bound`.
    ///
    /// # Panics
    ///
    /// Panics if `bound` is not positive, like Java throws `IllegalArgumentException`.
    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let mut r = self.next_bits(31);
        let m = bound - 1;
        if bound & m == 0 {
            // The bound is a power of two.
            return ((i64::from(bound) * i64::from(r)) >> 31) as i32;
        }
        let mut u = r;
        loop {
            r = u % bound;
            if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                return r;
            }
            u = self.next_bits(31);
        }
    }

    /// Equivalent of `java.util.Random#nextLong()`.
    fn next_long(&mut self) -> i64 {
        let high = i64::from(self.next_bits(32)) << 32;
        high.wrapping_add(i64::from(self.next_bits(32)))
    }

    /// Equivalent of `java.util.Random#nextBoolean()`.
    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    /// Equivalent of `java.util.Random#nextDouble()`: uniform in `[0, 1)`.
    fn next_double(&mut self) -> f64 {
        let high = i64::from(self.next_bits(26)) << 27;
        let value = high + i64::from(self.next_bits(27));
        value as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// Equivalent of `java.util.Random#nextFloat()`: uniform in `[0, 1)`.
    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1_u32 << 24) as f32
    }

    /// Equivalent of `java.util.Random#nextBytes(byte[])`.
    fn next_bytes(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(4) {
            let mut rnd = self.next_int();
            for byte in chunk {
                *byte = rnd as u8;
                rnd >>= 8;
            }
        }
    }
}

impl<R: RandomSource + ?Sized> RandomSource for &mut R {
    fn next_bits(&mut self, bits: u32) -> i32 {
        (**self).next_bits(bits)
    }

    fn next_bytes(&mut self, bytes: &mut [u8]) {
        (**self).next_bytes(bytes);
    }
}

impl<R: RandomSource + ?Sized> RandomSource for Box<R> {
    fn next_bits(&mut self, bits: u32) -> i32 {
        (**self).next_bits(bits)
    }

    fn next_bytes(&mut self, bytes: &mut [u8]) {
        (**self).next_bytes(bytes);
    }
}

/// Fills `buf` with operating system entropy.
///
/// # Panics
///
/// Panics if the operating system cannot provide randomness. Scrambles must never be
/// generated from a predictable source, so there is no sensible fallback.
pub(crate) fn os_entropy(buf: &mut [u8]) {
    getrandom::fill(buf).expect("the operating system failed to provide entropy");
}

/// A bit-exact port of `java.util.Random`, the 48-bit linear congruential generator.
///
/// ```
/// use tnoodle::java::{JavaRandom, RandomSource};
///
/// // Same values as `new java.util.Random(42).nextInt(10)` in Java.
/// let mut r = JavaRandom::new(42);
/// assert_eq!(r.next_int_bounded(10), 0);
/// assert_eq!(r.next_int_bounded(10), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaRandom {
    seed: i64,
}

impl JavaRandom {
    const MULTIPLIER: i64 = 0x5_DEEC_E66D;
    const ADDEND: i64 = 0xB;
    const MASK: i64 = (1 << 48) - 1;

    /// Equivalent of `new java.util.Random(seed)`.
    pub fn new(seed: i64) -> Self {
        let mut random = Self { seed: 0 };
        random.set_seed(seed);
        random
    }

    /// Creates a generator seeded from operating system entropy, the counterpart of
    /// `new java.util.Random()`.
    pub fn from_entropy() -> Self {
        let mut seed = [0_u8; 8];
        os_entropy(&mut seed);
        Self::new(i64::from_le_bytes(seed))
    }

    /// Equivalent of `java.util.Random#setSeed(long)`.
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ Self::MULTIPLIER) & Self::MASK;
    }
}

impl RandomSource for JavaRandom {
    fn next_bits(&mut self, bits: u32) -> i32 {
        debug_assert!((1..=32).contains(&bits));
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND)
            & Self::MASK;
        ((self.seed as u64) >> (48 - bits)) as i32
    }
}

/// A bit-exact port of the JDK's `SHA1PRNG` `SecureRandom` (`sun.security.provider.SecureRandom`).
///
/// TNoodle seeds this generator from a user supplied string to produce "seeded scrambles"
/// that are reproducible across machines. When it is created with
/// [`from_entropy`](Sha1Prng::from_entropy) it is seeded from the operating system, like an
/// unseeded `SecureRandom.getInstance("SHA1PRNG")`.
#[derive(Clone)]
pub struct Sha1Prng {
    state: Option<[u8; Self::DIGEST_SIZE]>,
    remainder: [u8; Self::DIGEST_SIZE],
    rem_count: usize,
}

impl std::fmt::Debug for Sha1Prng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the internal state of a cryptographic generator.
        f.debug_struct("Sha1Prng").finish_non_exhaustive()
    }
}

impl Default for Sha1Prng {
    fn default() -> Self {
        Self::from_entropy()
    }
}

impl Sha1Prng {
    const DIGEST_SIZE: usize = 20;

    /// Equivalent of `SecureRandom.getInstance("SHA1PRNG")` followed by `setSeed(seed)`.
    ///
    /// Because the seed is set before any output is requested the generator is fully
    /// deterministic, which is what TNoodle's seeded scrambles rely on.
    pub fn with_seed(seed: &[u8]) -> Self {
        let mut random = Self::unseeded();
        random.set_seed(seed);
        random
    }

    /// A generator seeded from operating system entropy.
    pub fn from_entropy() -> Self {
        let mut seed = [0_u8; Self::DIGEST_SIZE];
        os_entropy(&mut seed);
        Self::with_seed(&seed)
    }

    const fn unseeded() -> Self {
        Self {
            state: None,
            remainder: [0; Self::DIGEST_SIZE],
            rem_count: 0,
        }
    }

    /// Equivalent of `SecureRandom#setSeed(byte[])`: supplements (rather than replaces) the
    /// current seed.
    pub fn set_seed(&mut self, seed: &[u8]) {
        let mut digest = Sha1::new();
        if let Some(state) = self.state {
            digest.update(state);
        }
        digest.update(seed);
        self.state = Some(digest.finalize().into());
        self.rem_count = 0;
    }

    /// Adds `output + 1` to `state`, treating both as little-endian byte strings, exactly
    /// like `sun.security.provider.SecureRandom#updateState`.
    fn update_state(state: &mut [u8; Self::DIGEST_SIZE], output: &[u8; Self::DIGEST_SIZE]) {
        let mut last = 1_i32;
        let mut changed = false;
        for (s, &o) in state.iter_mut().zip(output) {
            let v = i32::from(*s as i8) + i32::from(o as i8) + last;
            let t = v as u8;
            changed |= *s != t;
            *s = t;
            last = v >> 8;
        }
        if !changed {
            state[0] = state[0].wrapping_add(1);
        }
    }
}

impl RandomSource for Sha1Prng {
    /// Equivalent of `SecureRandom#next(int)`, which is built on top of `nextBytes`.
    fn next_bits(&mut self, bits: u32) -> i32 {
        debug_assert!((1..=32).contains(&bits));
        let num_bytes = bits.div_ceil(8) as usize;
        let mut bytes = [0_u8; 4];
        self.next_bytes(&mut bytes[..num_bytes]);
        let mut next = 0_i32;
        for &b in &bytes[..num_bytes] {
            next = (next << 8) + i32::from(b);
        }
        ((next as u32) >> (num_bytes as u32 * 8 - bits)) as i32
    }

    fn next_bytes(&mut self, result: &mut [u8]) {
        let state = self.state.get_or_insert_with(|| {
            let mut seed = [0_u8; Self::DIGEST_SIZE];
            os_entropy(&mut seed);
            Sha1::digest(seed).into()
        });
        let mut index = 0;
        let mut output = self.remainder;

        let mut r = self.rem_count;
        if r > 0 {
            let todo = (result.len() - index).min(Self::DIGEST_SIZE - r);
            for byte in &mut result[..todo] {
                *byte = output[r];
                output[r] = 0;
                r += 1;
            }
            self.rem_count += todo;
            index += todo;
        }

        while index < result.len() {
            output = Sha1::digest(*state).into();
            Self::update_state(state, &output);
            let todo = (result.len() - index).min(Self::DIGEST_SIZE);
            for out in output.iter_mut().take(todo) {
                result[index] = *out;
                index += 1;
                *out = 0;
            }
            self.rem_count += todo;
        }

        self.remainder = output;
        self.rem_count %= Self::DIGEST_SIZE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_random_matches_known_values() {
        // new Random(0).nextInt() == -1155484576 in every JDK.
        assert_eq!(JavaRandom::new(0).next_int(), -1_155_484_576);
        assert_eq!(JavaRandom::new(42).next_int_bounded(10), 0);
    }

    #[test]
    #[should_panic(expected = "bound must be positive")]
    fn bounded_requires_positive_bound() {
        JavaRandom::new(1).next_int_bounded(0);
    }

    #[test]
    fn power_of_two_bounds_use_high_bits() {
        let mut a = JavaRandom::new(7);
        let mut b = JavaRandom::new(7);
        for _ in 0..100 {
            let high = b.next_bits(31);
            assert_eq!(a.next_int_bounded(16), high >> 27);
        }
    }

    #[test]
    fn sha1prng_is_deterministic_when_seeded() {
        let mut a = Sha1Prng::with_seed(b"tnoodle");
        let mut b = Sha1Prng::with_seed(b"tnoodle");
        for bound in 1..200 {
            assert_eq!(a.next_int_bounded(bound), b.next_int_bounded(bound));
        }
    }

    #[test]
    fn sha1prng_debug_hides_state() {
        let r = Sha1Prng::with_seed(b"secret");
        assert_eq!(format!("{r:?}"), "Sha1Prng { .. }");
    }

    #[test]
    fn entropy_generators_differ() {
        let a: Vec<i32> = (0..4)
            .map(|_| JavaRandom::from_entropy().next_int())
            .collect();
        assert!(a.windows(2).any(|w| w[0] != w[1]));
        let mut s = Sha1Prng::from_entropy();
        let mut t = Sha1Prng::from_entropy();
        assert_ne!(s.next_long(), t.next_long());
    }
}
