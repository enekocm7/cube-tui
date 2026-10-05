//! Differential tests of the 2x2x2, Pyraminx and Skewb solvers against the Java code.

mod common;

use common::{as_array, as_i32, fixture, ints, opt_str, strings};
use serde_json::Value;
use tnoodle::java::{JavaRandom, choose};
use tnoodle::puzzle::{
    CubePuzzle, PyraminxPuzzle, PyraminxSolver, SkewbPuzzle, SkewbSolver, TwoByTwoSolver,
};
use tnoodle::scrambles::{Puzzle, PuzzleState};

/// Replays a random walk recorded from Java (`Puzzle.choose` over the successors).
fn walk<S: PuzzleState>(start: S, r: &mut JavaRandom, expected: &Value) -> S {
    let mut state = start;
    for mv in strings(expected) {
        let succ = state.successors_by_name();
        let chosen = choose(r, succ.iter().map(|(m, _)| m.clone())).unwrap();
        assert_eq!(chosen, mv);
        state = succ.into_iter().find(|(m, _)| *m == chosen).unwrap().1;
    }
    state
}

#[test]
fn two_by_two_solver_matches_java() {
    let f = fixture("two_by_two_solver");
    let solver = TwoByTwoSolver::new();
    for case in as_array(&f["random"]) {
        let seed = i64::from(as_i32(&case["seed"]));
        let state = solver.random_state(&mut JavaRandom::new(seed));
        assert_eq!(
            state.permutation,
            as_i32(&case["permutation"]),
            "seed {seed}"
        );
        assert_eq!(
            state.orientation,
            as_i32(&case["orientation"]),
            "seed {seed}"
        );
        assert_eq!(
            solver.solve_in(state, 11).as_deref(),
            opt_str(&case["optimal"]),
            "optimal, seed {seed}"
        );
        assert_eq!(
            solver.solve_in(state, 5).as_deref(),
            opt_str(&case["solveIn5"]),
            "solveIn(5), seed {seed}"
        );
        assert_eq!(
            solver.generate_exactly(state, 11).as_deref(),
            opt_str(&case["exactly11"]),
            "exactly, seed {seed}"
        );
    }
    let twos = CubePuzzle::new(2);
    let mut r = JavaRandom::new(3);
    for case in as_array(&f["conversions"]) {
        let state = walk(twos.solved_state(), &mut r, &case["moves"]).to_two_by_two_state();
        assert_eq!(
            state.permutation,
            as_i32(&case["permutation"]),
            "{:?}",
            case["moves"]
        );
        assert_eq!(
            state.orientation,
            as_i32(&case["orientation"]),
            "{:?}",
            case["moves"]
        );
    }
}

#[test]
fn pyraminx_solver_matches_java() {
    let f = fixture("pyraminx_solver");
    let solver = PyraminxSolver::new();
    for case in as_array(&f["random"]) {
        let seed = i64::from(as_i32(&case["seed"]));
        let state = solver.random_state(&mut JavaRandom::new(seed));
        assert_eq!(
            vec![
                state.edge_perm,
                state.edge_orient,
                state.corner_orient,
                state.tips
            ],
            ints(&case["state"]),
            "seed {seed}"
        );
        assert_eq!(state.unsolved_tips(), as_i32(&case["unsolvedTips"]));
        let solve = |len, exact, inverse, tips, search_seed| {
            PyraminxSolver::solve(
                state,
                len,
                exact,
                inverse,
                tips,
                &mut JavaRandom::new(search_seed),
            )
        };
        assert_eq!(
            solve(11, false, false, true, seed + 500).as_deref(),
            opt_str(&case["solveIn"]),
            "seed {seed}"
        );
        assert_eq!(
            solve(9, false, false, false, seed + 600).as_deref(),
            opt_str(&case["solveInNoTips"]),
            "seed {seed}"
        );
        assert_eq!(
            solve(11, true, true, false, seed + 700).as_deref(),
            opt_str(&case["exactly"]),
            "seed {seed}"
        );
        assert_eq!(
            solve(3, false, false, true, seed + 800).as_deref(),
            opt_str(&case["tooShort"]),
            "seed {seed}"
        );
    }
    let pyra = PyraminxPuzzle::new();
    let mut r = JavaRandom::new(4);
    for case in as_array(&f["conversions"]) {
        let s = walk(pyra.solved_state(), &mut r, &case["moves"]).to_solver_state();
        assert_eq!(
            vec![s.edge_perm, s.edge_orient, s.corner_orient, s.tips],
            ints(&case["state"]),
            "{:?}",
            case["moves"]
        );
    }
}

#[test]
fn skewb_solver_matches_java() {
    let f = fixture("skewb_solver");
    let solver = SkewbSolver::new();
    for case in as_array(&f["random"]) {
        let seed = i64::from(as_i32(&case["seed"]));
        let state = solver.random_state(&mut JavaRandom::new(seed));
        assert_eq!(
            vec![state.perm, state.twst],
            ints(&case["state"]),
            "seed {seed}"
        );
        let solve = |len, exact, inverse, search_seed| {
            SkewbSolver::solve(
                state,
                len,
                exact,
                inverse,
                &mut JavaRandom::new(search_seed),
            )
        };
        assert_eq!(
            solve(11, false, false, seed + 500).as_deref(),
            opt_str(&case["solveIn"]),
            "seed {seed}"
        );
        assert_eq!(
            solve(11, true, true, seed + 700).as_deref(),
            opt_str(&case["exactly"]),
            "seed {seed}"
        );
        assert_eq!(
            solve(4, false, false, seed + 800).as_deref(),
            opt_str(&case["tooShort"]),
            "seed {seed}"
        );
    }
    let skewb = SkewbPuzzle::new();
    let mut r = JavaRandom::new(6);
    for (i, case) in as_array(&f["conversions"]).iter().enumerate() {
        let state = walk(skewb.solved_state(), &mut r, &case["moves"]);
        let s = state.to_solver_state();
        assert_eq!(
            vec![s.perm, s.twst],
            ints(&case["state"]),
            "{:?}",
            case["moves"]
        );
        let solution = SkewbSolver::solve(s, 11, false, false, &mut JavaRandom::new(i as i64));
        assert_eq!(
            solution.as_deref(),
            opt_str(&case["solveIn"]),
            "{:?}",
            case["moves"]
        );
        // The solution, read in fixed corner notation, solves the puzzle state.
        assert!(
            state
                .apply_algorithm(solution.as_deref().unwrap())
                .unwrap()
                .is_solved()
        );
    }
}
