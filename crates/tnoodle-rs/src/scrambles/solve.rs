//! The generic bidirectional breadth first solver (`Puzzle.solveIn`).

use std::collections::{BTreeMap, HashMap};

use super::algorithm_builder::{AlgorithmBuilder, MergingMode};
use super::puzzle::PuzzleState;

/// Elements grouped by an integer priority; pops the most recently pushed element of the
/// lowest priority (`Puzzle.SortedBuckets`).
#[derive(Debug)]
struct SortedBuckets<T> {
    buckets: BTreeMap<i32, Vec<T>>,
}

impl<T> SortedBuckets<T> {
    fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
        }
    }

    fn add(&mut self, element: T, value: i32) {
        self.buckets.entry(value).or_default().push(element);
    }

    fn smallest_value(&self) -> Option<i32> {
        self.buckets.keys().next().copied()
    }

    fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }

    fn pop(&mut self) -> Option<T> {
        let mut entry = self.buckets.first_entry()?;
        let element = entry.get_mut().pop();
        if entry.get().is_empty() {
            entry.remove();
        }
        element
    }
}

/// The search from the solved state.
const SOLVED: usize = 0;
/// The search from the scrambled state.
const SCRAMBLED: usize = 1;

/// Searches from both the solved and the scrambled state until the two frontiers meet,
/// returning a canonical solution of cost at most `n`.
pub(crate) fn solve_in_bfs<S: PuzzleState>(ps: &S, n: i32) -> Option<String> {
    if ps.is_solved() {
        return Some(String::new());
    }

    let mut seen: [HashMap<S, i32>; 2] = [HashMap::new(), HashMap::new()];
    let mut fringe: [SortedBuckets<S>; 2] = [SortedBuckets::new(), SortedBuckets::new()];

    // We're only interested in solutions of cost <= n.
    let mut best_intersection_cost = n + 1;
    let mut best_intersection: Option<S> = None;

    let solved_normalized = ps.solved().normalized().into_owned();
    fringe[SOLVED].add(solved_normalized.clone(), 0);
    seen[SOLVED].insert(solved_normalized, 0);
    let scrambled_normalized = ps.normalized().into_owned();
    fringe[SCRAMBLED].add(scrambled_normalized.clone(), 0);
    seen[SCRAMBLED].insert(scrambled_normalized, 0);

    let mut fringe_ties = 0;
    let mut min_fringe = [-1; 2];
    while !fringe[SOLVED].is_empty() || !fringe[SCRAMBLED].is_empty() {
        // Extend the non empty fringe whose nearest node is closest to its origin,
        // alternating on ties.
        for side in [SCRAMBLED, SOLVED] {
            if let Some(v) = fringe[side].smallest_value() {
                min_fringe[side] = v;
            }
        }
        let extend_solved = if fringe[SOLVED].is_empty() || fringe[SCRAMBLED].is_empty() {
            !fringe[SOLVED].is_empty()
        } else if min_fringe[SOLVED] != min_fringe[SCRAMBLED] {
            min_fringe[SOLVED] < min_fringe[SCRAMBLED]
        } else {
            fringe_ties += 1;
            (fringe_ties - 1) % 2 == 0
        };
        let (ext, cmp) = if extend_solved {
            (SOLVED, SCRAMBLED)
        } else {
            (SCRAMBLED, SOLVED)
        };

        let node = fringe[ext].pop().expect("the extended fringe is not empty");
        let distance = seen[ext][&node];
        if let Some(&other) = seen[cmp].get(&node) {
            // An intersection: the cost of the path through this node.
            let cost = other + distance;
            if cost < best_intersection_cost {
                best_intersection = Some(node);
                best_intersection_cost = cost;
            }
            continue;
        }
        // The best solution through this node goes through a child of it that reaches the
        // other fringe's closest node.
        if distance + min_fringe[cmp] >= best_intersection_cost {
            continue;
        }
        if distance >= (n + 1) / 2 {
            // If n is odd one side searches n/2 and the other n/2 + 1; as we don't know
            // which is which, both search (n+1)/2.
            continue;
        }

        for (next, mv) in node.canonical_moves_by_state() {
            let next_distance = distance + node.move_cost(&mv);
            let next = next.normalized().into_owned();
            if seen[ext].get(&next).is_some_and(|&d| next_distance >= d) {
                // We already found a better path to next.
                continue;
            }
            fringe[ext].add(next.clone(), next_distance);
            seen[ext].insert(next, next_distance);
        }
    }

    let best_intersection = best_intersection?;
    let [seen_solved, seen_scrambled] = seen;

    // We found the bound between both searches; recover the moves:
    //   solved <----- best_intersection <----- scrambled

    // Step 1: walk from best_intersection back towards the scramble.
    let mut state = best_intersection.clone();
    let mut distance_from_scrambled = seen_scrambled[&state];
    let mut linked_states: Vec<Option<S>> = vec![None; distance_from_scrambled as usize + 1];
    linked_states[distance_from_scrambled as usize] = Some(state.clone());
    'outer: while distance_from_scrambled > 0 {
        for (next, _) in state.canonical_moves_by_state() {
            let next = next.normalized().into_owned();
            if let Some(&d) = seen_scrambled.get(&next)
                && d < distance_from_scrambled
            {
                state = next;
                distance_from_scrambled = d;
                linked_states[d as usize] = Some(state.clone());
                continue 'outer;
            }
        }
        unreachable!("the scrambled side of the search is connected");
    }

    // Step 2: replay from the scramble to best_intersection.
    let mut solution = AlgorithmBuilder::with_state(MergingMode::CanonicalizeMoves, ps.clone());
    let mut state = ps.clone();
    let mut distance_from_scrambled = 0;
    'outer: while !state.equals_normalized(&best_intersection) {
        for (next_state, move_name) in state.canonical_moves_by_state() {
            let target = linked_states
                .get(distance_from_scrambled + 1)
                .and_then(Option::as_ref);
            if target.is_some_and(|t| next_state.equals_normalized(t)) {
                state = next_state;
                solution
                    .append_move(&move_name)
                    .expect("canonical moves are valid");
                distance_from_scrambled = seen_scrambled[&*state.normalized()] as usize;
                continue 'outer;
            }
        }
        unreachable!("the path to the intersection exists");
    }

    // Step 3: walk from best_intersection to the solved state.
    let mut distance_from_solved = seen_solved[&*state.normalized()];
    'outer: while distance_from_solved > 0 {
        for (next_state, move_name) in state.canonical_moves_by_state() {
            let normalized = next_state.normalized().into_owned();
            if let Some(&d) = seen_solved.get(&normalized)
                && d < distance_from_solved
            {
                state = next_state;
                distance_from_solved = d;
                solution
                    .append_move(&move_name)
                    .expect("canonical moves are valid");
                continue 'outer;
            }
        }
        unreachable!("the solved side of the search is connected");
    }

    Some(solution.to_string())
}

#[cfg(test)]
mod tests {
    use super::SortedBuckets;

    #[test]
    fn buckets_pop_lifo_within_lowest_value() {
        let mut b = SortedBuckets::new();
        b.add("a", 2);
        b.add("b", 1);
        b.add("c", 1);
        assert_eq!(b.smallest_value(), Some(1));
        assert_eq!(b.pop(), Some("c"));
        assert_eq!(b.pop(), Some("b"));
        assert_eq!(b.smallest_value(), Some(2));
        assert_eq!(b.pop(), Some("a"));
        assert!(b.is_empty());
        assert_eq!(b.pop(), None);
    }
}
