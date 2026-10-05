//! The hypothesis tests used by the scramble analysis, replacing Apache Commons Math's
//! `ChiSquareTest` and `BinomialTest` (and the expected distributions of
//! `scrambleanalysis.statistics.Distribution`).

use std::f64::consts::PI;

const EPSILON: f64 = 1e-14;
/// Replaces zero denominators in the continued fraction.
const SMALL: f64 = 1e-50;
const MAX_ITERATIONS: usize = 100_000;

/// `ln Γ(x)` for `x > 0` (Lanczos approximation, g = 7, n = 9).
pub fn ln_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection formula.
        return (PI / (PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = COEFFICIENTS[0];
    let t = x + G + 0.5;
    for (i, &c) in COEFFICIENTS.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// The regularized lower incomplete gamma function `P(a, x)`.
pub fn regularized_gamma_p(a: f64, x: f64) -> f64 {
    if a.is_nan() || x.is_nan() || a <= 0.0 || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x >= a + 1.0 {
        return 1.0 - regularized_gamma_q(a, x);
    }
    // Series expansion.
    let mut n = 0.0;
    let mut an = 1.0 / a;
    let mut sum = an;
    let mut iterations = 0;
    while (an / sum).abs() > EPSILON && iterations < MAX_ITERATIONS && sum.is_finite() {
        n += 1.0;
        an *= x / (a + n);
        sum += an;
        iterations += 1;
    }
    if sum.is_infinite() {
        return 1.0;
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() * sum
}

/// The regularized upper incomplete gamma function `Q(a, x) = 1 - P(a, x)`.
pub fn regularized_gamma_q(a: f64, x: f64) -> f64 {
    if a.is_nan() || x.is_nan() || a <= 0.0 || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return 1.0;
    }
    if x < a + 1.0 {
        return 1.0 - regularized_gamma_p(a, x);
    }
    // Continued fraction (modified Lentz).
    let get_a = |n: f64| (2.0 * n + 1.0) - a + x;
    let get_b = |n: f64| n * (a - n);
    let mut h_prev = get_a(0.0);
    if h_prev.abs() < SMALL {
        h_prev = SMALL;
    }
    let mut d_prev = 0.0;
    let mut c_prev = h_prev;
    let mut h = h_prev;
    for n in 1..MAX_ITERATIONS {
        let n = n as f64;
        let (an, bn) = (get_a(n), get_b(n));
        let mut d = an + bn * d_prev;
        if d.abs() < SMALL {
            d = SMALL;
        }
        let mut c = an + bn / c_prev;
        if c.abs() < SMALL {
            c = SMALL;
        }
        d = 1.0 / d;
        let delta = c * d;
        h = h_prev * delta;
        if (delta - 1.0).abs() < EPSILON {
            break;
        }
        d_prev = d;
        c_prev = c;
        h_prev = h;
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() / h
}

/// `P(X <= x)` for a chi-squared distribution with `degrees_of_freedom`.
pub fn chi_squared_cdf(x: f64, degrees_of_freedom: f64) -> f64 {
    if x <= 0.0 {
        0.0
    } else {
        regularized_gamma_p(degrees_of_freedom / 2.0, x / 2.0)
    }
}

/// Pearson's chi-squared statistic of `observed` counts against `expected` frequencies,
/// which are rescaled when they do not sum to the same total.
pub fn chi_square(expected: &[f64], observed: &[u64]) -> f64 {
    assert_eq!(expected.len(), observed.len(), "dimension mismatch");
    assert!(expected.len() >= 2, "at least two categories are needed");
    let sum_expected: f64 = expected.iter().sum();
    let sum_observed: f64 = observed.iter().map(|&o| o as f64).sum();
    let ratio = if (sum_expected - sum_observed).abs() > 10e-6 {
        sum_observed / sum_expected
    } else {
        1.0
    };
    expected
        .iter()
        .zip(observed)
        .map(|(&e, &o)| {
            let e = ratio * e;
            let dev = o as f64 - e;
            dev * dev / e
        })
        .sum()
}

/// The p-value of the goodness of fit test of `observed` against `expected`.
pub fn chi_square_test(expected: &[f64], observed: &[u64]) -> f64 {
    1.0 - chi_squared_cdf(chi_square(expected, observed), (expected.len() - 1) as f64)
}

/// Whether the goodness of fit test rejects the null hypothesis at significance `alpha`
/// (i.e. `observed` does not follow `expected`).
pub fn chi_square_rejects(expected: &[f64], observed: &[u64], alpha: f64) -> bool {
    chi_square_test(expected, observed) < alpha
}

/// The chi-squared statistic comparing two sets of counts.
///
/// # Panics
///
/// Panics if the sets have different lengths, fewer than two categories, or a category is
/// empty in both.
pub fn chi_square_data_sets_comparison(observed1: &[u64], observed2: &[u64]) -> f64 {
    assert_eq!(observed1.len(), observed2.len(), "dimension mismatch");
    assert!(observed1.len() >= 2, "at least two categories are needed");
    let sum1: u64 = observed1.iter().sum();
    let sum2: u64 = observed2.iter().sum();
    let weight = (sum1 as f64 / sum2 as f64).sqrt();
    observed1
        .iter()
        .zip(observed2)
        .map(|(&o1, &o2)| {
            assert!(
                o1 != 0 || o2 != 0,
                "a category has no observations in either data set"
            );
            let (o1, o2) = (o1 as f64, o2 as f64);
            let dev = if sum1 == sum2 {
                o1 - o2
            } else {
                o1 / weight - o2 * weight
            };
            dev * dev / (o1 + o2)
        })
        .sum()
}

/// Whether the two sets of counts come from different distributions at significance
/// `alpha`.
pub fn chi_square_data_sets_differ(observed1: &[u64], observed2: &[u64], alpha: f64) -> bool {
    let statistic = chi_square_data_sets_comparison(observed1, observed2);
    1.0 - chi_squared_cdf(statistic, (observed1.len() - 1) as f64) < alpha
}

fn binomial_pmf(trials: u64, k: u64, p: f64) -> f64 {
    if k > trials {
        return 0.0;
    }
    let (n, k_f) = (trials as f64, k as f64);
    let ln_choose = ln_gamma(n + 1.0) - ln_gamma(k_f + 1.0) - ln_gamma(n - k_f + 1.0);
    let ln_p = if k == 0 { 0.0 } else { k_f * p.ln() };
    let ln_q = if k == trials {
        0.0
    } else {
        (n - k_f) * (1.0 - p).ln()
    };
    (ln_choose + ln_p + ln_q).exp()
}

/// The two sided p-value of observing `successes` in `trials` with success probability `p`
/// (summing the probabilities of all outcomes at most as likely, like Commons Math).
pub fn binomial_test_two_sided(trials: u64, successes: u64, p: f64) -> f64 {
    let mut low = 0_u64;
    let mut high = trials as i64;
    let mut total = 0.0;
    loop {
        let p_low = binomial_pmf(trials, low, p);
        let p_high = binomial_pmf(trials, high as u64, p);
        let close = (p_low - p_high).abs() <= 1e-12 * p_low.max(p_high);
        if close {
            total += 2.0 * p_low;
            low += 1;
            high -= 1;
        } else if p_low < p_high {
            total += p_low;
            low += 1;
        } else {
            total += p_high;
            high -= 1;
        }
        if low > successes || high < successes as i64 {
            break;
        }
    }
    total.min(1.0)
}

/// `n choose k`, exactly.
pub fn n_choose_k(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    (0..k).fold(1, |acc, i| acc * (n - i) / (i + 1))
}

const EDGES: u64 = 12;
const CORNERS: u64 = 8;

/// The probabilities of 0, 2, 4, ..., 12 misoriented edges (index = misoriented / 2) in a
/// random 3x3x3 state.
pub fn expected_edges_orientation_probability() -> [f64; 7] {
    let counts: [u64; 7] = std::array::from_fn(|i| n_choose_k(12, 2 * i as u64));
    let total: u64 = counts.iter().sum();
    counts.map(|c| c as f64 / total as f64)
}

/// `N / 12` for each of the 12 edge positions.
pub fn expected_edges_final_position(n: u64) -> [u64; EDGES as usize] {
    [n / EDGES; EDGES as usize]
}

/// The probabilities of corner orientation sums 0, 3, 6, ..., 15 (index = sum / 3) in a
/// random 3x3x3 state, counting 1 per clockwise and 2 per counter-clockwise twist.
pub fn expected_corners_orientation_probability() -> [f64; 6] {
    let mut counts = [0_u64; 6];
    for (i, c) in counts.iter_mut().enumerate() {
        let sum = 3 * i as u64;
        for j in 0..CORNERS {
            for k in 0..CORNERS {
                if j + k <= 8 && j * 2 + k == sum {
                    *c += n_choose_k(8, j) * n_choose_k(8 - j, k);
                }
            }
        }
    }
    let total: u64 = counts.iter().sum();
    counts.map(|c| c as f64 / total as f64)
}

/// `N / 8` for each of the 8 corner positions.
pub fn expected_corners_final_position(n: u64) -> [u64; CORNERS as usize] {
    [n / CORNERS; CORNERS as usize]
}

/// The minimum sample size for which every expected count is large enough (6144).
pub fn minimum_sample_size() -> u64 {
    let edges = expected_edges_orientation_probability()
        .iter()
        .map(|&p| crate::java::round(3.0 / p) as u64)
        .max()
        .unwrap_or(0);
    let corners = expected_corners_orientation_probability()
        .iter()
        .map(|&p| crate::java::round(1.0 / p) as u64)
        .max()
        .unwrap_or(0);
    edges.max(corners)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn gamma_functions() {
        assert!(close(ln_gamma(1.0), 0.0, 1e-13));
        assert!(close(ln_gamma(5.0), 24_f64.ln(), 1e-12));
        assert!(close(ln_gamma(0.5), PI.sqrt().ln(), 1e-12));
        assert!(close(
            regularized_gamma_p(1.0, 1.0),
            1.0 - (-1.0_f64).exp(),
            1e-13
        ));
        assert!(close(
            regularized_gamma_p(3.0, 10.0) + regularized_gamma_q(3.0, 10.0),
            1.0,
            1e-13
        ));
        assert!(regularized_gamma_p(-1.0, 1.0).is_nan());
        assert_eq!(regularized_gamma_q(2.0, 0.0), 1.0);
    }

    #[test]
    fn chi_squared_distribution() {
        // Critical values: P(X <= 3.841) = 0.95 with 1 degree of freedom, and
        // P(X <= 11.070) = 0.95 with 5 degrees of freedom.
        assert!(close(
            chi_squared_cdf(3.841_458_820_694_124, 1.0),
            0.95,
            1e-9
        ));
        assert!(close(
            chi_squared_cdf(11.070_497_693_516_35, 5.0),
            0.95,
            1e-9
        ));
        assert_eq!(chi_squared_cdf(-1.0, 3.0), 0.0);
    }

    #[test]
    fn chi_square_tests() {
        let expected = [0.25, 0.25, 0.25, 0.25];
        assert!(!chi_square_rejects(&expected, &[250, 248, 252, 250], 0.01));
        assert!(chi_square_rejects(&expected, &[400, 200, 200, 200], 0.01));
        // (12 - 10)² / 10 + (8 - 10)² / 10
        assert!(close(chi_square(&[10.0, 10.0], &[12, 8]), 0.8, 1e-12));
        // Expected frequencies are rescaled to the observed total.
        assert!(close(chi_square(&[1.0, 1.0], &[12, 8]), 0.8, 1e-12));
        assert!(!chi_square_data_sets_differ(
            &[100, 100, 100],
            &[98, 102, 100],
            0.01
        ));
        assert!(chi_square_data_sets_differ(
            &[300, 0, 1],
            &[1, 300, 1],
            0.01
        ));
        assert!(close(
            chi_square_data_sets_comparison(&[10, 20], &[20, 40]),
            0.0,
            1e-12
        ));
    }

    #[test]
    fn binomial_tests() {
        assert!(close(binomial_test_two_sided(10, 5, 0.5), 1.0, 1e-12));
        // P(X <= 1 or X >= 9) for Bin(10, 1/2) = 22 / 1024.
        assert!(close(
            binomial_test_two_sided(10, 1, 0.5),
            22.0 / 1024.0,
            1e-12
        ));
        assert!(binomial_test_two_sided(1000, 400, 0.5) < 0.01);
    }

    #[test]
    fn distributions() {
        assert_eq!(n_choose_k(8, 2), 28);
        assert_eq!(n_choose_k(2, 3), 0);
        assert!(close(
            expected_edges_orientation_probability().iter().sum(),
            1.0,
            1e-12
        ));
        assert!(close(
            expected_corners_orientation_probability().iter().sum(),
            1.0,
            1e-12
        ));
        assert_eq!(expected_edges_final_position(24), [2; 12]);
        assert_eq!(expected_corners_final_position(16), [2; 8]);
        assert_eq!(minimum_sample_size(), 6144);
    }
}
