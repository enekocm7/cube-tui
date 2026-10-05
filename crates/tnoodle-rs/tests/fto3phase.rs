//! Differential tests of the fto3phase solver against the Java implementation, plus ports
//! of `FtoCubieTest`.

mod common;

use common::{as_array, as_i32, as_str, fixture, ints};
use tnoodle::fto3phase::{FtoCubie, G3_MOVESET, Search, from_alg, moves};
use tnoodle::java::{JavaRandom, RandomSource};

#[test]
fn random_cubies_and_solutions_match_java() {
    let f = fixture("fto3phase");
    let search = Search::new();
    for case in as_array(&f["random"]) {
        let seed = i64::from(as_i32(&case["seed"]));
        let c = FtoCubie::random(&mut JavaRandom::new(seed));
        assert_eq!(
            c.corner_perm().to_vec(),
            ints(&case["cornerPerm"]),
            "seed {seed}"
        );
        assert_eq!(
            c.corner_ori().to_vec(),
            ints(&case["cornerOri"]),
            "seed {seed}"
        );
        assert_eq!(c.edges().to_vec(), ints(&case["edges"]), "seed {seed}");
        assert_eq!(
            c.triangles_ufbrbl().to_vec(),
            ints(&case["trianglesUFBrBl"]),
            "seed {seed}"
        );
        assert_eq!(
            c.triangles_rlbd().to_vec(),
            ints(&case["trianglesRLBD"]),
            "seed {seed}"
        );
        assert_eq!(c.java_hash(), as_i32(&case["hash"]), "seed {seed}");
        assert_eq!(
            search.solution(&c),
            as_str(&case["solution"]),
            "seed {seed}"
        );
    }
}

#[test]
fn coordinates_match_java() {
    let f = fixture("fto3phase");
    for case in as_array(&f["packs"]) {
        let mut c = FtoCubie::new();
        for m in ints(&case["moves"]) {
            c.turn(m as usize);
        }
        let ctx = format!("moves {:?}", case["moves"]);
        assert_eq!(c.pack_all_edges(), as_i32(&case["allEdges"]), "{ctx}");
        assert_eq!(
            c.pack_all_corner_permutation(),
            as_i32(&case["allCornerPermutation"]),
            "{ctx}"
        );
        assert_eq!(
            c.pack_all_corner_orientation(),
            as_i32(&case["allCornerOrientation"]),
            "{ctx}"
        );
        assert_eq!(
            c.pack_all_triangles(0),
            as_i32(&case["allTriangles0"]),
            "{ctx}"
        );
        assert_eq!(
            c.pack_all_triangles(1),
            as_i32(&case["allTriangles1"]),
            "{ctx}"
        );
        assert_eq!(c.g1_pack_edges(), as_i32(&case["g1Edges"]), "{ctx}");
        assert_eq!(c.g1_pack_triangles(), as_i32(&case["g1Triangles"]), "{ctx}");
        assert_eq!(c.g3_pack_corners(), as_i32(&case["g3Corners"]), "{ctx}");
        if !case["g2Edges"].is_null() {
            assert_eq!(c.g2_pack_edges(), as_i32(&case["g2Edges"]), "{ctx}");
            assert_eq!(c.g2_pack_triangles(), as_i32(&case["g2Triangles"]), "{ctx}");
            let triples: Vec<i32> = (0..4).map(|color| c.g2_pack_triples(color)).collect();
            assert_eq!(triples, ints(&case["g2Triples"]), "{ctx}");
        }
    }
}

const CW: [usize; 8] = [
    moves::R,
    moves::L,
    moves::B,
    moves::D,
    moves::U,
    moves::F,
    moves::BR,
    moves::BL,
];
const CCW: [usize; 8] = [
    moves::RP,
    moves::LP,
    moves::BP,
    moves::DP,
    moves::UP,
    moves::FP,
    moves::BRP,
    moves::BLP,
];

fn scrambled(r: &mut JavaRandom, moveset: &[usize]) -> FtoCubie {
    let mut c = FtoCubie::new();
    for _ in 0..100 {
        c.turn(moveset[r.next_int_bounded(moveset.len() as i32) as usize]);
    }
    c
}

#[test]
fn every_move_has_order_three_and_an_inverse() {
    for i in 0..8 {
        let mut c = FtoCubie::new();
        c.turn(CW[i]);
        c.turn(CW[i]);
        c.turn(CW[i]);
        assert_eq!(c, FtoCubie::new());
        assert_eq!(
            FtoCubie::new().turned(CW[i]).turned(CCW[i]),
            FtoCubie::new()
        );
    }
    let mut r = JavaRandom::new(1);
    for _ in 0..50 {
        let state = scrambled(&mut r, &(0..16).collect::<Vec<_>>());
        for m in (0..16).step_by(2) {
            assert_eq!(state.turned(m).turned(m + 1), state);
            assert_eq!(state.turned(m + 1).turned(m), state);
        }
    }
}

#[test]
fn coordinates_round_trip() {
    let all: Vec<usize> = (0..16).collect();
    let g2: Vec<usize> = (0..10).collect();
    let mut r = JavaRandom::new(2);
    for _ in 0..2000 {
        let c = scrambled(&mut r, &all);
        let mut t = FtoCubie::new();
        t.set_all_edges(c.pack_all_edges());
        assert_eq!(t.edges(), c.edges());
        t.set_all_triangles(c.pack_all_triangles(0), 0);
        assert_eq!(t.triangles_ufbrbl(), c.triangles_ufbrbl());
        t.set_all_corner_permutation(c.pack_all_corner_permutation());
        assert_eq!(t.corner_perm(), c.corner_perm());
        t.set_all_corner_orientation(c.pack_all_corner_orientation());
        assert_eq!(t.corner_ori(), c.corner_ori());
        let mut t = FtoCubie::new();
        t.g1_set_edges(c.g1_pack_edges());
        assert_eq!(t.g1_pack_edges(), c.g1_pack_edges());
        t.g1_set_triangles(c.g1_pack_triangles());
        assert_eq!(t.g1_pack_triangles(), c.g1_pack_triangles());

        let c = scrambled(&mut r, &g2);
        let mut t = FtoCubie::new();
        t.g2_set_triangles(c.g2_pack_triangles());
        assert_eq!(t.triangles_rlbd(), c.triangles_rlbd());
        t.g2_set_edges(c.g2_pack_edges());
        assert_eq!(t.g2_pack_edges(), c.g2_pack_edges());
        for color in 0..4 {
            let mut t = FtoCubie::new();
            t.g2_set_triples(c.g2_pack_triples(color), color);
            assert_eq!(t.g2_pack_triples(color), c.g2_pack_triples(color));
        }

        let c = scrambled(&mut r, &G3_MOVESET);
        let mut t = FtoCubie::new();
        t.g3_set_corners(c.g3_pack_corners());
        assert_eq!(t.g3_pack_corners(), c.g3_pack_corners());
    }
}

#[test]
fn solves_states_already_in_later_phases() {
    let search = Search::new();
    for alg in ["R U L D B U B L D B R L B D", "R L D B U B L B R L B", ""] {
        let state = from_alg(alg);
        let solution = search.solution(&state);
        assert_eq!(from_alg(&solution), state, "{alg:?}");
    }
}
