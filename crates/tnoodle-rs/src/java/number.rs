//! Java's number formatting and rounding, as used when TNoodle renders SVG attributes.

/// Formats `value` exactly like Java's `Double.toString(double)` (JDK 19 and later).
///
/// Java prints the shortest decimal that uniquely identifies the double. Values in
/// `[1e-3, 1e7)` use plain notation with at least one fractional digit (`1.0`, `0.001`);
/// everything else uses computerized scientific notation (`1.0E7`, `4.9E-324`).
pub fn double_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    if value == 0.0 {
        return format!("{sign}0.0");
    }
    let abs = value.abs();
    let (digits, exponent) = shortest_digits(abs);

    let body = if (1e-3..1e7).contains(&abs) {
        if exponent >= 0 {
            let int_len = exponent as usize + 1;
            if digits.len() > int_len {
                format!("{}.{}", &digits[..int_len], &digits[int_len..])
            } else {
                format!("{digits}{}.0", "0".repeat(int_len - digits.len()))
            }
        } else {
            format!("0.{}{digits}", "0".repeat((-exponent - 1) as usize))
        }
    } else {
        let fraction = if digits.len() > 1 { &digits[1..] } else { "0" };
        format!("{}.{fraction}E{exponent}", &digits[..1])
    };
    format!("{sign}{body}")
}

/// Splits a `{:e}` formatted float into its significant digits and decimal exponent.
fn split_scientific(formatted: &str) -> (String, i32) {
    let (mantissa, exponent) = formatted
        .split_once('e')
        .expect("scientific notation contains an exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent = exponent.parse().expect("exponent is an integer");
    (digits, exponent)
}

/// The shortest decimal digits that round-trip to `abs` (a positive finite double), and the
/// exponent `e` such that `abs ≈ d.ddd × 10^e`, selected with Java's tie-breaking rules.
fn shortest_digits(abs: f64) -> (String, i32) {
    let (mut digits, mut exponent) = split_scientific(&format!("{abs:e}"));
    // Among the shortest decimals that round-trip, Java picks the one closest to the exact
    // binary value, breaking ties towards an even last digit. Rust's exact formatting rounds
    // the exact value half-to-even, so re-rounding to the same length selects Java's choice
    // whenever that decimal still round-trips.
    let exact = format!("{abs:.prec$e}", prec = digits.len() - 1);
    if exact.parse::<f64>() == Ok(abs) {
        (digits, exponent) = split_scientific(&exact);
    }
    if digits.len() == 1 {
        // Java considers all one- and two-digit decimals when the shortest one has a single
        // digit, and prints the one closest to the exact value. This only makes a difference
        // for subnormals, e.g. `Double.MIN_VALUE` prints as `4.9E-324` rather than `5.0E-324`.
        let two = format!("{abs:.1e}");
        let (two_digits, two_exponent) = split_scientific(&two);
        if two.parse::<f64>() == Ok(abs) && !two_digits.ends_with('0') {
            return (two_digits, two_exponent);
        }
    }
    (digits, exponent)
}

/// Java's `Math.round(double)`: rounds half up and saturates to the range of `long`.
pub fn round(a: f64) -> i64 {
    const SIGNIFICAND_WIDTH: i64 = 53;
    const EXP_BIAS: i64 = 1023;
    const EXP_BIT_MASK: i64 = 0x7FF0_0000_0000_0000;
    const SIGNIF_BIT_MASK: i64 = 0x000F_FFFF_FFFF_FFFF;

    let long_bits = a.to_bits() as i64;
    let biased_exp = (long_bits & EXP_BIT_MASK) >> (SIGNIFICAND_WIDTH - 1);
    let shift = (SIGNIFICAND_WIDTH - 2 + EXP_BIAS) - biased_exp;
    if shift & -64 == 0 {
        let mut r = (long_bits & SIGNIF_BIT_MASK) | (SIGNIF_BIT_MASK + 1);
        if long_bits < 0 {
            r = -r;
        }
        ((r >> shift) + 1) >> 1
    } else {
        // Infinity saturates, NaN becomes zero, and large values are already integers.
        a as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_java() {
        let cases = [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (10.0, "10.0"),
            (0.5, "0.5"),
            (0.001, "0.001"),
            (0.0001, "1.0E-4"),
            (1e7, "1.0E7"),
            (9_999_999.0, "9999999.0"),
            (123.456, "123.456"),
            (1.0 / 3.0, "0.3333333333333333"),
            (1e21, "1.0E21"),
            (1.5e-10, "1.5E-10"),
            (f64::MIN_POSITIVE * 0.5, "1.1125369292536007E-308"),
            (5e-324, "4.9E-324"),
            (f64::MAX, "1.7976931348623157E308"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
            (-2.5, "-2.5"),
        ];
        for (value, expected) in cases {
            assert_eq!(double_to_string(value), expected, "formatting {value:e}");
        }
    }

    #[test]
    fn rounds_like_java() {
        assert_eq!(round(0.5), 1);
        assert_eq!(round(-0.5), 0);
        assert_eq!(round(-0.500_000_1), -1);
        assert_eq!(round(0.499_999_999_999_999_94), 0);
        assert_eq!(round(2.5), 3);
        assert_eq!(round(f64::NAN), 0);
        assert_eq!(round(f64::INFINITY), i64::MAX);
        assert_eq!(round(-1e300), i64::MIN);
        assert_eq!(round(4_503_599_627_370_497.0), 4_503_599_627_370_497);
    }
}
