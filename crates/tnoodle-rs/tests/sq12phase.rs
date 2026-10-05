//! Differential tests of the sq12phase Square-1 solver against the Java implementation.

mod common;

use common::{as_array, as_i32, fixture, ints, opt_str};
use tnoodle::java::JavaRandom;
use tnoodle::sq12phase::{FullCube, INVERSE_SOLUTION, Search};

#[test]
fn random_cubes_and_solutions_match_java() {
    let mut s = Search::new();
    for case in as_array(&fixture("sq12phase")) {
        let seed = i64::from(as_i32(&case["seed"]));
        let cube = FullCube::random(&mut JavaRandom::new(seed));
        assert_eq!(cube.raw().to_vec(), ints(&case["cube"]), "seed {seed}");
        assert_eq!(
            s.solution(&cube, INVERSE_SOLUTION).as_deref(),
            opt_str(&case["inverse"]),
            "inverse solution, seed {seed}"
        );
        assert_eq!(
            s.solution(&cube, 0).as_deref(),
            opt_str(&case["plain"]),
            "solution, seed {seed}"
        );
    }
}
