//! A faithful Rust port of [TNoodle](https://github.com/thewca/tnoodle-lib), the official
//! World Cube Association scramble program library.
//!
//! The port reproduces the Java library's behaviour exactly: given the same
//! `java.util.Random` seed it generates the same scrambles, solutions and SVG drawings.
//!
//! # Overview
//!
//! * [`scrambles`]: the puzzle framework ([`Puzzle`], [`PuzzleState`]), move merging,
//!   solving, and the [`PuzzleRegistry`] of WCA puzzles.
//! * [`puzzle`]: the puzzles themselves.
//! * [`min2phase`], [`threephase`], [`sq12phase`], [`fto3phase`]: the random-state solvers
//!   for the 3x3x3, 4x4x4, Square-1 and FTO.
//! * [`svg`]: the SVG drawing library.
//! * [`analysis`]: statistical checks of scramble quality.
//! * [`java`]: the emulated Java platform behaviour that makes the output bit-exact.
//!
//! ```
//! use tnoodle::java::JavaRandom;
//! use tnoodle::PuzzleRegistry;
//!
//! let skewb = PuzzleRegistry::Skewb.scrambler();
//! let scramble = skewb.generate_wca_scramble(&mut JavaRandom::new(1));
//! assert_eq!(scramble.split(' ').count(), 11);
//! let svg = skewb.draw_scramble(Some(&scramble), None).unwrap();
//! assert!(svg.to_string().starts_with("<svg"));
//! ```

// The solvers are index-heavy ports of Java code; indexing loops mirror the original
// algorithms and are easier to compare with it than iterator chains.
#![allow(clippy::needless_range_loop)]
// Puzzle names are string literals returned through `&self` trait methods.
#![allow(clippy::unnecessary_literal_bound)]

pub mod analysis;
pub mod error;
pub mod fto3phase;
pub mod java;
pub mod min2phase;
pub mod puzzle;
pub mod scrambles;
pub mod sq12phase;
pub mod svg;
pub mod threephase;

pub use error::{InvalidHexColorError, InvalidMoveError, InvalidScrambleError};
pub use scrambles::{Puzzle, PuzzleRegistry, PuzzleState, Scrambler};
