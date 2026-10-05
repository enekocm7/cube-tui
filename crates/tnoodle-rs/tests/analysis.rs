//! End-to-end statistical tests of scramble quality (ports of `CubeTestTest` and the
//! `scrambleanalysis` `App`). They generate thousands of scrambles, so they are ignored by
//! default: `cargo test --release -- --ignored`.

use std::time::Duration;

use tnoodle::analysis::{convert_to_cube_states, generate_wca_scrambles, test_scrambles};
use tnoodle::java::Sha1Prng;
use tnoodle::puzzle::CubePuzzle;

#[test]
#[ignore = "generates 20000 scrambles"]
fn random_move_scrambles_are_detected_as_non_random() {
    // There is a very slim chance that random-move scrambles look as good as random-state
    // ones; if this fails, think about the quality of the random state solver before
    // dismissing it as a false positive.
    let scrambles =
        generate_wca_scrambles(&CubePuzzle::new(3), 20_000, &mut Sha1Prng::from_entropy());
    let report = test_scrambles(&convert_to_cube_states(&scrambles).unwrap()).unwrap();
    println!("{report}");
    assert!(!report.passed());
}

#[test]
#[ignore = "generates 6500 random-state scrambles"]
fn random_state_scrambles_look_random() {
    let puzzle = CubePuzzle::three_by_three().with_min_search_time(Duration::ZERO);
    let scrambles = generate_wca_scrambles(&puzzle, 6500, &mut Sha1Prng::from_entropy());
    let report = test_scrambles(&convert_to_cube_states(&scrambles).unwrap()).unwrap();
    println!("{report}");
    assert!(report.passed(), "{report}");
}
