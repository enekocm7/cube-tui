//! Differential tests of the threephase 4x4x4 solver against the Java implementation.

mod common;

use common::{as_array, as_i32, as_str, fixture};
use tnoodle::java::JavaRandom;
use tnoodle::threephase::{self, Search};

#[test]
fn random_states_and_solutions_match_java() {
    let f = fixture("threephase");
    // One searcher for the whole sequence, exactly like the Java recording: a searcher's first solve
    // starts from freshly zeroed scratch state, which influences the result.
    let mut s = Search::new();
    for case in as_array(&f["random"]) {
        let seed = i64::from(as_i32(&case["seed"]));
        assert_eq!(
            s.random_state(&mut JavaRandom::new(seed)),
            as_str(&case["randomState"]),
            "random state, seed {seed}"
        );
        let facelet = threephase::random_cube(&mut JavaRandom::new(seed + 100));
        assert_eq!(facelet, as_str(&case["facelet"]), "facelets, seed {seed}");
        assert_eq!(
            s.solution(&facelet),
            as_str(&case["solution"]),
            "solution, seed {seed}"
        );
    }
}

#[test]
fn rotation_and_inverse_options_match_java() {
    let f = fixture("threephase");
    let mut s = Search::new();
    s.inverse_solution = false;
    s.with_rotation = true;
    assert_eq!(
        s.random_state(&mut JavaRandom::new(77)),
        as_str(&f["noInverseWithRotation"])
    );
    let mut s = Search::new();
    s.with_rotation = true;
    assert_eq!(
        s.random_state(&mut JavaRandom::new(78)),
        as_str(&f["inverseWithRotation"])
    );
    assert_eq!(
        Search::new().solve("R U Rw' F2 Uw2 Bw D' l r2 B"),
        as_str(&f["solveScramble"])
    );
}
