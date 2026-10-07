//! The puzzle abstraction (`Puzzle` and `Puzzle.PuzzleState`).

use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use super::algorithm_builder::{AlgorithmBuilder, MergingMode, split_algorithm};
use super::solve::solve_in_bfs;
use crate::error::{InvalidMoveError, InvalidScrambleError};
use crate::java::{
    JavaHash, RandomSource, Sha1Prng, choose, java_hash_order, split_dropping_trailing_empty,
};
use crate::svg::{Color, Dimension, Element, Svg};

/// A mapping from face names to colours. Missing faces fall back to the puzzle's defaults
/// when drawing.
pub type ColorScheme = BTreeMap<String, Color>;

/// A puzzle state and the move sequence that produced it (`PuzzleStateAndGenerator`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleStateAndGenerator<S> {
    /// The state reached by applying `generator` to the solved state.
    pub state: S,
    /// The move sequence.
    pub generator: String,
}

/// A state of a puzzle (`Puzzle.PuzzleState`).
///
/// States are immutable values: moves return new states. Two states that differ only by a
/// whole-puzzle rotation are not equal, but have equal [normalized](Self::normalized) forms.
pub trait PuzzleState: Clone + Eq + Hash + Debug + Send + Sync {
    /// The successors of this state by move name, in the iteration order of Java's
    /// `getSuccessorsByName()`. Preferred notations come first.
    fn successors_by_name(&self) -> Vec<(String, Self)>;

    /// The `hashCode()` of the Java state, which determines `HashMap` iteration orders.
    fn java_hash_code(&self) -> i32;

    /// The solved state of this state's puzzle (`getPuzzle().getSolvedState()`).
    #[must_use]
    fn solved(&self) -> Self;

    /// Draws the state.
    fn draw(&self, color_scheme: &ColorScheme) -> Svg;

    /// A representative that all rotations of this state normalize to.
    fn normalized(&self) -> Cow<'_, Self> {
        Cow::Borrowed(self)
    }

    /// Whether this state is its own normalized form.
    fn is_normalized(&self) -> bool {
        *self == *self.normalized()
    }

    /// Whether the two states are a whole-puzzle rotation apart.
    fn equals_normalized(&self, other: &Self) -> bool {
        *self.normalized() == *other.normalized()
    }

    /// The cost of a move, e.g. a Square-1 `(3,3)` counts as one move.
    fn move_cost(&self, _mv: &str) -> i32 {
        1
    }

    /// The successors that are unique up to rotation, keyed by state, in the iteration
    /// order of Java's `getCanonicalMovesByState()` (a `HashMap` keyed by state).
    fn canonical_moves_by_state(&self) -> Vec<(Self, String)> {
        let mut seen_normalized: HashSet<Self> = HashSet::new();
        seen_normalized.insert(self.normalized().into_owned());
        let unique = self
            .successors_by_name()
            .into_iter()
            .filter_map(|(name, state)| {
                let normalized = state.normalized().into_owned();
                seen_normalized
                    .insert(normalized)
                    .then(|| (ByJavaHash(state), name))
            });
        order_by_state_hash(unique.map(|(k, v)| (k.0, v)).collect())
    }

    /// The name of the first of the [`canonical_moves_by_state`](Self::canonical_moves_by_state)
    /// that leads to a state equal to `target` up to rotation. `target` must be normalized.
    /// Puzzles can override this with a faster search giving the same answer.
    fn canonical_move_to(&self, target: &Self) -> Option<String> {
        first_canonical_move_to(self, target)
    }

    /// The moves used to generate random-turn scrambles, in the iteration order of Java's
    /// `getScrambleSuccessors()` (a `HashMap` keyed by move name by default).
    fn scramble_successors(&self) -> Vec<(String, Self)> {
        java_hash_order(
            self.canonical_moves_by_state()
                .into_iter()
                .map(|(state, name)| (name, state)),
        )
    }

    /// The names of [`scramble_successors`](Self::scramble_successors), in the same order.
    /// Puzzles that know them without computing the successor states can override this.
    fn scramble_successor_names(&self) -> Vec<String> {
        self.scramble_successors()
            .into_iter()
            .map(|(m, _)| m)
            .collect()
    }

    /// Applies one move.
    fn apply(&self, mv: &str) -> Result<Self, InvalidMoveError> {
        self.successors_by_name()
            .into_iter()
            .find_map(|(name, state)| (name == mv).then_some(state))
            .ok_or_else(|| InvalidMoveError::unrecognized(mv))
    }

    /// Applies a whitespace separated sequence of moves.
    fn apply_algorithm(&self, algorithm: &str) -> Result<Self, InvalidScrambleError> {
        let mut state = self.clone();
        for mv in split_algorithm(algorithm) {
            state = state
                .apply(mv)
                .map_err(|e| InvalidScrambleError::new(algorithm, e))?;
        }
        Ok(state)
    }

    /// Whether this state is solved (up to rotation).
    fn is_solved(&self) -> bool {
        self.equals_normalized(&self.solved())
    }

    /// A solution of at most `n` moves, or `None` if there is none.
    fn solve_in(&self, n: i32) -> Option<String> {
        solve_in_bfs(self, n)
    }

    /// Whether `move1` and `move2` lead to the same state in either order from this state.
    /// Unknown moves never commute.
    fn moves_commute(&self, move1: &str, move2: &str) -> bool {
        let state1 = self.apply(move1).and_then(|s| s.apply(move2));
        let state2 = self.apply(move2).and_then(|s| s.apply(move1));
        matches!((state1, state2), (Ok(a), Ok(b)) if a == b)
    }
}

/// The default [`PuzzleState::canonical_move_to`].
pub(crate) fn first_canonical_move_to<S: PuzzleState>(state: &S, target: &S) -> Option<String> {
    state
        .canonical_moves_by_state()
        .into_iter()
        .find_map(|(ps, name)| (*ps.normalized() == *target).then_some(name))
}

/// Reorders `(state, move)` pairs into the iteration order of a Java `HashMap` keyed by
/// state into which they were inserted in the given order.
pub(crate) fn order_by_state_hash<S: PuzzleState>(items: Vec<(S, String)>) -> Vec<(S, String)> {
    java_hash_order(items.into_iter().map(|(s, n)| (ByJavaHash(s), n)))
        .into_iter()
        .map(|(k, v)| (k.0, v))
        .collect()
}

/// Keys a state by its Java hash code, to reproduce `HashMap<PuzzleState, _>` orders.
#[derive(PartialEq, Eq)]
struct ByJavaHash<S>(S);

impl<S: PuzzleState> JavaHash for ByJavaHash<S> {
    fn java_hash(&self) -> i32 {
        self.0.java_hash_code()
    }
}

/// A twisty puzzle that can be scrambled and drawn (`Puzzle`).
pub trait Puzzle: Send + Sync {
    /// The type of the puzzle's states.
    type State: PuzzleState;

    /// A URL friendly name, e.g. `333`.
    fn short_name(&self) -> &str;

    /// A human readable name, e.g. `3x3x3`.
    fn long_name(&self) -> &str;

    /// The minimum distance from solved of every WCA scramble.
    fn wca_min_scramble_distance(&self) -> i32;

    /// The default colour scheme, in the order the Java source inserts it into its map.
    fn default_color_scheme_entries(&self) -> &'static [(&'static str, Color)];

    /// The natural size of the puzzle's drawing.
    fn preferred_size(&self) -> Dimension;

    /// The solved state, from which scrambles are applied.
    fn solved_state(&self) -> Self::State;

    /// How many random moves make a sufficiently scrambled random-turn scramble.
    fn random_move_count(&self) -> i32;

    /// How far along the (possibly slow) solver initialisation is, from 0 to 1.
    fn initialization_status(&self) -> f64 {
        1.0
    }

    /// Generates a scramble and the state it produces. The default generates
    /// [`random_move_count`](Self::random_move_count) non-cancelling random turns.
    fn generate_random_moves(
        &self,
        r: &mut dyn RandomSource,
    ) -> PuzzleStateAndGenerator<Self::State> {
        generate_random_turns(self.solved_state(), self.random_move_count(), r)
    }

    /// Generates a scramble that is at least
    /// [`wca_min_scramble_distance`](Self::wca_min_scramble_distance) moves from solved.
    fn generate_wca_scramble(&self, r: &mut dyn RandomSource) -> String {
        loop {
            let psag = self.generate_random_moves(r);
            if psag
                .state
                .solve_in(self.wca_min_scramble_distance() - 1)
                .is_none()
            {
                return psag.generator;
            }
        }
    }

    /// Generates a WCA scramble from a SHA1PRNG seeded with operating system entropy.
    fn generate_scramble(&self) -> String {
        self.generate_wca_scramble(&mut Sha1Prng::from_entropy())
    }

    /// Generates `count` WCA scrambles from a SHA1PRNG seeded with operating system
    /// entropy.
    fn generate_scrambles(&self, count: usize) -> Vec<String> {
        let mut r = Sha1Prng::from_entropy();
        (0..count)
            .map(|_| self.generate_wca_scramble(&mut r))
            .collect()
    }

    /// Generates a WCA scramble that is fully determined by `seed`, using the same
    /// SHA1PRNG as TNoodle so that the scramble matches TNoodle's.
    fn generate_seeded_scramble(&self, seed: &str) -> String {
        self.generate_wca_scramble(&mut Sha1Prng::with_seed(seed.as_bytes()))
    }

    /// Generates `count` WCA scrambles fully determined by `seed`.
    fn generate_seeded_scrambles(&self, seed: &str, count: usize) -> Vec<String> {
        let mut r = Sha1Prng::with_seed(seed.as_bytes());
        (0..count)
            .map(|_| self.generate_wca_scramble(&mut r))
            .collect()
    }

    /// The default colour scheme.
    fn default_color_scheme(&self) -> ColorScheme {
        self.default_color_scheme_entries()
            .iter()
            .map(|&(face, color)| (face.to_owned(), color))
            .collect()
    }

    /// The face names, sorted alphabetically.
    fn face_names(&self) -> Vec<String> {
        self.default_color_scheme().into_keys().collect()
    }

    /// Parses a colour scheme: either comma separated colours, or one hex digit per face,
    /// assigned to the faces in [`face_names`](Self::face_names) order. `None` or an empty
    /// string gives the default scheme; an invalid scheme gives `None`.
    fn parse_color_scheme(&self, scheme: Option<&str>) -> Option<ColorScheme> {
        let mut color_scheme = self.default_color_scheme();
        let Some(scheme) = scheme.filter(|s| !s.is_empty()) else {
            return Some(color_scheme);
        };
        let faces = self.face_names();
        let colors: Vec<String> = if scheme.find(',').is_some_and(|i| i > 0) {
            split_dropping_trailing_empty(scheme, ",")
                .into_iter()
                .map(str::to_owned)
                .collect()
        } else {
            scheme.chars().map(String::from).collect()
        };
        if colors.len() != faces.len() {
            return None;
        }
        for (face, color) in faces.into_iter().zip(colors) {
            color_scheme.insert(face, Color::from_hex(&color).ok()?);
        }
        Some(color_scheme)
    }

    /// The best size to draw the puzzle within `max_width` x `max_height` (0 means
    /// unconstrained), keeping the aspect ratio.
    fn preferred_size_within(&self, max_width: i32, max_height: i32) -> Dimension {
        if max_width == 0 && max_height == 0 {
            return self.preferred_size();
        }
        let (mut max_w, mut max_h) = (max_width, max_height);
        if max_w == 0 {
            max_w = i32::MAX;
        } else if max_h == 0 {
            max_h = i32::MAX;
        }
        let preferred = self.preferred_size();
        let ratio = f64::from(preferred.width) / f64::from(preferred.height);
        let width = f64::from(max_w).min((f64::from(max_h) * ratio).ceil()) as i32;
        let height = f64::from(max_h).min((f64::from(max_w) / ratio).ceil()) as i32;
        Dimension::new(width, height)
    }

    /// Draws the state reached by `scramble` (`None` is the solved state). Colours missing
    /// from `color_scheme` come from the default scheme.
    fn draw_scramble(
        &self,
        scramble: Option<&str>,
        color_scheme: Option<&ColorScheme>,
    ) -> Result<Svg, InvalidScrambleError> {
        let mut scheme = self.default_color_scheme();
        if let Some(overrides) = color_scheme {
            scheme.extend(overrides.iter().map(|(k, v)| (k.clone(), *v)));
        }
        let state = self
            .solved_state()
            .apply_algorithm(scramble.unwrap_or(""))?;
        let mut svg = state.draw(&scheme);

        // Moving everything half a pixel prevents aliasing of horizontal and vertical lines.
        let mut g = Element::group();
        for child in std::mem::take(svg.root_mut().children_mut()) {
            g.append_child(child);
        }
        g.translate(0.5, 0.5);
        svg.append_child(g);
        Ok(svg)
    }
}

/// `Puzzle.generateRandomMoves`: `move_count` random turns, never choosing a move that
/// is redundant with the ones before it.
pub(crate) fn generate_random_turns<S: PuzzleState>(
    solved: S,
    move_count: i32,
    r: &mut dyn RandomSource,
) -> PuzzleStateAndGenerator<S> {
    let mut ab = AlgorithmBuilder::with_state(MergingMode::NoMerging, solved);
    while ab.total_cost() < move_count {
        let mut successors = ab.state().scramble_successor_names();
        let mv = loop {
            let mv = choose(r, successors.iter().cloned())
                .expect("a puzzle state always has a non-redundant move");
            // If this move happens to be redundant, there is no reason to try it again.
            successors.retain(|m| *m != mv);
            if !ab
                .is_redundant(&mv)
                .expect("scramble successors are valid moves")
            {
                break mv;
            }
        };
        ab.append_move(&mv)
            .expect("scramble successors are valid moves");
    }
    ab.state_and_generator()
}
