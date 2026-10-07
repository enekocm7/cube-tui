//! Building move sequences while merging redundant moves (`AlgorithmBuilder`).

use std::fmt;

use super::puzzle::{PuzzleState, PuzzleStateAndGenerator};
use crate::error::InvalidMoveError;

/// How aggressively [`AlgorithmBuilder`] merges moves. Examples are on a 3x3x3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MergingMode {
    /// Blindly append moves: `R R` stays unmodified.
    NoMerging,
    /// Merge moves into canonical moves: `R R` becomes `R2`, `L Rw` becomes `L2` and
    /// `F x U` becomes `F2`. The final state may differ from the unmerged one by a rotation.
    CanonicalizeMoves,
}

/// Where a move would go in the sequence, and what it would become (`IndexAndMove`).
///
/// `mv` is `None` when the move cancels the move at `index` (or is a rotation, which
/// TNoodle reports as cancelling the first move).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexAndMove {
    /// The position in the move sequence.
    pub index: usize,
    /// The resulting move, if any.
    pub mv: Option<String>,
}

/// Splits an algorithm into moves exactly like Java's `algorithm.split("\\s+")`, including
/// a leading empty move when the algorithm starts with whitespace (which is then rejected
/// as an invalid move). A blank algorithm has no moves.
pub fn split_algorithm(algorithm: &str) -> Vec<&str> {
    // `String.trim()` strips every character up to U+0020.
    if algorithm.chars().all(|c| c <= ' ') {
        return Vec::new();
    }
    let is_java_whitespace = |c: char| matches!(c, ' ' | '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r');
    let mut moves: Vec<&str> = algorithm.split(is_java_whitespace).collect();
    // Runs of whitespace are a single separator.
    let mut out = Vec::with_capacity(moves.len());
    let mut first = true;
    for m in moves.drain(..) {
        if first || !m.is_empty() {
            out.push(m);
        }
        first = false;
    }
    while out.last().is_some_and(|m| m.is_empty()) {
        out.pop();
    }
    out
}

/// Builds a move sequence for a puzzle, optionally merging redundant moves.
#[derive(Debug, Clone)]
pub struct AlgorithmBuilder<S: PuzzleState> {
    moves: Vec<Option<String>>,
    /// `states[i]` is the state after `moves[0..i]`.
    states: Vec<S>,
    original_state: S,
    /// The state reached by naively appending every move, which can differ by a rotation
    /// from the merged state.
    un_normalized_state: S,
    total_cost: i32,
    merging_mode: MergingMode,
}

impl<S: PuzzleState> AlgorithmBuilder<S> {
    /// A builder starting from `original_state`.
    pub fn with_state(merging_mode: MergingMode, original_state: S) -> Self {
        Self {
            moves: Vec::new(),
            states: vec![original_state.clone()],
            un_normalized_state: original_state.clone(),
            original_state,
            total_cost: 0,
            merging_mode,
        }
    }

    fn reset_to_state(&mut self, original_state: S) {
        self.total_cost = 0;
        self.un_normalized_state = original_state.clone();
        self.moves.clear();
        self.states.clear();
        self.states.push(original_state.clone());
        self.original_state = original_state;
    }

    /// Whether appending `mv` would merge with or cancel a previous move.
    pub fn is_redundant(&self, mv: &str) -> Result<bool, InvalidMoveError> {
        let im = self.find_best_index_for_move(mv, MergingMode::CanonicalizeMoves)?;
        Ok(im.index < self.moves.len() || im.mv.is_none())
    }

    /// Where `mv` would be merged into the sequence under `merging_mode`.
    pub fn find_best_index_for_move(
        &self,
        mv: &str,
        merging_mode: MergingMode,
    ) -> Result<IndexAndMove, InvalidMoveError> {
        if merging_mode == MergingMode::NoMerging {
            return Ok(IndexAndMove {
                index: self.moves.len(),
                mv: Some(mv.to_owned()),
            });
        }

        let new_un_normalized = self.un_normalized_state.apply(mv)?;
        if new_un_normalized.equals_normalized(&self.un_normalized_state) {
            // The move must just be a rotation.
            return Ok(IndexAndMove { index: 0, mv: None });
        }
        let new_normalized = new_un_normalized.normalized();

        let mv = self.state().canonical_move_to(&new_normalized);

        for last_index in (0..self.moves.len()).rev() {
            let state_before = &self.states[last_index];
            let commute = match (&self.moves[last_index], &mv) {
                (Some(last), Some(m)) => state_before.moves_commute(last, m),
                _ => false,
            };
            if !commute {
                break;
            }
            let m = mv.as_deref().expect("commuting moves are known");
            let after_both = self.states[last_index + 1].apply(m)?;
            let after_both = after_both.normalized();
            if *state_before.normalized() == *after_both {
                // The move cancels the last move.
                return Ok(IndexAndMove {
                    index: last_index,
                    mv: None,
                });
            }
            if let Some(alternate) = state_before.canonical_move_to(&after_both) {
                // The move merges with the last move.
                return Ok(IndexAndMove {
                    index: last_index,
                    mv: Some(alternate),
                });
            }
        }
        Ok(IndexAndMove {
            index: self.moves.len(),
            mv,
        })
    }

    fn cost_of(state: &S, mv: Option<&str>) -> i32 {
        mv.map_or(1, |m| state.move_cost(m))
    }

    fn apply_opt(state: &S, mv: Option<&str>) -> Result<S, InvalidMoveError> {
        state.apply(mv.unwrap_or("null"))
    }

    /// Appends a move, merging it according to the builder's [`MergingMode`].
    ///
    /// Like TNoodle, appending a rotation in [`MergingMode::CanonicalizeMoves`] mode removes
    /// the first move (or fails on an empty builder), and after an error the builder is in
    /// an unspecified state.
    pub fn append_move(&mut self, new_move: &str) -> Result<(), InvalidMoveError> {
        let im = self.find_best_index_for_move(new_move, self.merging_mode)?;
        let (old_cost, new_cost);
        if im.index < self.moves.len() {
            // The move is redundant.
            old_cost = Self::cost_of(&self.states[im.index], self.moves[im.index].as_deref());
            match &im.mv {
                None => {
                    // It cancelled perfectly with the move at im.index.
                    self.moves.remove(im.index);
                    self.states.remove(im.index + 1);
                    new_cost = 0;
                }
                Some(m) => {
                    // It merged with the move at im.index.
                    self.moves[im.index] = Some(m.clone());
                    new_cost = Self::cost_of(&self.states[im.index], Some(m));
                }
            }
        } else {
            old_cost = 0;
            let last = self.states.last().expect("there is always a state").clone();
            new_cost = Self::cost_of(&last, im.mv.as_deref());
            self.moves.push(im.mv.clone());
            self.states.push(last);
        }

        self.total_cost += new_cost - old_cost;

        // Everything after the modified move must be recomputed.
        for i in im.index + 1..self.states.len() {
            self.states[i] = Self::apply_opt(&self.states[i - 1], self.moves[i - 1].as_deref())?;
        }

        self.un_normalized_state = self.un_normalized_state.apply(new_move)?;
        Ok(())
    }

    /// Removes the move at `index` and replays the remaining moves; returns the removed move.
    pub fn pop_move(&mut self, index: usize) -> Option<String> {
        let mut moves = self.moves.clone();
        let popped = moves.remove(index);
        self.reset_to_state(self.original_state.clone());
        for m in moves {
            self.append_move(m.as_deref().unwrap_or("null"))
                .expect("replaying moves that were accepted before");
        }
        popped
    }

    /// Appends every move of a whitespace separated algorithm.
    pub fn append_algorithm(&mut self, algorithm: &str) -> Result<(), InvalidMoveError> {
        for mv in split_algorithm(algorithm) {
            self.append_move(mv)?;
        }
        Ok(())
    }

    /// Appends every move of every algorithm.
    pub fn append_algorithms<'a>(
        &mut self,
        algorithms: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), InvalidMoveError> {
        for algorithm in algorithms {
            self.append_algorithm(algorithm)?;
        }
        Ok(())
    }

    /// The current state.
    pub fn state(&self) -> &S {
        self.states.last().expect("there is always a state")
    }

    /// The total cost of the moves.
    pub fn total_cost(&self) -> i32 {
        self.total_cost
    }

    /// The moves; `None` is a rotation that TNoodle recorded as a `null` move.
    pub fn moves(&self) -> &[Option<String>] {
        &self.moves
    }

    /// The current state and the move sequence.
    pub fn state_and_generator(&self) -> PuzzleStateAndGenerator<S> {
        PuzzleStateAndGenerator {
            state: self.state().clone(),
            generator: self.to_string(),
        }
    }
}

impl<S: PuzzleState> fmt::Display for AlgorithmBuilder<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let moves: Vec<&str> = self
            .moves
            .iter()
            .map(|m| m.as_deref().unwrap_or("null"))
            .collect();
        f.write_str(&moves.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::split_algorithm;

    #[test]
    fn splits_like_java() {
        assert_eq!(split_algorithm(""), [] as [&str; 0]);
        assert_eq!(split_algorithm(" \t\n"), [] as [&str; 0]);
        assert_eq!(split_algorithm("\u{1}"), [] as [&str; 0]);
        assert_eq!(split_algorithm("R U"), ["R", "U"]);
        assert_eq!(split_algorithm("R  U\n"), ["R", "U"]);
        assert_eq!(split_algorithm(" R U"), ["", "R", "U"]);
        assert_eq!(split_algorithm("R\u{1}U"), ["R\u{1}U"]);
    }
}
