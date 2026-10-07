//! A Square-1 layer is a ring of twelve 30-degree units. Corners occupy two
//! units, so a slice is legal only when both cuts fall between whole pieces.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(not(feature = "wca-scrambles"))]
pub struct Unit {
    pub piece: u8,
    pub upper_color: bool,
    pub side: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(not(feature = "wca-scrambles"))]
pub struct Square1State {
    pub layers: [[Unit; 12]; 2],
    pub middle_flipped: bool,
}

#[cfg(not(feature = "wca-scrambles"))]
impl Square1State {
    pub fn new() -> Self {
        let upper = [0, 0, 1, 2, 2, 3, 4, 4, 5, 6, 6, 7];
        let lower = [8, 9, 9, 10, 11, 11, 12, 13, 13, 14, 15, 15];
        let upper_sides = [0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 0, 0];
        let lower_sides = [1, 1, 0, 0, 0, 3, 3, 3, 2, 2, 2, 1];
        Self {
            layers: [
                std::array::from_fn(|i| Unit {
                    piece: upper[i],
                    upper_color: true,
                    side: upper_sides[i],
                }),
                std::array::from_fn(|i| Unit {
                    piece: lower[i],
                    upper_color: false,
                    side: lower_sides[i],
                }),
            ],
            middle_flipped: false,
        }
    }

    pub fn rotate(&mut self, upper: i32, lower: i32) {
        self.layers[0].rotate_right(upper.rem_euclid(12) as usize);
        self.layers[1].rotate_right(lower.rem_euclid(12) as usize);
    }

    pub fn can_slice(&self) -> bool {
        self.layers
            .iter()
            .all(|ring| ring[11].piece != ring[0].piece && ring[5].piece != ring[6].piece)
    }

    pub fn slice(&mut self) -> bool {
        if !self.can_slice() {
            return false;
        }
        for offset in 0..6 {
            let top = self.layers[0][6 + offset];
            self.layers[0][6 + offset] = self.layers[1][offset];
            self.layers[1][offset] = top;
        }
        self.middle_flipped = !self.middle_flipped;
        true
    }
}

#[cfg(test)]
#[cfg(not(feature = "wca-scrambles"))]
mod tests {
    use super::*;

    #[test]
    fn slicing_twice_restores_every_piece_and_the_middle() {
        let mut state = Square1State::new();
        let solved = state.clone();
        assert!(state.slice());
        assert!(state.middle_flipped);
        assert_eq!(
            state.layers[0].map(|unit| unit.piece),
            [0, 0, 1, 2, 2, 3, 8, 9, 9, 10, 11, 11]
        );
        assert!(state.slice());
        assert_eq!(state, solved);
    }

    #[test]
    fn cuts_through_corners_are_rejected_without_mutation() {
        let mut state = Square1State::new();
        state.rotate(-1, 0);
        let before = state.clone();
        assert!(!state.can_slice());
        assert!(!state.slice());
        assert_eq!(state, before);
    }
}
