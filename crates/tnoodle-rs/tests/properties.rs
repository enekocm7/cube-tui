//! Property-based tests: invariants that must hold for any input, complementing the
//! fixed differential fixtures.

use std::collections::HashMap;
use std::time::Duration;

use proptest::prelude::*;
use tnoodle::fto3phase::{self, FtoCubie};
use tnoodle::java::{JavaHashMap, JavaRandom, RandomSource, double_to_string, string_hash};
use tnoodle::min2phase::{INVERSE_SOLUTION, Search, SearchWca, tools};
use tnoodle::puzzle::{
    ClockPuzzle, CubePuzzle, FtoPuzzle, MegaminxPuzzle, PyraminxPuzzle, SkewbPuzzle,
    SquareOnePuzzle,
};
use tnoodle::scrambles::{AlgorithmBuilder, MergingMode, Puzzle, PuzzleState, split_algorithm};
use tnoodle::sq12phase::{self, FullCube};
use tnoodle::svg::Color;

fn config(cases: u32) -> ProptestConfig {
    ProptestConfig {
        cases,
        ..ProptestConfig::default()
    }
}

/// Java's `String.split("\\s+")` (after an emptiness check on `trim()`), written with a
/// different algorithm than the library's.
fn reference_split(s: &str) -> Vec<String> {
    if s.chars().all(|c| c <= ' ') {
        return Vec::new();
    }
    let ws = |c: char| matches!(c, ' ' | '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r');
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if ws(c) {
            while chars.peek().copied().is_some_and(ws) {
                chars.next();
            }
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    parts.push(current);
    while parts.last().is_some_and(String::is_empty) {
        parts.pop();
    }
    parts
}

proptest! {
    #![proptest_config(config(2000))]

    #[test]
    fn double_to_string_round_trips(bits in any::<u64>()) {
        let x = f64::from_bits(bits);
        prop_assume!(x.is_finite());
        let s = double_to_string(x);
        let parsed: f64 = s.parse().unwrap();
        prop_assert_eq!(parsed.to_bits(), x.to_bits(), "{}", s);
        let abs = x.abs();
        if abs != 0.0 && !(1e-3..1e7).contains(&abs) {
            prop_assert!(s.contains('E'), "{} should be scientific", s);
        } else {
            prop_assert!(s.contains('.') && !s.contains('E'), "{} should be plain", s);
        }
    }

    #[test]
    fn split_algorithm_matches_java_semantics(s in "[ \t\nRUF'2a-c\u{1}]{0,12}") {
        let expected = reference_split(&s);
        let actual: Vec<String> = split_algorithm(&s).into_iter().map(str::to_owned).collect();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn hex_colors_round_trip(r: u8, g: u8, b: u8) {
        let hex = format!("{r:02x}{g:02x}{b:02x}");
        let c = Color::from_hex(&hex).unwrap();
        prop_assert_eq!((c.red(), c.green(), c.blue()), (r, g, b));
        prop_assert_eq!(c.to_hex(), hex.clone());
        prop_assert_eq!(Color::from_hex(&format!("#{hex}")).unwrap(), c);
    }

    #[test]
    fn java_random_stays_in_bounds(seed: i64, bound in 1..i32::MAX) {
        let mut r = JavaRandom::new(seed);
        for _ in 0..8 {
            let v = r.next_int_bounded(bound);
            prop_assert!((0..bound).contains(&v));
            let d = r.next_double();
            prop_assert!((0.0..1.0).contains(&d));
        }
    }

    #[test]
    fn java_hash_map_behaves_like_a_map(ops in prop::collection::vec((any::<bool>(), "[A-Z'2]{1,3}"), 0..200)) {
        let mut model: HashMap<String, usize> = HashMap::new();
        let mut map = JavaHashMap::new();
        for (i, (insert, key)) in ops.iter().enumerate() {
            if *insert {
                prop_assert_eq!(map.insert(key.clone(), i), model.insert(key.clone(), i));
            } else {
                prop_assert_eq!(map.remove(key), model.remove(key));
            }
            prop_assert_eq!(map.len(), model.len());
        }
        let mut keys: Vec<&String> = map.keys().collect();
        prop_assert_eq!(keys.len(), model.len());
        keys.sort();
        keys.dedup();
        prop_assert_eq!(keys.len(), model.len());
        for (k, v) in map.iter() {
            prop_assert_eq!(model.get(k), Some(v));
        }
        let copy = JavaHashMap::copy_of(&map);
        prop_assert_eq!(copy.len(), map.len());
        for (k, v) in copy.iter() {
            prop_assert_eq!(map.get(k), Some(v));
        }
    }

    #[test]
    fn string_hash_is_the_java_polynomial(s in "\\PC{0,20}") {
        let expected = s
            .encode_utf16()
            .fold(0_i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)));
        prop_assert_eq!(string_hash(&s), expected);
    }
}

/// Checks the invariants of WCA scrambles and solving for one puzzle and seed.
fn check_scramble<P: Puzzle>(p: &P, seed: i64) -> Result<(), TestCaseError> {
    check_scramble_with(p, seed, true)
}

/// Like [`check_scramble`]; `check_merging` is off for Clock, whose `y2` is a rotation
/// that TNoodle's `AlgorithmBuilder` refuses to canonicalize.
fn check_scramble_with<P: Puzzle>(
    p: &P,
    seed: i64,
    check_merging: bool,
) -> Result<(), TestCaseError> {
    let scramble = p.generate_wca_scramble(&mut JavaRandom::new(seed));
    let state = p.solved_state().apply_algorithm(&scramble).unwrap();
    // Every WCA scramble is at least the minimum distance away from solved.
    prop_assert_eq!(
        state.solve_in(p.wca_min_scramble_distance() - 1),
        None,
        "{}",
        scramble
    );
    if check_merging {
        // Merging moves never changes the state (up to rotation).
        let mut ab = AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, p.solved_state());
        ab.append_algorithm(&scramble).unwrap();
        prop_assert!(ab.state().equals_normalized(&state));
    }
    // Drawing works for every scramble.
    let svg = p.draw_scramble(Some(&scramble), None).unwrap().to_string();
    prop_assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
    Ok(())
}

/// A random walk of `len` moves, then a check that `solve_in(len)` solves it.
fn check_walk_is_solvable<P: Puzzle>(p: &P, seed: i64, len: usize) -> Result<(), TestCaseError> {
    let mut r = JavaRandom::new(seed);
    let mut state = p.solved_state();
    for _ in 0..len {
        let succ = state.successors_by_name();
        let i = r.next_int_bounded(succ.len() as i32) as usize;
        state = succ.into_iter().nth(i).unwrap().1;
    }
    let solution = state.solve_in(len as i32);
    prop_assert!(solution.is_some(), "no solution of {} moves", len);
    let solved = state.apply_algorithm(&solution.unwrap()).unwrap();
    prop_assert!(solved.is_solved());
    // Normalization is idempotent and preserves the solved status.
    let normalized = state.normalized().into_owned();
    prop_assert_eq!(&*normalized.normalized(), &normalized);
    prop_assert!(normalized.equals_normalized(&state));
    Ok(())
}

proptest! {
    #![proptest_config(config(12))]

    #[test]
    fn fast_puzzles_generate_valid_wca_scrambles(seed: i64) {
        check_scramble(&CubePuzzle::two_by_two(), seed)?;
        check_scramble(&CubePuzzle::three_by_three().with_min_search_time(Duration::ZERO), seed)?;
        check_scramble(&CubePuzzle::three_by_three_fewest_moves().with_min_search_time(Duration::ZERO), seed)?;
        check_scramble(&CubePuzzle::new(5), seed)?;
        check_scramble(&PyraminxPuzzle::new(), seed)?;
        check_scramble(&SkewbPuzzle::new(), seed)?;
        check_scramble(&SquareOnePuzzle::new(), seed)?;
        check_scramble(&MegaminxPuzzle::new(), seed)?;
        check_scramble_with(&ClockPuzzle::new(), seed, false)?;
        check_scramble(&FtoPuzzle::new(), seed)?;
    }

    #[test]
    fn random_walks_are_solved_within_their_length(seed: i64) {
        check_walk_is_solvable(&CubePuzzle::two_by_two(), seed, 4)?;
        check_walk_is_solvable(&CubePuzzle::three_by_three(), seed, 4)?;
        check_walk_is_solvable(&CubePuzzle::new(4), seed, 2)?;
        check_walk_is_solvable(&PyraminxPuzzle::new(), seed, 4)?;
        check_walk_is_solvable(&SkewbPuzzle::new(), seed, 4)?;
        check_walk_is_solvable(&SquareOnePuzzle::new(), seed, 3)?;
        check_walk_is_solvable(&MegaminxPuzzle::new(), seed, 2)?;
        check_walk_is_solvable(&ClockPuzzle::new(), seed, 2)?;
        check_walk_is_solvable(&FtoPuzzle::new(), seed, 2)?;
    }

    #[test]
    fn min2phase_solutions_generate_the_cube(seed: i64) {
        let facelets = tools::random_cube(&mut JavaRandom::new(seed));
        let scramble = Search::new().solution(&facelets, 21, 100_000, 0, INVERSE_SOLUTION);
        prop_assert_eq!(tools::from_scramble(&scramble), facelets.clone());
        let wca = SearchWca::new().solution(&facelets, 21, 60_000, 0, INVERSE_SOLUTION, Some("R"), Some("U"));
        prop_assert_eq!(tools::from_scramble(&wca), facelets);
        // The restrictions apply to the solution, so they constrain the opposite ends of
        // the inverted scramble.
        let moves = split_algorithm(wca.trim());
        prop_assert!(!moves[0].starts_with('U') && !moves[0].starts_with('D'));
        let last = moves[moves.len() - 1];
        prop_assert!(!last.starts_with('R') && !last.starts_with('L'));
    }

    #[test]
    fn sq12phase_solutions_generate_the_cube(seed: i64) {
        let cube = FullCube::random(&mut JavaRandom::new(seed));
        let scramble = sq12phase::Search::new().solution(&cube, sq12phase::INVERSE_SOLUTION).unwrap();
        let state = SquareOnePuzzle::new().solved_state().apply_algorithm(scramble.trim()).unwrap();
        prop_assert_eq!(state.to_full_cube().raw(), cube.raw());
    }

    #[test]
    fn fto_solutions_generate_the_cubie(seed: i64) {
        let cubie = FtoCubie::random(&mut JavaRandom::new(seed));
        let scramble = fto3phase::Search::new().solution(&cubie);
        prop_assert_eq!(fto3phase::from_alg(&scramble), cubie);
    }
}
