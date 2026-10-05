//! Differential tests of the min2phase 3x3x3 solver against the Java implementation.

mod common;

use common::{as_array, as_i32, as_str, fixture, opt_str};
use tnoodle::java::JavaRandom;
use tnoodle::min2phase::{
    APPEND_LENGTH, INVERSE_SOLUTION, OPTIMAL_SOLUTION, Search, SearchWca, USE_SEPARATOR, tools,
};

#[test]
fn random_cubes_and_solutions_match_java() {
    let f = fixture("min2phase");
    for case in as_array(&f["random"]) {
        let seed = i64::from(as_i32(&case["seed"]));
        let facelet = tools::random_cube(&mut JavaRandom::new(seed));
        assert_eq!(facelet, as_str(&case["facelet"]), "seed {seed}");

        let ctx = |k: &str| format!("seed {seed}, {k}");
        assert_eq!(
            Search::new().solution(&facelet, 21, 100_000, 0, 0),
            as_str(&case["plain"]),
            "{}",
            ctx("plain")
        );
        assert_eq!(
            Search::new().solution(
                &facelet,
                21,
                100_000,
                0,
                INVERSE_SOLUTION | USE_SEPARATOR | APPEND_LENGTH
            ),
            as_str(&case["inverseSeparatorLength"]),
            "{}",
            ctx("inverse+separator+length")
        );
        assert_eq!(
            Search::new().solution(&facelet, 21, 100_000, 0, USE_SEPARATOR),
            as_str(&case["separator"]),
            "{}",
            ctx("separator")
        );
        assert_eq!(
            Search::new().solution(&facelet, 21, 100_000, 200, 0),
            as_str(&case["probeMin200"]),
            "{}",
            ctx("probeMin 200")
        );
        assert_eq!(
            Search::new().solution(&facelet, 20, 2000, 0, 0),
            as_str(&case["depth20"]),
            "{}",
            ctx("depth 20")
        );

        let mut s = Search::new();
        let next = as_array(&case["next"]);
        assert_eq!(s.solution(&facelet, 21, 100_000, 0, 0), as_str(&next[0]));
        assert_eq!(s.length(), as_i32(&next[1]), "{}", ctx("length"));
        assert_eq!(s.next(100_000, 0, 0), as_str(&next[2]), "{}", ctx("next"));
        assert_eq!(
            s.next(100_000, 50, APPEND_LENGTH),
            as_str(&next[3]),
            "{}",
            ctx("next 2")
        );
        assert_eq!(
            s.number_of_probes(),
            i64::from(as_i32(&next[4])),
            "{}",
            ctx("probes")
        );

        let first = opt_str(&case["first"]);
        let last = opt_str(&case["last"]);
        assert_eq!(
            SearchWca::new().solution(&facelet, 21, 60_000, 0, 0, first, last),
            as_str(&case["wca"]),
            "{}",
            ctx("wca")
        );
        assert_eq!(
            SearchWca::new().solution(
                &facelet,
                21,
                60_000,
                0,
                INVERSE_SOLUTION | APPEND_LENGTH | USE_SEPARATOR,
                first,
                last
            ),
            as_str(&case["wcaInverse"]),
            "{}",
            ctx("wca inverse")
        );
    }
}

#[test]
fn short_scrambles_and_optimal_solutions_match_java() {
    let f = fixture("min2phase");
    for case in as_array(&f["short"]) {
        let scramble = as_str(&case["scramble"]);
        let facelet = tools::from_scramble(scramble);
        assert_eq!(facelet, as_str(&case["facelet"]), "{scramble}");
        assert_eq!(tools::verify(&facelet), as_i32(&case["verify"]));
        let ctx = |k: &str| format!("scramble {scramble:?}, {k}");
        assert_eq!(
            Search::new().solution(&facelet, 21, 100_000, 0, 0),
            as_str(&case["plain"]),
            "{}",
            ctx("plain")
        );
        assert_eq!(
            Search::new().solution(&facelet, 21, 1_000_000, 0, OPTIMAL_SOLUTION),
            as_str(&case["optimal"]),
            "{}",
            ctx("optimal")
        );
        assert_eq!(
            Search::new().solution(
                &facelet,
                21,
                1_000_000,
                0,
                OPTIMAL_SOLUTION | INVERSE_SOLUTION
            ),
            as_str(&case["optimalInverse"]),
            "{}",
            ctx("optimal inverse")
        );
        let first = opt_str(&case["first"]);
        let last = opt_str(&case["last"]);
        assert_eq!(
            SearchWca::new().solution(&facelet, 21, 60_000, 0, 0, first, last),
            as_str(&case["wca"]),
            "{}",
            ctx("wca")
        );
        let len = scramble.split_whitespace().count() as i32;
        assert_eq!(
            SearchWca::new().solution(&facelet, (len - 1).max(0), 60_000, 0, 0, first, last),
            as_str(&case["wcaShort"]),
            "{}",
            ctx("wca short")
        );
    }
}

#[test]
fn error_codes_match_java() {
    let f = fixture("min2phase");
    for case in as_array(&f["errors"]) {
        let case = as_array(case);
        let facelet = as_str(&case[0]);
        assert_eq!(
            tools::verify(facelet),
            as_i32(&case[1]),
            "verify {facelet:?}"
        );
        assert_eq!(
            Search::new().solution(facelet, 21, 100_000, 0, 0),
            as_str(&case[2]),
            "solve {facelet:?}"
        );
    }
}

#[test]
fn tools_match_java() {
    let f = fixture("min2phase");
    let tools_fixture = &f["tools"];
    assert_eq!(tools::super_flip(), as_str(&tools_fixture["superFlip"]));
    for (seed, case) in as_array(&tools_fixture["partial"]).iter().enumerate() {
        let case = as_array(case);
        let mut r = JavaRandom::new(seed as i64);
        let generators: [fn(&mut JavaRandom) -> String; 9] = [
            |r| tools::random_cube(r),
            |r| tools::random_last_layer(r),
            |r| tools::random_last_slot(r),
            |r| tools::random_zb_last_layer(r),
            |r| tools::random_corner_of_last_layer(r),
            |r| tools::random_edge_of_last_layer(r),
            |r| tools::random_cross_solved(r),
            |r| tools::random_edge_solved(r),
            |r| tools::random_corner_solved(r),
        ];
        for (i, generate) in generators.iter().enumerate() {
            assert_eq!(
                generate(&mut r),
                as_str(&case[i]),
                "seed {seed}, generator {i}"
            );
        }
    }
    for case in as_array(&tools_fixture["fromScramble"]) {
        let case = as_array(case);
        assert_eq!(
            tools::from_scramble(as_str(&case[0])),
            as_str(&case[1]),
            "{:?}",
            case[0]
        );
    }
}
