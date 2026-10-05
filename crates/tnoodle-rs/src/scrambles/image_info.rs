//! The colours and size of a puzzle's drawing (`PuzzleImageInfo`).

use std::fmt::Write as _;

use super::puzzle::Puzzle;
use crate::java::JavaHashMap;
use crate::svg::{Color, Dimension};

/// The default colour scheme and preferred size of a puzzle, for clients that draw puzzles
/// themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleImageInfo {
    /// The colour of each face, in the iteration order of TNoodle's scheme map.
    pub color_scheme: Vec<(String, Color)>,
    /// The preferred drawing size.
    pub size: Dimension,
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c < ' ' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl PuzzleImageInfo {
    /// The image info of a puzzle.
    pub fn new<P: Puzzle + ?Sized>(puzzle: &P) -> Self {
        // TNoodle copies a static map (filled in source order) with `new HashMap<>(map)`.
        let defaults: JavaHashMap<String, Color> = puzzle
            .default_color_scheme_entries()
            .iter()
            .map(|&(face, color)| (face.to_owned(), color))
            .collect();
        let copy = JavaHashMap::copy_of(&defaults);
        Self {
            color_scheme: copy.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            size: puzzle.preferred_size(),
        }
    }

    /// The JSON form (`toJsonable`), with keys in TNoodle's `HashMap` order:
    /// `{"size":{"width":..,"height":..},"colorScheme":{"U":"ffffff",..}}`.
    pub fn to_json(&self) -> String {
        let dim: JavaHashMap<String, String> = [
            ("width".to_owned(), self.size.width.to_string()),
            ("height".to_owned(), self.size.height.to_string()),
        ]
        .into_iter()
        .collect();
        let colors: JavaHashMap<String, String> = self
            .color_scheme
            .iter()
            .map(|(face, color)| (face.clone(), json_string(&color.to_hex())))
            .collect();
        let object = |map: &JavaHashMap<String, String>| {
            let entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}:{v}", json_string(k)))
                .collect();
            format!("{{{}}}", entries.join(","))
        };
        let top: JavaHashMap<String, String> = [
            ("size".to_owned(), object(&dim)),
            ("colorScheme".to_owned(), object(&colors)),
        ]
        .into_iter()
        .collect();
        object(&top)
    }
}

#[cfg(test)]
mod tests {
    use super::json_string;

    #[test]
    fn escapes_json() {
        assert_eq!(json_string("a\"b\\c\n"), r#""a\"b\\c\u000a""#);
    }
}
