//! Error types, mirroring TNoodle's checked exceptions.

use thiserror::Error;

/// A move that the puzzle does not recognise (`InvalidMoveException`).
///
/// The message matches Java's: `Invalid move: Unrecognized turn <move>`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid move: {message}")]
pub struct InvalidMoveError {
    message: String,
}

impl InvalidMoveError {
    /// The error raised when `turn` is not one of a state's successors.
    pub fn unrecognized(turn: &str) -> Self {
        Self {
            message: format!("Unrecognized turn {turn}"),
        }
    }

    /// The detail message, without the `Invalid move: ` prefix.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A scramble containing an invalid move (`InvalidScrambleException`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid scramble: {scramble}")]
pub struct InvalidScrambleError {
    scramble: String,
    #[source]
    source: InvalidMoveError,
}

impl InvalidScrambleError {
    /// Wraps the move error that made `scramble` invalid.
    pub fn new(scramble: impl Into<String>, source: InvalidMoveError) -> Self {
        Self {
            scramble: scramble.into(),
            source,
        }
    }

    /// The offending scramble.
    pub fn scramble(&self) -> &str {
        &self.scramble
    }

    /// The underlying invalid move.
    pub fn move_error(&self) -> &InvalidMoveError {
        &self.source
    }
}

/// A colour that is not a valid HTML hex colour (`InvalidHexColorException`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct InvalidHexColorError(pub String);

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn messages_match_java() {
        let m = InvalidMoveError::unrecognized("Q");
        assert_eq!(m.to_string(), "Invalid move: Unrecognized turn Q");
        assert_eq!(m.message(), "Unrecognized turn Q");
        let s = InvalidScrambleError::new("R Q", m.clone());
        assert_eq!(s.to_string(), "Invalid scramble: R Q");
        assert_eq!(s.scramble(), "R Q");
        assert_eq!(s.move_error(), &m);
        assert_eq!(s.source().unwrap().to_string(), m.to_string());
        assert_eq!(InvalidHexColorError("zz".into()).to_string(), "zz");
    }
}
