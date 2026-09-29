use super::{Cell, StickerColor, Visualization, VisualizationError};

// All cube sizes share right-handed coordinates: +x right, +y up, +z front.
// Each face's basis
// describes its rows as seen from outside, including the unfolded back face.
#[derive(Clone, Copy)]
struct Face {
    normal: [i8; 3],
    right: [i8; 3],
    down: [i8; 3],
    color: StickerColor,
    offset: (usize, usize),
}

const FACES: [Face; 6] = [
    Face {
        normal: [0, 1, 0],
        right: [1, 0, 0],
        down: [0, 0, 1],
        color: StickerColor::White,
        offset: (7, 0),
    },
    Face {
        normal: [-1, 0, 0],
        right: [0, 0, 1],
        down: [0, -1, 0],
        color: StickerColor::Orange,
        offset: (0, 3),
    },
    Face {
        normal: [0, 0, 1],
        right: [1, 0, 0],
        down: [0, -1, 0],
        color: StickerColor::Green,
        offset: (7, 3),
    },
    Face {
        normal: [1, 0, 0],
        right: [0, 0, -1],
        down: [0, -1, 0],
        color: StickerColor::Red,
        offset: (14, 3),
    },
    Face {
        normal: [0, 0, -1],
        right: [-1, 0, 0],
        down: [0, -1, 0],
        color: StickerColor::Blue,
        offset: (21, 3),
    },
    Face {
        normal: [0, -1, 0],
        right: [1, 0, 0],
        down: [0, 0, -1],
        color: StickerColor::Yellow,
        offset: (7, 6),
    },
];

#[derive(Clone, Copy)]
struct Sticker {
    position: [i8; 3],
    normal: [i8; 3],
    color: StickerColor,
}

fn dot(a: [i8; 3], b: [i8; 3]) -> i8 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

/// Rotates clockwise when looking at the turning face from outside the cube.
fn rotate([x, y, z]: [i8; 3], axis: usize, side: i8) -> [i8; 3] {
    match axis {
        0 => [x, side * z, -side * y],
        1 => [-side * z, y, side * x],
        2 => [side * y, -side * x, z],
        _ => unreachable!(),
    }
}

pub(super) fn visualize_size(
    size: usize,
    scramble: &str,
) -> Result<Visualization, VisualizationError> {
    let extent = (size - 1) as i8;
    let mut stickers = Vec::with_capacity(6 * size * size);
    for face in FACES {
        for row in 0..size {
            for col in 0..size {
                stickers.push(Sticker {
                    position: std::array::from_fn(|axis| {
                        extent * face.normal[axis]
                            + (2 * col as i8 - extent) * face.right[axis]
                            + (2 * row as i8 - extent) * face.down[axis]
                    }),
                    normal: face.normal,
                    color: face.color,
                });
            }
        }
    }

    for token in scramble.split_whitespace() {
        let Turn {
            axis,
            side,
            first,
            end,
            turns,
        } = parse_move(token, size)?;
        for _ in 0..turns {
            for sticker in &mut stickers {
                let depth = ((extent - sticker.position[axis] * side) / 2) as usize;
                if depth >= first && depth < end {
                    sticker.position = rotate(sticker.position, axis, side);
                    sticker.normal = rotate(sticker.normal, axis, side);
                }
            }
        }
    }

    let stride = if size <= 3 { 2 } else { 1 };
    let face_width = size * stride;
    let mut net = Visualization::new(4 * face_width + 3, 3 * size);
    for face in FACES {
        let x = face.offset.0 / 7 * (face_width + 1);
        let y = face.offset.1 / 3 * size;
        for sticker in &stickers {
            if sticker.normal == face.normal {
                let col = i8::midpoint(dot(sticker.position, face.right), extent) as usize;
                let row = i8::midpoint(dot(sticker.position, face.down), extent) as usize;
                net.cells[y + row][x + stride * col] = Cell {
                    symbol: sticker.color.symbol(),
                    color: Some(sticker.color),
                };
                if stride == 2 {
                    net.cells[y + row][x + stride * col + 1] = Cell {
                        symbol: ' ',
                        color: Some(sticker.color),
                    };
                }
            }
        }
    }
    Ok(net)
}

struct Turn {
    axis: usize,
    side: i8,
    first: usize,
    end: usize,
    turns: usize,
}

fn parse_move(token: &str, size: usize) -> Result<Turn, VisualizationError> {
    let invalid = || VisualizationError::InvalidMove(token.into());
    let (base, turns) =
        if let Some(base) = token.strip_suffix("2'").or_else(|| token.strip_suffix('2')) {
            (base, 2)
        } else if let Some(base) = token.strip_suffix('\'') {
            (base, 3)
        } else {
            (token, 1)
        };
    let prefix_len = base.bytes().take_while(u8::is_ascii_digit).count();
    let count = if prefix_len == 0 {
        None
    } else {
        Some(base[..prefix_len].parse::<usize>().map_err(|_| invalid())?)
    };
    let family = &base[prefix_len..];
    let special = match family {
        "x" => Some((0, 1, 0, size)),
        "y" => Some((1, 1, 0, size)),
        "z" => Some((2, 1, 0, size)),
        "M" if size == 3 => Some((0, -1, 1, 2)),
        "E" if size == 3 => Some((1, -1, 1, 2)),
        "S" if size == 3 => Some((2, 1, 1, 2)),
        _ => None,
    };
    if let Some((axis, side, first, end)) = special {
        if count.is_some() {
            return Err(invalid());
        }
        return Ok(Turn {
            axis,
            side,
            first,
            end,
            turns,
        });
    }
    let wide = family.ends_with('w') || matches!(family, "r" | "l" | "u" | "d" | "f" | "b");
    let face = family.strip_suffix('w').unwrap_or(family);
    let (axis, side) = match face {
        "R" | "r" => (0, 1),
        "L" | "l" => (0, -1),
        "U" | "u" => (1, 1),
        "D" | "d" => (1, -1),
        "F" | "f" => (2, 1),
        "B" | "b" => (2, -1),
        _ => return Err(invalid()),
    };
    let end = if wide { count.unwrap_or(2) } else { 1 };
    if (wide && (end < 2 || end >= size)) || (!wide && count.is_some()) {
        return Err(invalid());
    }
    Ok(Turn {
        axis,
        side,
        first: 0,
        end,
        turns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visualize(scramble: &str) -> Result<Visualization, VisualizationError> {
        visualize_size(3, scramble)
    }

    fn face_strings(scramble: &str) -> Vec<String> {
        let net = visualize(scramble).unwrap();
        FACES
            .iter()
            .map(|face| {
                (0..3)
                    .flat_map(|row| (0..3).map(move |col| (row, col)))
                    .map(|(row, col)| {
                        net.cells[face.offset.1 + row][face.offset.0 + 2 * col].symbol
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn solved_net_has_standard_color_scheme() {
        assert_eq!(
            face_strings(""),
            [
                "WWWWWWWWW",
                "OOOOOOOOO",
                "GGGGGGGGG",
                "RRRRRRRRR",
                "BBBBBBBBB",
                "YYYYYYYYY"
            ]
        );
    }

    // Independent facelet fixtures catch reversed turns and mirrored back rows.
    #[rstest::rstest]
    #[case("R", ["WWGWWGWWG", "OOOOOOOOO", "GGYGGYGGY", "RRRRRRRRR", "WBBWBBWBB", "YYBYYBYYB"])]
    #[case("L", ["BWWBWWBWW", "OOOOOOOOO", "WGGWGGWGG", "RRRRRRRRR", "BBYBBYBBY", "GYYGYYGYY"])]
    #[case("U", ["WWWWWWWWW", "GGGOOOOOO", "RRRGGGGGG", "BBBRRRRRR", "OOOBBBBBB", "YYYYYYYYY"])]
    #[case("D", ["WWWWWWWWW", "OOOOOOBBB", "GGGGGGOOO", "RRRRRRGGG", "BBBBBBRRR", "YYYYYYYYY"])]
    #[case("F", ["WWWWWWOOO", "OOYOOYOOY", "GGGGGGGGG", "WRRWRRWRR", "BBBBBBBBB", "RRRYYYYYY"])]
    #[case("B", ["RRRWWWWWW", "WOOWOOWOO", "GGGGGGGGG", "RRYRRYRRY", "BBBBBBBBB", "YYYYYYOOO"])]
    fn clockwise_turns_match_known_facelets(#[case] scramble: &str, #[case] expected: [&str; 6]) {
        assert_eq!(face_strings(scramble), expected);
    }

    #[test]
    fn moves_and_their_inverses_restore_the_cube() {
        let solved = visualize("").unwrap();
        for face in ["U", "D", "L", "R", "F", "B"] {
            assert_eq!(visualize(&format!("{face} {face}'")).unwrap(), solved);
            assert_eq!(
                visualize(&format!("{face} {face} {face} {face}")).unwrap(),
                solved
            );
            assert_eq!(visualize(&format!("{face}2 {face}2")).unwrap(), solved);
            assert_eq!(
                visualize(&format!("{face}2")).unwrap(),
                visualize(&format!("{face} {face}")).unwrap()
            );
        }
        assert_eq!(
            visualize("R U2 F' L D B2 B2 D' L' F U2 R'").unwrap(),
            solved
        );
    }

    #[test]
    fn repeated_sexy_move_restores_the_cube() {
        assert_eq!(
            visualize(&"R U R' U' ".repeat(6)).unwrap(),
            visualize("").unwrap()
        );
        assert_ne!(visualize("R U R' U'").unwrap(), visualize("").unwrap());
    }

    #[test]
    fn invalid_tokens_never_produce_a_partial_preview() {
        for token in ["3Rw", "1Rw", "2R", "R3", "R''", "xx", "💥", "R💥"] {
            assert_eq!(
                visualize(&format!("U {token}")),
                Err(VisualizationError::InvalidMove(token.into()))
            );
        }
    }

    #[test]
    fn scrambled_net_preserves_stickers_and_centers() {
        let net = visualize("R U2 F' D B2 L' U R2 F D'").unwrap();
        for face in FACES {
            assert_eq!(
                net.cells
                    .iter()
                    .flatten()
                    .filter(|cell| cell.color == Some(face.color) && cell.symbol != ' ')
                    .count(),
                9
            );
            assert_eq!(
                net.cells[face.offset.1 + 1][face.offset.0 + 2].color,
                Some(face.color)
            );
        }
    }

    #[test]
    fn every_cube_size_preserves_stickers_and_supports_all_face_turns() {
        for size in 2..=7 {
            let solved = visualize_size(size, "").unwrap();
            let scrambled = visualize_size(size, "R U2 F' L D B2").unwrap();
            for face in FACES {
                assert_eq!(
                    scrambled
                        .cells
                        .iter()
                        .flatten()
                        .filter(|cell| cell.color == Some(face.color) && cell.symbol != ' ')
                        .count(),
                    size * size
                );
            }
            for face in ["R", "L", "U", "D", "F", "B"] {
                assert_eq!(
                    visualize_size(size, &format!("{face} {face}'")).unwrap(),
                    solved
                );
                assert_eq!(
                    visualize_size(size, &format!("{face} {face} {face} {face}")).unwrap(),
                    solved
                );
            }
            assert_ne!(scrambled, solved);
        }
    }

    #[test]
    fn wide_turns_move_exactly_the_requested_layers() {
        for (size, layers, token) in [(4, 2, "Rw"), (5, 2, "Rw"), (6, 3, "3Rw"), (7, 3, "3Rw")] {
            let net = visualize_size(size, token).unwrap();
            let face_start = size + 1;
            for row in net.cells.iter().take(size) {
                for col in 0..size {
                    assert_eq!(
                        row[face_start + col].color,
                        Some(if col >= size - layers {
                            StickerColor::Green
                        } else {
                            StickerColor::White
                        })
                    );
                }
            }
            assert_eq!(
                visualize_size(size, &format!("{token} {token}'")).unwrap(),
                visualize_size(size, "").unwrap()
            );
        }
        assert_eq!(
            visualize_size(3, "r").unwrap(),
            visualize_size(3, "Rw").unwrap()
        );
        assert_eq!(
            visualize_size(3, "Rw").unwrap(),
            visualize_size(3, "R M'").unwrap()
        );
        assert_eq!(
            visualize_size(7, "3Rw 4Lw'").unwrap(),
            visualize_size(7, "x").unwrap()
        );
        assert_eq!(
            visualize_size(4, "2Rw").unwrap(),
            visualize_size(4, "Rw").unwrap()
        );
        assert!(visualize_size(2, "Rw").is_err());
        assert!(visualize_size(4, "4Rw").is_err());
        assert!(visualize_size(7, "0Rw").is_err());
    }
}
