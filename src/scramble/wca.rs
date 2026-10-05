use super::WcaEvent;
use tnoodle::{PuzzleRegistry, Scrambler};

/// Generates an official random-state scramble with `tnoodle-rs`.
///
/// Like `TNoodle`, it seeds a `SHA1PRNG` from OS entropy for every scramble. A
/// `java.util.Random` would not do: its 48-bit seed cannot reach most states
/// of the larger puzzles.
pub fn get_wca_scramble(event: WcaEvent) -> String {
    event_to_puzzle(event).generate_scramble()
}

/// Maps an event to its `tnoodle-rs` scrambler.
fn event_to_puzzle(event: WcaEvent) -> &'static Scrambler {
    let puzzle = match event {
        WcaEvent::Cube2x2 => PuzzleRegistry::Two,
        WcaEvent::Cube3x3 => PuzzleRegistry::Three,
        WcaEvent::Cube4x4 => PuzzleRegistry::Four,
        WcaEvent::Cube5x5 => PuzzleRegistry::Five,
        WcaEvent::Cube6x6 => PuzzleRegistry::Six,
        WcaEvent::Cube7x7 => PuzzleRegistry::Seven,
        WcaEvent::Megaminx => PuzzleRegistry::Mega,
        WcaEvent::Pyraminx => PuzzleRegistry::Pyra,
        WcaEvent::Fto => PuzzleRegistry::Fto,
        WcaEvent::Skewb => PuzzleRegistry::Skewb,
        WcaEvent::Square1 => PuzzleRegistry::Sq1,
        WcaEvent::Clock => PuzzleRegistry::Clock,
    };
    puzzle.scrambler()
}

#[cfg(test)]
mod tests {
    use super::{WcaEvent, event_to_puzzle, get_wca_scramble};

    const EVENTS: [WcaEvent; 12] = [
        WcaEvent::Cube2x2,
        WcaEvent::Cube3x3,
        WcaEvent::Cube4x4,
        WcaEvent::Cube5x5,
        WcaEvent::Cube6x6,
        WcaEvent::Cube7x7,
        WcaEvent::Megaminx,
        WcaEvent::Pyraminx,
        WcaEvent::Skewb,
        WcaEvent::Square1,
        WcaEvent::Clock,
        WcaEvent::Fto,
    ];

    #[test]
    fn events_map_to_their_wca_puzzles() {
        let keys: Vec<&str> = EVENTS
            .iter()
            .map(|&event| event_to_puzzle(event).short_name())
            .collect();
        assert_eq!(
            keys,
            [
                "222", "333", "444", "555", "666", "777", "minx", "pyram", "skewb", "sq1", "clock",
                "fto"
            ]
        );
    }

    #[test]
    fn generates_a_valid_scramble_for_every_event() {
        for event in EVENTS {
            let scramble = get_wca_scramble(event);
            assert!(!scramble.trim().is_empty(), "{event:?}: empty scramble");
            let puzzle = event_to_puzzle(event);
            // Every WCA scramble parses and leaves the puzzle unsolved.
            let solved = puzzle
                .is_solved_by(&scramble)
                .unwrap_or_else(|error| panic!("{event:?}: {scramble}: {error}"));
            assert!(!solved, "{event:?}: {scramble} solves the puzzle");
        }
    }

    #[test]
    fn scrambles_are_random() {
        for event in [WcaEvent::Cube2x2, WcaEvent::Pyraminx, WcaEvent::Skewb] {
            assert_ne!(
                get_wca_scramble(event),
                get_wca_scramble(event),
                "{event:?}"
            );
        }
    }
}
