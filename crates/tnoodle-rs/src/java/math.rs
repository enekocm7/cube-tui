//! The trigonometric functions used when TNoodle draws puzzles.
//!
//! The Java specification only requires `Math.sin`/`Math.cos` to be within one ulp, and real
//! JVMs differ: HotSpot on x86-64 uses Intel-derived intrinsics while other platforms use
//! fdlibm. The x86-64 intrinsics return the correctly rounded result for almost every input,
//! so the closest platform independent match is a correctly rounded implementation, which is
//! what [`sin`] and [`cos`] provide (via double-double arithmetic). `Math.acos` is not
//! intrinsified and matches fdlibm, which [`acos`] uses.

/// A double-double: an unevaluated sum `hi + lo` with `|lo| <= ulp(hi) / 2`.
#[derive(Debug, Clone, Copy)]
struct Dd {
    hi: f64,
    lo: f64,
}

/// π/2 split into four doubles whose exact sum is π/2 to about 210 bits.
const PIO2: [f64; 4] = [
    f64::from_bits(0x3ff9_21fb_5444_2d18),
    f64::from_bits(0x3c91_a626_3314_5c07),
    f64::from_bits(0xb91f_1976_b7ed_8fbc),
    f64::from_bits(0x35b4_cf98_e804_177d),
];

/// Above this magnitude the simple Cody-Waite style reduction loses accuracy; such inputs
/// never occur in TNoodle and fall back to fdlibm.
const REDUCTION_LIMIT: f64 = 1e9;

fn two_sum(a: f64, b: f64) -> Dd {
    let s = a + b;
    let bb = s - a;
    Dd {
        hi: s,
        lo: (a - (s - bb)) + (b - bb),
    }
}

fn quick_two_sum(a: f64, b: f64) -> Dd {
    let s = a + b;
    Dd {
        hi: s,
        lo: b - (s - a),
    }
}

fn two_prod(a: f64, b: f64) -> Dd {
    let p = a * b;
    Dd {
        hi: p,
        lo: libm::fma(a, b, -p),
    }
}

impl Dd {
    const fn new(v: f64) -> Self {
        Self { hi: v, lo: 0.0 }
    }

    fn add(self, y: Self) -> Self {
        let s = two_sum(self.hi, y.hi);
        let t = two_sum(self.lo, y.lo);
        let s = quick_two_sum(s.hi, s.lo + t.hi);
        quick_two_sum(s.hi, s.lo + t.lo)
    }

    fn neg(self) -> Self {
        Self {
            hi: -self.hi,
            lo: -self.lo,
        }
    }

    fn mul(self, y: Self) -> Self {
        let p = two_prod(self.hi, y.hi);
        quick_two_sum(p.hi, p.lo + (self.hi * y.lo + self.lo * y.hi))
    }

    fn div_f64(self, d: f64) -> Self {
        let q1 = self.hi / d;
        let p = two_prod(q1, d);
        let s = two_sum(self.hi, -p.hi);
        let e = s.lo - p.lo + self.lo;
        let q2 = (s.hi + e) / d;
        quick_two_sum(q1, q2)
    }
}

/// Returns `(sin x, cos x)` in double-double precision, or `None` if `x` is out of the
/// supported range.
fn sin_cos_dd(x: f64) -> Option<(Dd, Dd)> {
    if !x.is_finite() || x.abs() > REDUCTION_LIMIT {
        return None;
    }
    let k = (x * std::f64::consts::FRAC_2_PI).round();
    let mut r = Dd::new(x);
    for p in PIO2 {
        r = r.add(two_prod(k, p).neg());
    }
    let s = sin_series(r);
    let c = cos_series(r);
    let q = (k as i64).rem_euclid(4);
    Some(match q {
        0 => (s, c),
        1 => (c, s.neg()),
        2 => (s.neg(), c.neg()),
        _ => (c.neg(), s),
    })
}

/// `sin(r) = r - r^3/3! + r^5/5! - ...` for `|r| <= π/4`.
fn sin_series(r: Dd) -> Dd {
    taylor(r, 1)
}

/// `cos(r) = 1 - r^2/2! + r^4/4! - ...` for `|r| <= π/4`.
fn cos_series(r: Dd) -> Dd {
    taylor(r, 0)
}

/// `Σ_{n>=0} (-1)^n r^(2n + offset) / (2n + offset)!`, evaluated with Horner's rule.
fn taylor(r: Dd, offset: u32) -> Dd {
    const TERMS: u32 = 16;
    let r2 = r.mul(r);
    let mut coefficients = Vec::with_capacity(TERMS as usize + 1);
    let mut f = Dd::new(1.0);
    if offset == 0 {
        coefficients.push(f);
    }
    for k in 1..=(2 * TERMS + offset) {
        f = f.div_f64(f64::from(k));
        if k % 2 == offset % 2 {
            coefficients.push(f);
        }
    }
    let mut acc = Dd::new(0.0);
    for (n, c) in coefficients.iter().enumerate().rev() {
        let c = if n % 2 == 1 { c.neg() } else { *c };
        acc = acc.mul(r2).add(c);
    }
    if offset == 1 { acc.mul(r) } else { acc }
}

/// `Math.sin`, correctly rounded for `|x| <= 1e9` (fdlibm beyond).
pub fn sin(x: f64) -> f64 {
    if x == 0.0 {
        return x;
    }
    match sin_cos_dd(x) {
        Some((s, _)) => s.hi + s.lo,
        None => libm::sin(x),
    }
}

/// `Math.cos`, correctly rounded for `|x| <= 1e9` (fdlibm beyond).
pub fn cos(x: f64) -> f64 {
    match sin_cos_dd(x) {
        Some((_, c)) => c.hi + c.lo,
        None => libm::cos(x),
    }
}

/// `Math.acos` (fdlibm).
pub fn acos(x: f64) -> f64 {
    libm::acos(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_6, PI};

    #[test]
    fn exact_values() {
        assert_eq!(sin(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(sin(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(cos(0.0), 1.0);
        assert_eq!(sin(FRAC_PI_2), 1.0);
        assert_eq!(cos(PI), -1.0);
        // `FRAC_PI_6` is the double nearest to π/6, just above it; Java's `toRadians(30)` is
        // the double just below it.
        assert_eq!(sin(FRAC_PI_6), 0.5);
        assert_eq!(sin(30_f64.to_radians()), 0.49999999999999994);
        assert!(sin(f64::NAN).is_nan());
        assert!(cos(f64::INFINITY).is_nan());
        assert_eq!(sin(1e12), libm::sin(1e12));
        assert_eq!(acos(1.0), 0.0);
    }

    #[test]
    fn close_to_fdlibm_everywhere() {
        let mut x = -50.0;
        while x < 50.0 {
            assert!(
                (sin(x) - libm::sin(x)).abs() <= 2.0 * f64::EPSILON,
                "sin({x})"
            );
            assert!(
                (cos(x) - libm::cos(x)).abs() <= 2.0 * f64::EPSILON,
                "cos({x})"
            );
            x += 0.0137;
        }
    }
}
