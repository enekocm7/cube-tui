//! End-to-end differential tests of every TNoodle puzzle against the Java implementation.
//!
//! For each puzzle in the registry the fixture records metadata, move orderings, scrambles
//! for seeded `java.util.Random`s and SHA1PRNG seeds, SVG drawings, random walks with their
//! hashes and `solveIn` results, and `AlgorithmBuilder` behaviour. On the Java side, the
//! 3x3x3 minimum search time was zero and the Pyraminx and Skewb solvers were seeded with
//! [`SEARCH_SEED`], which made the time and entropy dependent scramblers deterministic.

mod common;

use std::time::Duration;

use common::{as_array, as_i32, as_str, fixture, opt_str, strings};
use serde_json::Value;
use tnoodle::java::{JavaRandom, RandomSource, choose};
use tnoodle::puzzle::{
    ClockPuzzle, CubePuzzle, FtoPuzzle, MegaminxPuzzle, PyraminxPuzzle, SkewbPuzzle,
    SquareOnePuzzle,
};
use tnoodle::scrambles::{
    AlgorithmBuilder, MergingMode, Puzzle, PuzzleRegistry, PuzzleState, Scrambler,
};

/// The search seed the Java recording used for the Pyraminx and Skewb solvers.
const SEARCH_SEED: i64 = 4242;

/// A puzzle configured exactly like the Java recording's (deterministic) one.
fn deterministic_scrambler(registry: PuzzleRegistry) -> Scrambler {
    let three = |p: CubePuzzle| Scrambler::Cube(p.with_min_search_time(Duration::ZERO));
    match registry {
        PuzzleRegistry::Two => Scrambler::Cube(CubePuzzle::two_by_two()),
        PuzzleRegistry::Three => three(CubePuzzle::three_by_three()),
        PuzzleRegistry::Four => Scrambler::Cube(CubePuzzle::four_by_four()),
        PuzzleRegistry::FourFast => Scrambler::Cube(CubePuzzle::four_by_four_random_turns()),
        PuzzleRegistry::Five => Scrambler::Cube(CubePuzzle::new(5)),
        PuzzleRegistry::Six => Scrambler::Cube(CubePuzzle::new(6)),
        PuzzleRegistry::Seven => Scrambler::Cube(CubePuzzle::new(7)),
        PuzzleRegistry::ThreeNi => three(CubePuzzle::three_by_three_no_inspection()),
        PuzzleRegistry::FourNi => Scrambler::Cube(CubePuzzle::four_by_four_no_inspection()),
        PuzzleRegistry::FiveNi => Scrambler::Cube(CubePuzzle::five_by_five_no_inspection()),
        PuzzleRegistry::ThreeFm => three(CubePuzzle::three_by_three_fewest_moves()),
        PuzzleRegistry::Pyra => {
            Scrambler::Pyraminx(PyraminxPuzzle::new().with_search_seed(SEARCH_SEED))
        }
        PuzzleRegistry::Sq1 => Scrambler::SquareOne(SquareOnePuzzle::new()),
        PuzzleRegistry::Mega => Scrambler::Megaminx(MegaminxPuzzle::new()),
        PuzzleRegistry::Clock => Scrambler::Clock(ClockPuzzle::new()),
        PuzzleRegistry::Skewb => Scrambler::Skewb(SkewbPuzzle::new().with_search_seed(SEARCH_SEED)),
        PuzzleRegistry::Fto => Scrambler::Fto(FtoPuzzle::new()),
    }
}

fn puzzle_fixture(registry: PuzzleRegistry) -> Value {
    as_array(&fixture("puzzles"))
        .iter()
        .find(|p| as_str(&p["registry"]) == registry.name())
        .unwrap_or_else(|| panic!("no fixture for {registry:?}"))
        .clone()
}

fn scheme_json(scheme: Option<&tnoodle::scrambles::ColorScheme>) -> Value {
    scheme.map_or(Value::Null, |s| {
        Value::Object(
            s.iter()
                .map(|(k, v)| (k.clone(), Value::String(v.to_hex())))
                .collect(),
        )
    })
}

/// Megaminx faces are drawn in identity-hash order in Java, so only the multiset of lines
/// is comparable.
fn normalize_svg(name: &str, svg: &str) -> Vec<String> {
    let mut lines: Vec<String> = svg.lines().map(str::to_owned).collect();
    if name == "minx" {
        lines.sort();
    }
    lines
}

/// Checks everything that is defined in terms of the statically typed puzzle.
fn check_typed<P: Puzzle>(p: &P, f: &Value) {
    let name = p.short_name();
    let solved = p.solved_state();
    assert_eq!(
        solved.java_hash_code(),
        as_i32(&f["solvedHash"]),
        "{name}: solved hash"
    );
    let names = |v: Vec<String>| v;
    assert_eq!(
        names(
            solved
                .successors_by_name()
                .into_iter()
                .map(|(m, _)| m)
                .collect()
        ),
        strings(&f["successorsByName"]),
        "{name}: successors"
    );
    assert_eq!(
        names(
            solved
                .canonical_moves_by_state()
                .into_iter()
                .map(|(_, m)| m)
                .collect()
        ),
        strings(&f["canonicalMoves"]),
        "{name}: canonical moves"
    );
    assert_eq!(
        names(
            solved
                .scramble_successors()
                .into_iter()
                .map(|(m, _)| m)
                .collect()
        ),
        strings(&f["scrambleSuccessors"]),
        "{name}: scramble successors"
    );

    for (w, walk) in as_array(&f["walks"]).iter().enumerate() {
        let mut wr = JavaRandom::new(1000 + w as i64);
        let mut state = solved.clone();
        for expected in strings(&walk["moves"]) {
            let succ = state.successors_by_name();
            let mv = choose(&mut wr, succ.iter().map(|(m, _)| m.clone())).unwrap();
            assert_eq!(mv, expected, "{name}: walk {w} move");
            state = succ.into_iter().find(|(m, _)| *m == mv).unwrap().1;
        }
        let ctx = format!("{name}: walk {w} {:?}", walk["moves"]);
        assert_eq!(state.java_hash_code(), as_i32(&walk["hash"]), "{ctx}: hash");
        assert_eq!(
            state.normalized().java_hash_code(),
            as_i32(&walk["normalizedHash"]),
            "{ctx}: normalized hash"
        );
        assert_eq!(
            state.is_solved(),
            walk["isSolved"].as_bool().unwrap(),
            "{ctx}: is solved"
        );
        assert_eq!(
            state
                .canonical_moves_by_state()
                .into_iter()
                .map(|(_, m)| m)
                .collect::<Vec<_>>(),
            strings(&walk["canonicalMoves"]),
            "{ctx}: canonical moves"
        );
        assert_eq!(
            state
                .scramble_successors()
                .into_iter()
                .map(|(m, _)| m)
                .collect::<Vec<_>>(),
            strings(&walk["scrambleSuccessors"]),
            "{ctx}: scramble successors"
        );
        if let Some(solutions) = walk.get("solutions") {
            for sol in as_array(solutions) {
                let sol = as_array(sol);
                let n = as_i32(&sol[0]);
                let solution = state.solve_in(n);
                assert_eq!(solution.as_deref(), opt_str(&sol[1]), "{ctx}: solveIn({n})");
                if let Some(s) = &solution {
                    assert!(
                        state.apply_algorithm(s).unwrap().is_solved(),
                        "{ctx}: solution {s} solves"
                    );
                }
            }
        }
    }

    let mut br = JavaRandom::new(77);
    for builder in as_array(&f["builders"]) {
        let mut state = solved.clone();
        let len = 1 + br.next_int_bounded(if name == "777" || name == "666" { 5 } else { 8 });
        let mut moves = Vec::new();
        for _ in 0..len {
            let succ = state.successors_by_name();
            let mv = choose(&mut br, succ.iter().map(|(m, _)| m.clone())).unwrap();
            state = succ.into_iter().find(|(m, _)| *m == mv).unwrap().1;
            moves.push(mv);
        }
        assert_eq!(moves, strings(&builder["moves"]), "{name}: builder moves");
        for (mode, key) in [
            (MergingMode::NoMerging, "NO_MERGING"),
            (MergingMode::CanonicalizeMoves, "CANONICALIZE_MOVES"),
        ] {
            let expected = &builder[key];
            let mut ab = AlgorithmBuilder::with_state(mode, solved.clone());
            let mut redundant = Vec::new();
            let mut error = None;
            for m in &moves {
                redundant.push(Value::Bool(ab.is_redundant(m).unwrap()));
                if let Err(e) = ab.append_move(m) {
                    error = Some(e.to_string());
                    break;
                }
            }
            let ctx = format!("{name}: {key} {moves:?}");
            assert_eq!(
                error.as_deref(),
                opt_str(&expected["error"]),
                "{ctx}: error"
            );
            assert_eq!(ab.to_string(), as_str(&expected["result"]), "{ctx}: result");
            assert_eq!(
                ab.total_cost(),
                as_i32(&expected["totalCost"]),
                "{ctx}: cost"
            );
            assert_eq!(
                &redundant,
                as_array(&expected["redundant"]),
                "{ctx}: redundancy"
            );
        }
    }

    for case in as_array(&f["invalidMoves"]) {
        let case = as_array(case);
        let result = solved.apply(as_str(&case[0])).err().map(|e| e.to_string());
        assert_eq!(
            result.as_deref(),
            opt_str(&case[1]),
            "{name}: apply {:?}",
            case[0]
        );
    }
}

fn check_puzzle(registry: PuzzleRegistry) {
    let f = puzzle_fixture(registry);
    let scrambler = deterministic_scrambler(registry);
    let name = scrambler.short_name().to_owned();
    assert_eq!(name, as_str(&f["shortName"]));
    assert_eq!(registry.key(), name);
    assert_eq!(scrambler.long_name(), as_str(&f["longName"]));
    assert_eq!(scrambler.to_string(), as_str(&f["toString"]));
    assert_eq!(
        scrambler.wca_min_scramble_distance(),
        as_i32(&f["wcaMinScrambleDistance"])
    );
    let size = scrambler.preferred_size();
    assert_eq!(
        vec![size.width, size.height],
        common::ints(&f["preferredSize"]),
        "{name}: size"
    );
    for case in as_array(&f["preferredSizes"]) {
        let c = common::ints(case);
        let d = scrambler.preferred_size_within(c[0], c[1]);
        assert_eq!(
            [d.width, d.height],
            [c[2], c[3]],
            "{name}: size within {}x{}",
            c[0],
            c[1]
        );
    }
    assert_eq!(
        scrambler.face_names(),
        strings(&f["faceNames"]),
        "{name}: face names"
    );
    assert_eq!(
        scheme_json(Some(&scrambler.default_color_scheme())),
        f["defaultColorScheme"],
        "{name}: scheme"
    );
    assert_eq!(
        scrambler.image_info().to_json(),
        serde_json::to_string(&f["imageInfo"]).unwrap(),
        "{name}: image info"
    );
    for case in as_array(&f["parsedColorSchemes"]) {
        let parsed = scrambler.parse_color_scheme(opt_str(&case["input"]));
        assert_eq!(
            scheme_json(parsed.as_ref()),
            case["output"],
            "{name}: parse {:?}",
            case["input"]
        );
    }

    for case in as_array(&f["scrambles"]) {
        let seed = i64::from(as_i32(&case["seed"]));
        let scramble = scrambler.generate_wca_scramble(&mut JavaRandom::new(seed));
        assert_eq!(
            scramble,
            as_str(&case["scramble"]),
            "{name}: scramble for seed {seed}"
        );
    }
    for case in as_array(&f["seededScrambles"]) {
        let seed = as_str(&case["seed"]);
        assert_eq!(
            scrambler.generate_seeded_scramble(seed),
            as_str(&case["scramble"]),
            "{name}: seeded {seed:?}"
        );
        assert_eq!(
            scrambler.generate_seeded_scrambles(seed, 2),
            strings(&case["scrambles"]),
            "{name}: seeded {seed:?}"
        );
    }

    for case in as_array(&f["svgs"]) {
        let scheme =
            opt_str(&case["scheme"]).map(|s| scrambler.parse_color_scheme(Some(s)).unwrap());
        let svg = scrambler
            .draw_scramble(opt_str(&case["scramble"]), scheme.as_ref())
            .unwrap()
            .to_string();
        let expected = as_str(&case["svg"]);
        let (actual, expected) = (normalize_svg(&name, &svg), normalize_svg(&name, expected));
        if let Some(i) =
            (0..actual.len().max(expected.len())).find(|&i| actual.get(i) != expected.get(i))
        {
            panic!(
                "{name}: svg for {:?} differs at line {i}:\n  rust: {:?}\n  java: {:?}",
                case["scramble"],
                actual.get(i),
                expected.get(i)
            );
        }
    }

    match &scrambler {
        Scrambler::Cube(p) => check_typed(p, &f),
        Scrambler::Clock(p) => check_typed(p, &f),
        Scrambler::Megaminx(p) => check_typed(p, &f),
        Scrambler::Pyraminx(p) => check_typed(p, &f),
        Scrambler::Skewb(p) => check_typed(p, &f),
        Scrambler::SquareOne(p) => check_typed(p, &f),
        Scrambler::Fto(p) => check_typed(p, &f),
    }
}

macro_rules! puzzle_tests {
    ($($test:ident => $registry:ident),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                check_puzzle(PuzzleRegistry::$registry);
            }
        )*
    };
}

puzzle_tests!(
    two_by_two_matches_java => Two,
    three_by_three_matches_java => Three,
    four_by_four_matches_java => Four,
    four_by_four_fast_matches_java => FourFast,
    five_by_five_matches_java => Five,
    six_by_six_matches_java => Six,
    seven_by_seven_matches_java => Seven,
    three_by_three_blindfolded_matches_java => ThreeNi,
    four_by_four_blindfolded_matches_java => FourNi,
    five_by_five_blindfolded_matches_java => FiveNi,
    three_by_three_fewest_moves_matches_java => ThreeFm,
    pyraminx_matches_java => Pyra,
    square_one_matches_java => Sq1,
    megaminx_matches_java => Mega,
    clock_matches_java => Clock,
    skewb_matches_java => Skewb,
    fto_matches_java => Fto,
);

#[test]
fn every_registry_entry_has_a_fixture() {
    let f = fixture("puzzles");
    let names: Vec<&str> = as_array(&f)
        .iter()
        .map(|p| as_str(&p["registry"]))
        .collect();
    let expected: Vec<&str> = PuzzleRegistry::ALL.iter().map(|r| r.name()).collect();
    assert_eq!(names, expected);
}
