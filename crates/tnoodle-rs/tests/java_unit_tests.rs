//! Ports of TNoodle's own JUnit tests (`HugeScrambleTest`, `AlgorithmBuilderTest`,
//! `NoInspectionThreeByThreeTest`, `NoInspectionFiveByFiveTest`, `SquareOnePuzzleTest` and
//! `ThreeByThreeCubeFewestMovesTest`).

use std::collections::HashSet;
use std::sync::{Arc, Condvar, Mutex};

use tnoodle::java::{RandomSource, Sha1Prng, choose};
use tnoodle::puzzle::{
    ClockPuzzle, CubeMove, CubePuzzle, Face, MegaminxPuzzle, PyraminxPuzzle, SquareOnePuzzle,
    TwoByTwoSolver, pyraminx_coords, two_by_two,
};
use tnoodle::scrambles::{
    AlgorithmBuilder, CacheListener, MergingMode, Puzzle, PuzzleRegistry, PuzzleState,
    ScrambleCacher, Scrambler, split_algorithm,
};

fn secure_random() -> Sha1Prng {
    Sha1Prng::from_entropy()
}

/// Runs `f` with the statically typed puzzle behind a registry entry.
macro_rules! with_puzzle {
    ($scrambler:expr, $p:ident => $body:expr) => {
        match $scrambler {
            Scrambler::Cube($p) => $body,
            Scrambler::Clock($p) => $body,
            Scrambler::Megaminx($p) => $body,
            Scrambler::Pyraminx($p) => $body,
            Scrambler::Skewb($p) => $body,
            Scrambler::SquareOne($p) => $body,
            Scrambler::Fto($p) => $body,
        }
    };
}

#[test]
fn scramble_filtering() {
    let mut r = secure_random();
    for registry in PuzzleRegistry::ALL {
        let scrambler = registry.scrambler();
        for _ in 0..5 {
            let scramble = scrambler.generate_wca_scramble(&mut r);
            let solution = scrambler
                .solve_scramble_in(&scramble, scrambler.wca_min_scramble_distance() - 1)
                .unwrap();
            assert_eq!(
                solution,
                None,
                "{} scramble {scramble} is too short",
                scrambler.short_name()
            );
        }
    }
}

fn check_solve_in<P: Puzzle>(p: &P, r: &mut dyn RandomSource) {
    const SCRAMBLE_LENGTH: usize = 4;
    assert_eq!(
        p.solved_state().solve_in(0).as_deref(),
        Some(""),
        "{}",
        p.short_name()
    );
    for _ in 0..10 {
        let mut state = p.solved_state();
        let mut moves = Vec::new();
        for _ in 0..SCRAMBLE_LENGTH {
            let succ = state.successors_by_name();
            let mv = choose(r, succ.iter().map(|(m, _)| m.clone())).unwrap();
            state = succ.into_iter().find(|(m, _)| *m == mv).unwrap().1;
            moves.push(mv);
        }
        let solution = state
            .solve_in(SCRAMBLE_LENGTH as i32)
            .unwrap_or_else(|| panic!("{} solveIn failed for {moves:?}", p.short_name()));
        assert!(
            state.apply_algorithm(&solution).unwrap().is_solved(),
            "{}: {solution} does not solve {moves:?}",
            p.short_name()
        );
    }
}

#[test]
fn solve_in() {
    let mut r = secure_random();
    for registry in PuzzleRegistry::ALL {
        with_puzzle!(registry.scrambler(), p => check_solve_in(p, &mut r));
    }
}

#[test]
fn names() {
    for registry in PuzzleRegistry::ALL {
        assert_eq!(registry.key(), registry.scrambler().short_name());
        assert_eq!(registry.description(), registry.scrambler().long_name());
        assert_eq!(PuzzleRegistry::from_key(registry.key()), Some(registry));
    }
    assert_eq!(PuzzleRegistry::from_key("nope"), None);
}

#[test]
fn threads() {
    const SCRAMBLE_COUNT: usize = 10;
    for registry in PuzzleRegistry::ALL {
        let scrambler = registry.scrambler();
        with_puzzle!(scrambler, p => assert_eq!(
            p.solved_state().java_hash_code(),
            p.solved_state().java_hash_code()
        ));
        let scramble = scrambler.generate_scramble();
        scrambler.draw_scramble(Some(&scramble), None).unwrap();
        // `None` is the empty scramble.
        scrambler.draw_scramble(None, None).unwrap();

        // Generate and draw two sets of scrambles simultaneously to shake out threading
        // problems.
        let done = Arc::new((Mutex::new(0), Condvar::new()));
        let stopper: CacheListener = {
            let done = Arc::clone(&done);
            Arc::new(move |cacher: &ScrambleCacher| {
                if cacher.available_count() == cacher.cache_size() && cacher.is_running() {
                    cacher.stop();
                    let (count, cvar) = &*done;
                    *count.lock().unwrap() += 1;
                    cvar.notify_all();
                }
            })
        };
        let c1 = ScrambleCacher::with_options(
            scrambler,
            SCRAMBLE_COUNT,
            true,
            vec![Arc::clone(&stopper)],
        );
        let c2 = ScrambleCacher::with_options(scrambler, SCRAMBLE_COUNT, true, vec![stopper]);
        let (count, cvar) = &*done;
        let mut finished = count.lock().unwrap();
        while *finished < 2 {
            finished = cvar.wait(finished).unwrap();
        }
        drop(finished);
        assert!(!c1.is_running() && !c2.is_running());
        let scrambles = c1.new_scrambles(SCRAMBLE_COUNT).unwrap();
        assert_eq!(scrambles.len(), SCRAMBLE_COUNT);
        assert_eq!(c2.available_count(), SCRAMBLE_COUNT);
        assert_eq!(c1.available_count(), 0);
    }
}

#[test]
fn clock_solve_in_does_not_break() {
    let clock = ClockPuzzle::new();
    let state = clock
        .solved_state()
        .apply_algorithm("ALL2+ y2 ALL1-")
        .unwrap();
    if let Some(solution) = state.solve_in(3) {
        assert!(state.apply_algorithm(&solution).unwrap().is_solved());
    }
}

#[test]
fn cube_normalization() {
    let fours = CubePuzzle::new(4);
    let solved = fours.solved_state();
    let state = solved.apply_algorithm("Rw Lw'").unwrap();
    let normalized = state.normalized().into_owned();
    let solved_normalized = solved.normalized().into_owned();
    assert_eq!(normalized, solved_normalized);
    assert_eq!(
        normalized.java_hash_code(),
        solved_normalized.java_hash_code()
    );
    let state = solved.apply_algorithm("Uw Dw'").unwrap();
    assert_eq!(*state.normalized(), solved_normalized);

    let threes = CubePuzzle::three_by_three();
    let solved3 = threes.solved_state();
    let b_done = solved3.apply("B").unwrap();
    let fw_done = solved3.apply("Fw").unwrap();
    assert!(b_done.equals_normalized(&fw_done));

    let mut ab3 = AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, solved3);
    let alg = "D2 U' L2 B2 F2 D B2 U' B2 F D' F U' R F2 L2 D' B D F'";
    ab3.append_algorithm(alg).unwrap();
    assert_eq!(ab3.to_string(), alg);

    let mut r = secure_random();
    let mut state = state;
    for _ in 0..100 {
        let succ = state.successors_by_name();
        state = choose(&mut r, succ.into_iter().map(|(_, s)| s)).unwrap();
        let rotated = state.apply_algorithm("Uw Dw'").unwrap();
        assert_eq!(*state.normalized(), *rotated.normalized());
    }
}

#[test]
fn algorithm_builder() {
    let fours = CubePuzzle::new(4);
    let mut ab4 =
        AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, fours.solved_state());
    ab4.append_algorithm("Rw Lw").unwrap();
    assert_eq!(split_algorithm(&ab4.to_string()).len(), 1);

    let sq1 = SquareOnePuzzle::new();
    let mut ab = AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, sq1.solved_state());
    ab.append_algorithm("(1,0) (0,1)").unwrap();
    assert_eq!(ab.to_string(), "(1,1)");
    let mut ab = AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, sq1.solved_state());
    ab.append_algorithm("(0,1) (1,1)").unwrap();
    assert_eq!(ab.to_string(), "(1,2)");

    let fives = CubePuzzle::new(5);
    let mut ab5 = AlgorithmBuilder::with_state(MergingMode::NoMerging, fives.solved_state());
    ab5.append_algorithm("U R 4Rw'").unwrap();
    assert_eq!(ab5.to_string(), "U R 4Rw'");
}

#[test]
fn twos_converter() {
    const MOVE_R: usize = 3;
    const MOVE_R_PRIME: usize = 5;
    let (move_perm, move_orient) = two_by_two::move_tables();
    let orient = move_orient[0][MOVE_R];
    let permute = move_perm[0][MOVE_R];
    let state = CubePuzzle::new(2)
        .solved_state()
        .apply("R")
        .unwrap()
        .to_two_by_two_state();
    assert_eq!(state.orientation, orient);
    assert_eq!(state.permutation, permute);
    assert_eq!(
        TwoByTwoSolver::new().solve_in(state, 1).as_deref(),
        Some("R'")
    );
    assert_eq!(move_orient[orient as usize][MOVE_R_PRIME], 0);
    assert_eq!(move_perm[permute as usize][MOVE_R_PRIME], 0);
}

#[test]
fn twos_solver() {
    let state = CubePuzzle::new(2).solved_state();
    assert_eq!(state.solve_in(0).as_deref(), Some(""));
    let state = state.apply_algorithm("R2 B2 F2").unwrap();
    let solution = state.solve_in(1).unwrap();
    assert!(state.apply_algorithm(&solution).unwrap().is_solved());
}

#[test]
fn pyra_converter() {
    const MOVES: [&str; 8] = ["U", "U'", "L", "L'", "R", "R'", "B", "B'"];
    let (edge_perm_t, edge_orient_t, corner_orient_t) = pyraminx_coords::move_tables();
    let pyra = PyraminxPuzzle::new();
    let s = pyra.solved_state().to_solver_state();
    assert_eq!(
        (s.edge_perm, s.edge_orient, s.corner_orient, s.tips),
        (0, 0, 0, 0)
    );
    let mut r = secure_random();
    for _ in 0..1000 {
        let (mut edge_perm, mut edge_orient, mut corner_orient) = (0, 0, 0);
        let mut state = pyra.solved_state();
        for _ in 0..20 {
            let mv = r.next_int_bounded(MOVES.len() as i32) as usize;
            edge_perm = edge_perm_t[edge_perm as usize][mv];
            edge_orient = edge_orient_t[edge_orient as usize][mv];
            corner_orient = corner_orient_t[corner_orient as usize][mv];
            state = state.apply(MOVES[mv]).unwrap();
        }
        let s = state.to_solver_state();
        assert_eq!(
            (s.edge_perm, s.edge_orient, s.corner_orient),
            (edge_perm, edge_orient, corner_orient)
        );
    }
    assert_eq!(pyraminx_coords::pack_edge_perm(&[0, 1, 2, 3, 4, 5]), 0);
    assert_eq!(pyraminx_coords::pack_edge_orient(&[0; 6]), 0);
    assert_eq!(pyraminx_coords::pack_corner_orient(&[0; 4]), 0);
}

#[test]
fn mega_spins_are_rotations() {
    let megaminx = MegaminxPuzzle::new();
    let solved = megaminx.solved_state();
    let spin_l = "R++ L2'";
    let spin_u = "D++ U2'";
    let mut state = solved.clone();
    for alg in [spin_l, spin_u, spin_u, spin_l, spin_l, spin_l, spin_u] {
        state = state.apply_algorithm(alg).unwrap();
    }
    assert!(state.equals_normalized(&solved));
}

#[test]
fn every_move_applied_twice_is_redundant() {
    let sixes = CubePuzzle::new(6);
    let moves: HashSet<String> = sixes
        .solved_state()
        .scramble_successors()
        .into_iter()
        .map(|(m, _)| m)
        .collect();
    for redundant in ["3Bw", "3Lw", "3Dw"] {
        assert!(!moves.contains(redundant));
    }
    for registry in PuzzleRegistry::ALL {
        with_puzzle!(registry.scrambler(), p => {
            for (mv, _) in p.solved_state().successors_by_name() {
                let mut ab = AlgorithmBuilder::with_state(MergingMode::NoMerging, p.solved_state());
                ab.append_algorithm(&mv).unwrap();
                assert!(ab.is_redundant(&mv).unwrap(), "{}: {mv}", p.short_name());
            }
        });
    }
}

#[test]
fn five_by_five_no_inspection_orientation() {
    let fives = CubePuzzle::five_by_five_no_inspection();
    let reorient = [CubeMove::new(Face::U, 1, 3)];
    assert_eq!(reorient[0].name(5).as_deref(), Some("4Uw"));
    let generate = |alg: &str| {
        let mut ab = AlgorithmBuilder::with_state(MergingMode::NoMerging, fives.solved_state());
        ab.append_algorithm(alg).unwrap();
        fives
            .apply_orientation(&reorient, ab.state_and_generator())
            .generator
    };
    // F R and 4Uw don't conflict.
    assert_eq!(generate("F R"), "F R 4Uw");
    // The D turn is redundant with 4Uw and is removed.
    assert_eq!(generate("F D"), "F 4Uw");
    assert_eq!(generate("D U D U"), "U U 4Uw");
}

fn opposite(face: &str) -> &'static str {
    let faces = "URFDLB";
    let opposites = ["D", "L", "B", "U", "R", "F"];
    opposites[faces.find(face).unwrap()]
}

#[test]
fn three_by_three_no_inspection() {
    let canonical: HashSet<String> = CubePuzzle::two_by_two()
        .solved_state()
        .canonical_moves_by_state()
        .into_iter()
        .map(|(_, m)| m)
        .collect();
    let desired: HashSet<String> = "RUF"
        .chars()
        .flat_map(|f| ["", "'", "2"].map(|d| format!("{f}{d}")))
        .collect();
    assert_eq!(canonical, desired);

    let threes = CubePuzzle::three_by_three_no_inspection();
    let solved = threes.solved_state();
    for modifier in ["", "'", "2"] {
        for face in "URFDLB".chars() {
            let face = face.to_string();
            for restriction in [face.as_str(), opposite(&face)] {
                let state = solved.apply(&format!("{restriction}{modifier}")).unwrap();
                let solution =
                    CubePuzzle::solve_in_restricted(&state, 20, Some(restriction), None).unwrap();
                assert!(
                    !solution.starts_with(restriction),
                    "{solution} starts with {restriction}"
                );
                assert!(!solution.starts_with(opposite(restriction)));
                assert!(state.apply_algorithm(&solution).unwrap().is_solved());
            }
        }
    }

    let scrambled = solved.apply_algorithm("L' R2 U D2 L2").unwrap();
    let solution = CubePuzzle::solve_in_restricted(&scrambled, 20, Some("L"), None).unwrap();
    assert!(!solution.starts_with('L'));
    assert!(scrambled.apply_algorithm(&solution).unwrap().is_solved());

    // min2phase also searches the inverse cube, which must respect the restriction too.
    let scrambled = solved
        .apply_algorithm("F D B L' U L' F D' L2 D L' B2 D F2 U B2 R2 U D2 L2")
        .unwrap();
    let solution = CubePuzzle::solve_in_restricted(&scrambled, 20, Some("L"), None).unwrap();
    assert!(!solution.starts_with('L') && !solution.starts_with('R'));
    assert!(scrambled.apply_algorithm(&solution).unwrap().is_solved());

    let mut r = secure_random();
    for _ in 0..3 {
        let scramble = threes.generate_wca_scramble(&mut r);
        assert!(threes.solved_state().apply_algorithm(&scramble).is_ok());
    }
}

#[test]
fn three_by_three_fewest_moves() {
    let threes = CubePuzzle::three_by_three_fewest_moves();
    let canonical: HashSet<String> = threes
        .solved_state()
        .canonical_moves_by_state()
        .into_iter()
        .map(|(_, m)| m)
        .collect();
    let desired: HashSet<String> = "RUFLDB"
        .chars()
        .flat_map(|f| ["", "'", "2"].map(|d| format!("{f}{d}")))
        .collect();
    assert_eq!(canonical, desired);

    let solved = threes.solved_state();
    for modifier in ["", "'", "2"] {
        for first in "URFDLB".chars() {
            for last in "URFDLB".chars() {
                let (first, last) = (first.to_string(), last.to_string());
                let state = solved.apply(&format!("{first}{modifier}")).unwrap();
                let solution =
                    CubePuzzle::solve_in_restricted(&state, 20, Some(&first), Some(&last)).unwrap();
                assert!(state.apply_algorithm(&solution).unwrap().is_solved());
                let moves = split_algorithm(&solution);
                let (first_move, last_move) = (moves[0], moves[moves.len() - 1]);
                assert!(
                    !first_move.starts_with(first.as_str())
                        && !first_move.starts_with(opposite(&first))
                );
                assert!(
                    !last_move.starts_with(last.as_str())
                        && !last_move.starts_with(opposite(&last))
                );
            }
        }
    }

    let mut r = secure_random();
    for _ in 0..3 {
        let uncancelled = threes.generate_wca_scramble(&mut r);
        let mut ab =
            AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, threes.solved_state());
        ab.append_algorithm(&uncancelled).unwrap();
        let scramble = ab.state_and_generator().generator;
        assert_eq!(scramble.len(), uncancelled.len());
        assert!(scramble.starts_with("R' U' F"));
        assert!(scramble.ends_with("R' U' F"));
    }
}

#[test]
fn square_one_merging_mode() {
    let sq1 = SquareOnePuzzle::new();
    let mut ab = AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, sq1.solved_state());
    assert_eq!(ab.total_cost(), 0);
    for (mv, cost) in [
        ("(1,0)", 1),
        ("(2,0)", 1),
        ("(0,-1)", 1),
        ("/", 2),
        ("/", 1),
    ] {
        ab.append_move(mv).unwrap();
        assert_eq!(ab.total_cost(), cost, "after {mv}");
    }
    let state = ab.state();
    assert_eq!(state.solve_in(1).as_deref(), Some("(-3,1)"));
    assert_eq!(state.solve_in(2).as_deref(), Some("(-3,1)"));
}

#[test]
fn square_one_slashability_solutions() {
    let sq1 = SquareOnePuzzle::new();
    for scramble in ["(3,0) / (4,0)", "(3,0) / (1,0)"] {
        let mut ab =
            AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, sq1.solved_state());
        ab.append_algorithm(scramble).unwrap();
        let state = ab.state();
        let solution = state
            .solve_in(3)
            .unwrap_or_else(|| panic!("no solution for {scramble}"));
        assert!(state.apply_algorithm(&solution).unwrap().is_solved());
    }
}

#[test]
fn min2phase_benchmark_style_solves() {
    let mut r = secure_random();
    let mut search = tnoodle::min2phase::Search::new();
    for _ in 0..20 {
        let facelets = tnoodle::min2phase::tools::random_cube(&mut r);
        let solution =
            search.solution(&facelets, 21, 5000, 0, tnoodle::min2phase::INVERSE_SOLUTION);
        assert!(!solution.starts_with("Error"), "{solution}");
        assert_eq!(
            tnoodle::min2phase::tools::from_scramble(&solution),
            facelets
        );
    }
}
