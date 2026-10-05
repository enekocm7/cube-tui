//! Event-specific simulators produce a shared, terminal-independent ASCII grid.
//! Add new puzzle simulators here; the preview widget only consumes this grid.

mod cube;

use std::fmt;

use super::WcaEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickerColor {
    White,
    Orange,
    Green,
    Red,
    Blue,
    Yellow,
}

impl StickerColor {
    pub const fn symbol(self) -> char {
        match self {
            Self::White => 'W',
            Self::Orange => 'O',
            Self::Green => 'G',
            Self::Red => 'R',
            Self::Blue => 'B',
            Self::Yellow => 'Y',
        }
    }

    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Self::White => [240, 240, 240],
            Self::Orange => [255, 153, 51],
            Self::Green => [51, 204, 102],
            Self::Red => [255, 85, 85],
            Self::Blue => [85, 153, 255],
            Self::Yellow => [255, 221, 51],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub symbol: char,
    pub color: Option<StickerColor>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: ' ',
            color: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Visualization {
    pub cells: Vec<Vec<Cell>>,
}

impl Visualization {
    pub(super) fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![Cell::default(); width]; height],
        }
    }

    pub fn width(&self) -> u16 {
        self.cells.iter().map(Vec::len).max().unwrap_or(0) as u16
    }

    pub fn height(&self) -> u16 {
        self.cells.len() as u16
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VisualizationError {
    InvalidMove(String),
    Unavailable,
}

impl fmt::Display for VisualizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMove(token) => write!(f, "Invalid move: {token}"),
            Self::Unavailable => f.write_str("Preview is not available for this puzzle"),
        }
    }
}

/// Applies the complete scramble, rejecting unsupported notation rather than
/// displaying a partially scrambled puzzle that could mislead the solver.
pub fn visualize(event: WcaEvent, scramble: &str) -> Result<Visualization, VisualizationError> {
    match event {
        WcaEvent::Cube2x2 => cube::visualize_size(2, scramble),
        WcaEvent::Cube3x3 => cube::visualize_size(3, scramble),
        WcaEvent::Cube4x4 => cube::visualize_size(4, scramble),
        WcaEvent::Cube5x5 => cube::visualize_size(5, scramble),
        WcaEvent::Cube6x6 => cube::visualize_size(6, scramble),
        WcaEvent::Cube7x7 => cube::visualize_size(7, scramble),
        WcaEvent::Pyraminx
        | WcaEvent::Skewb
        | WcaEvent::Megaminx
        | WcaEvent::Fto
        | WcaEvent::Square1
        | WcaEvent::Clock => Err(VisualizationError::Unavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUBES: [WcaEvent; 6] = [
        WcaEvent::Cube2x2,
        WcaEvent::Cube3x3,
        WcaEvent::Cube4x4,
        WcaEvent::Cube5x5,
        WcaEvent::Cube6x6,
        WcaEvent::Cube7x7,
    ];

    #[test]
    fn only_cubic_events_have_a_preview() {
        for event in WcaEvent::ALL {
            if CUBES.contains(&event) {
                let net = visualize(event, "").unwrap();
                assert!(net.width() > 0 && net.height() > 0, "{event:?}");
            } else {
                assert_eq!(visualize(event, ""), Err(VisualizationError::Unavailable));
                assert_eq!(
                    visualize(event, "invalid"),
                    Err(VisualizationError::Unavailable)
                );
            }
        }
    }

    #[test]
    fn built_in_cube_scrambles_can_be_visualized() {
        for event in CUBES {
            for _ in 0..10 {
                let scramble = crate::scramble::random_scramble(event);
                visualize(event, &scramble)
                    .unwrap_or_else(|error| panic!("{event:?}: {scramble}: {error}"));
            }
        }
    }

    #[cfg(feature = "wca-scrambles")]
    #[test]
    fn official_cube_scrambles_can_be_visualized() {
        for event in CUBES {
            let scramble = crate::scramble::wca::get_wca_scramble(event);
            visualize(event, &scramble)
                .unwrap_or_else(|error| panic!("{event:?}: {scramble}: {error}"));
        }
    }
}
