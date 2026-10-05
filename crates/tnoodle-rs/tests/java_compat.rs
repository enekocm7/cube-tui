//! Differential tests of the Java platform emulation against values recorded from a JVM.

mod common;

use common::{as_array, as_i32, as_i64, as_str, fixture};
use serde_json::Value;
use tnoodle::java::{self, JavaHashMap, JavaRandom, RandomSource, Sha1Prng};

/// Replays the operation log recorded from Java.
fn replay(r: &mut dyn RandomSource, ops: &[Value], context: &str) {
    for (i, op) in ops.iter().enumerate() {
        let op = as_array(op);
        let code = as_i64(&op[0]);
        let arg = as_i64(&op[1]);
        let ctx = || format!("{context}, op #{i} {op:?}");
        match code {
            0 => assert_eq!(i64::from(r.next_int()), as_i64(&op[2]), "{}", ctx()),
            1 => assert_eq!(
                i64::from(r.next_int_bounded(arg as i32)),
                as_i64(&op[2]),
                "{}",
                ctx()
            ),
            2 => assert_eq!(r.next_long(), as_i64(&op[2]), "{}", ctx()),
            3 => assert_eq!(i64::from(r.next_boolean()), as_i64(&op[2]), "{}", ctx()),
            4 => assert_eq!(
                r.next_double().to_bits() as i64,
                as_i64(&op[2]),
                "{}",
                ctx()
            ),
            5 => assert_eq!(
                i64::from(r.next_float().to_bits()),
                as_i64(&op[2]),
                "{}",
                ctx()
            ),
            6 => {
                let mut bytes = vec![0_u8; arg as usize];
                r.next_bytes(&mut bytes);
                let expected: Vec<u8> = as_array(&op[2]).iter().map(|b| as_i64(b) as u8).collect();
                assert_eq!(bytes, expected, "{}", ctx());
            }
            _ => panic!("unknown op {code}"),
        }
    }
}

#[test]
fn java_util_random_matches_the_jdk() {
    for case in as_array(&fixture("java_random")) {
        let seed = as_i64(&case["seed"]);
        replay(
            &mut JavaRandom::new(seed),
            as_array(&case["ops"]),
            &format!("seed {seed}"),
        );
    }
}

#[test]
fn sha1prng_matches_the_jdk() {
    for case in as_array(&fixture("sha1prng")) {
        let seed = as_str(&case["seed"]);
        replay(
            &mut Sha1Prng::with_seed(seed.as_bytes()),
            as_array(&case["ops"]),
            &format!("seed {seed:?}"),
        );
    }
}

#[test]
fn string_hash_codes_match_the_jdk() {
    for case in as_array(&fixture("java_strings")) {
        let case = as_array(case);
        assert_eq!(
            java::string_hash(as_str(&case[0])),
            as_i32(&case[1]),
            "{:?}",
            case[0]
        );
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Key {
    Str(String),
    Int(i32),
}

impl java::JavaHash for Key {
    fn java_hash(&self) -> i32 {
        match self {
            Self::Str(s) => java::string_hash(s),
            Self::Int(i) => *i,
        }
    }

    fn java_compare(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Str(a), Self::Str(b)) => Some(java::string_compare(a, b)),
            (Self::Int(a), Self::Int(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
}

fn key(v: &Value) -> Key {
    v.as_str()
        .map_or_else(|| Key::Int(as_i32(v)), |s| Key::Str(s.to_owned()))
}

#[test]
fn hash_map_iteration_order_matches_the_jdk() {
    for (n, case) in as_array(&fixture("java_hashmap")).iter().enumerate() {
        let mut map = JavaHashMap::new();
        for (i, op) in as_array(&case["ops"]).iter().enumerate() {
            let op = as_array(op);
            match as_str(&op[0]) {
                "put" => {
                    map.insert(key(&op[1]), i);
                }
                "remove" => {
                    map.remove(&key(&op[1]));
                }
                other => panic!("unknown op {other}"),
            }
        }
        let expected: Vec<Key> = as_array(&case["order"]).iter().map(key).collect();
        let actual: Vec<Key> = map.keys().cloned().collect();
        assert_eq!(actual, expected, "case {n}");
        let expected_copy: Vec<Key> = as_array(&case["copyOrder"]).iter().map(key).collect();
        let copy = JavaHashMap::copy_of(&map);
        assert_eq!(
            copy.keys().cloned().collect::<Vec<_>>(),
            expected_copy,
            "copy of case {n}"
        );
        assert_eq!(copy.len(), map.len());
    }
}

#[test]
fn double_to_string_matches_the_jdk() {
    let mut failures = Vec::new();
    for case in as_array(&fixture("java_double")) {
        let case = as_array(case);
        let value = f64::from_bits(as_i64(&case[0]) as u64);
        let actual = java::double_to_string(value);
        if actual != as_str(&case[1]) {
            failures.push(format!(
                "{value:e}: expected {} got {actual}",
                as_str(&case[1])
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// `Math.sin`/`Math.cos` on HotSpot x86-64 are correctly rounded except for a handful of
/// inputs, while other JVMs use fdlibm, so "Java's" trigonometry is platform dependent. The
/// port is correctly rounded. Checked against 300-bit mpmath evaluations when this fixture
/// was recorded, exactly 27 of the recorded sin/cos values are *not* correctly rounded in
/// Java (they are off by one ulp); every other value must match bit for bit.
const JAVA_SIN_COS_NOT_CORRECTLY_ROUNDED: usize = 27;

fn ulp_distance(a: f64, b: f64) -> u64 {
    (a.to_bits() as i64).abs_diff(b.to_bits() as i64)
}

#[test]
fn math_functions_match_the_jdk() {
    let mut trig_mismatches = Vec::new();
    for case in as_array(&fixture("java_math")) {
        let c: Vec<i64> = as_array(case).iter().map(as_i64).collect();
        let x = f64::from_bits(c[0] as u64);
        for (name, actual, expected) in [("sin", java::sin(x), c[1]), ("cos", java::cos(x), c[2])] {
            let expected = f64::from_bits(expected as u64);
            if actual.to_bits() != expected.to_bits() {
                assert_eq!(
                    ulp_distance(actual, expected),
                    1,
                    "{name}({x:e}) = {actual:e}, Java {expected:e}"
                );
                trig_mismatches.push(format!("{name}({x:e})"));
            }
        }
        assert_eq!(x.abs().sqrt().to_bits() as i64, c[3], "sqrt({x})");
        assert_eq!(
            java::acos((x / 20.0).clamp(-1.0, 1.0)).to_bits() as i64,
            c[4],
            "acos({x} / 20)"
        );
        assert_eq!(
            (x * 30.0).to_radians().to_bits() as i64,
            c[5],
            "toRadians({x} * 30)"
        );
        assert_eq!(java::round(x * 10.0), c[6], "round({x} * 10)");
    }
    assert!(
        trig_mismatches.len() <= JAVA_SIN_COS_NOT_CORRECTLY_ROUNDED,
        "{} sin/cos mismatches: {trig_mismatches:?}",
        trig_mismatches.len()
    );
}
