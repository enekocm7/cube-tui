use chrono::DateTime;

use crate::scramble::WcaEvent;

/// Parses seconds, M:SS, or H:MM:SS with up to three fractional digits.
/// Integer arithmetic avoids rounding away milliseconds or accepting NaN.
pub fn parse_time(text: &str) -> Result<u64, String> {
    let invalid = || {
        "Use seconds, M:SS or H:MM:SS, with up to 3 decimal places (less than 24 hours).".to_owned()
    };
    let parts: Vec<_> = text.trim().split(':').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err(invalid());
    }
    let (seconds, fraction) = parts[parts.len() - 1]
        .split_once('.')
        .unwrap_or((parts[parts.len() - 1], ""));
    if seconds.is_empty()
        || !seconds.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 3
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || text.trim().ends_with('.')
    {
        return Err(invalid());
    }
    let mut total: u64 = 0;
    for (index, part) in parts[..parts.len() - 1]
        .iter()
        .chain(std::iter::once(&seconds))
        .enumerate()
    {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        let number = part.parse::<u64>().map_err(|_| invalid())?;
        if index > 0 && number >= 60 {
            return Err(invalid());
        }
        total = total
            .checked_mul(60)
            .and_then(|n| n.checked_add(number))
            .ok_or_else(invalid)?;
    }
    let millis = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().map_err(|_| invalid())? * 10_u64.pow(3 - fraction.len() as u32)
    };
    total
        .checked_mul(1000)
        .and_then(|n| n.checked_add(millis))
        .filter(|&n| n < 86_400_000)
        .ok_or_else(invalid)
}

/// Empty means the original solve date is unknown. Explicit dates need a zone.
pub fn parse_date(text: &str) -> Result<u64, String> {
    if text.trim().is_empty() {
        return Ok(0);
    }
    DateTime::parse_from_rfc3339(text.trim()).ok()
        .filter(|date| date.timestamp_subsec_nanos() % 1_000_000 == 0)
        .and_then(|date| u64::try_from(date.timestamp_millis()).ok())
        .filter(|&date| date > 0)
        .ok_or_else(|| "Use an RFC 3339 date after 1970, with a timezone and millisecond precision, or leave blank for unknown.".to_owned())
}

pub fn validate_comment(text: &str) -> Result<(), String> {
    if text.chars().count() > 4096
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        Err(
            "Comment must be at most 4096 characters, without terminal control characters."
                .to_owned(),
        )
    } else {
        Ok(())
    }
}

/// Checks event-specific notation, not whether a puzzle state is reachable.
pub fn validate_scramble(text: &str, event: WcaEvent) -> Result<(), String> {
    if text.trim().is_empty() || text.len() > 16_384 {
        return Err("Scramble is required and must be at most 16384 bytes.".to_owned());
    }
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err("Scramble contains control characters.".to_owned());
    }
    if event == WcaEvent::Square1 {
        return if square_one(text) {
            Ok(())
        } else {
            Err("Square-1 requires turn pairs (-5..6, -5..6) separated by / slices.".to_owned())
        };
    }
    for token in text.split_whitespace() {
        if !valid_move(token, event) {
            return Err(format!("Invalid {} move: {token}", event.name()));
        }
    }
    Ok(())
}

fn valid_move(token: &str, event: WcaEvent) -> bool {
    let base = token
        .strip_suffix('\'')
        .or_else(|| token.strip_suffix('2'))
        .unwrap_or(token);
    match event {
        WcaEvent::Cube2x2
        | WcaEvent::Cube3x3
        | WcaEvent::Cube4x4
        | WcaEvent::Cube5x5
        | WcaEvent::Cube6x6
        | WcaEvent::Cube7x7 => {
            let size = match event {
                WcaEvent::Cube2x2 => 2,
                WcaEvent::Cube3x3 => 3,
                WcaEvent::Cube4x4 => 4,
                WcaEvent::Cube5x5 => 5,
                WcaEvent::Cube6x6 => 6,
                _ => 7,
            };
            if matches!(base, "R" | "L" | "U" | "D" | "F" | "B" | "x" | "y" | "z") {
                return true;
            }
            if matches!(base, "M" | "E" | "S") {
                return size >= 3;
            }
            if matches!(base, "r" | "l" | "u" | "d" | "f" | "b") {
                return size >= 3;
            }
            let Some(wide) = base.strip_suffix('w') else {
                return false;
            };
            let (layers, face) = if wide.starts_with(|c: char| c.is_ascii_digit()) {
                let split = wide
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(wide.len());
                let Ok(layers) = wide[..split].parse::<u8>() else {
                    return false;
                };
                (layers, &wide[split..])
            } else {
                (2, wide)
            };
            layers >= 2 && layers < size && matches!(face, "R" | "L" | "U" | "D" | "F" | "B")
        }
        WcaEvent::Megaminx => matches!(token, "R++" | "R--" | "D++" | "D--" | "U" | "U'"),
        WcaEvent::Pyraminx => {
            !token.ends_with('2') && matches!(base, "R" | "L" | "U" | "B" | "r" | "l" | "u" | "b")
        }
        WcaEvent::Skewb => !token.ends_with('2') && matches!(base, "R" | "L" | "U" | "B"),
        WcaEvent::Fto => {
            !token.ends_with('2')
                && matches!(
                    base,
                    "R" | "L" | "U" | "D" | "F" | "B" | "BR" | "BL" | "Br" | "Bl"
                )
        }
        WcaEvent::Clock => clock_move(token),
        WcaEvent::Square1 => false,
    }
}

fn clock_move(token: &str) -> bool {
    if matches!(token, "y2" | "UR" | "DR" | "DL" | "UL") {
        return true;
    }
    ["ALL", "UR", "DR", "DL", "UL", "U", "R", "D", "L"]
        .iter()
        .any(|prefix| {
            let Some(turn) = token.strip_prefix(prefix) else {
                return false;
            };
            // Support WCA's UR3+ and the built-in generator's UR+3 notation.
            let digits = turn
                .strip_suffix(['+', '-'])
                .or_else(|| turn.strip_prefix(['+', '-']));
            digits.is_some_and(|digits| {
                digits.len() == 1 && matches!(digits.as_bytes()[0], b'0'..=b'6')
            })
        })
}

fn square_one(text: &str) -> bool {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut rest = compact.as_str();
    let mut previous_pair = false;
    let mut previous_slice = false;
    while !rest.is_empty() {
        if let Some(next) = rest.strip_prefix('/') {
            if previous_slice {
                return false;
            }
            previous_pair = false;
            previous_slice = true;
            rest = next;
        } else if let Some(pair) = rest.strip_prefix('(') {
            if previous_pair {
                return false;
            }
            let Some((numbers, next)) = pair.split_once(')') else {
                return false;
            };
            let Some((a, b)) = numbers.split_once(',') else {
                return false;
            };
            if ![a, b]
                .iter()
                .all(|n| n.parse::<i8>().is_ok_and(|n| (-5..=6).contains(&n)))
            {
                return false;
            }
            rest = next;
            previous_pair = true;
            previous_slice = false;
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_preserve_milliseconds_and_reject_malformed_or_overflowing_values() {
        for (text, expected) in [
            ("12", 12_000),
            ("0.001", 1),
            (" 1:02.34 ", 62_340),
            ("1:02:03.456", 3_723_456),
            ("0", 0),
        ] {
            assert_eq!(parse_time(text).unwrap(), expected, "{text}");
        }
        for text in [
            "",
            "-1",
            "+1",
            "NaN",
            "inf",
            "1e3",
            "1:60",
            "1:60:00",
            "1:2:3:4",
            "12.",
            ".5",
            "1.0001",
            "1..2",
            "24:00:00",
            "18446744073709551616",
            "9999999999999999:00",
        ] {
            assert!(parse_time(text).is_err(), "{text}");
        }
    }

    #[test]
    fn dates_require_valid_calendar_values_timezones_and_supported_precision() {
        assert_eq!(parse_date("").unwrap(), 0);
        assert_eq!(
            parse_date("2026-09-29T14:30:00.123+02:00").unwrap(),
            parse_date("2026-09-29T12:30:00.123Z").unwrap()
        );
        for text in [
            "yesterday",
            "2026-02-30T12:00:00Z",
            "2026-09-29T12:00:00",
            "1960-01-01T00:00:00Z",
            "2026-09-29T12:00:00.1234Z",
        ] {
            assert!(parse_date(text).is_err(), "{text}");
        }
        assert!(validate_comment(&"界".repeat(4096)).is_ok());
        assert!(validate_comment(&"界".repeat(4097)).is_err());
        assert!(validate_comment("control\u{1b}").is_err());
        assert!(validate_comment("").is_ok());
    }

    #[test]
    fn generated_scrambles_pass_validation_for_every_supported_event() {
        for event in WcaEvent::ALL {
            for _ in 0..5 {
                let scramble = crate::scramble::generate_scramble(event);
                assert!(
                    validate_scramble(scramble.as_str(), event).is_ok(),
                    "{}: {}",
                    event.name(),
                    scramble.as_str()
                );
            }
        }
    }

    #[test]
    fn notation_validation_is_event_specific() {
        let valid = [
            (WcaEvent::Cube3x3, "R U2 F'"),
            (WcaEvent::Cube4x4, "Rw U2 Fw'"),
            (WcaEvent::Cube7x7, "3Rw 3Uw2 B'"),
            (WcaEvent::Megaminx, "R++ D-- U'\nR-- D++ U"),
            (WcaEvent::Pyraminx, "R U' l b'"),
            (WcaEvent::Fto, "BR BL' U Br'"),
            (WcaEvent::Skewb, "R U B'"),
            (WcaEvent::Square1, "/ (1, -3) / (0,6)"),
            (WcaEvent::Clock, "UR3+ DR2- y2 ALL0+ UR"),
            (WcaEvent::Clock, "UR+3 DR-2 y2 ALL+0"),
        ];
        for (event, scramble) in valid {
            assert!(validate_scramble(scramble, event).is_ok(), "{scramble}");
        }
        let invalid = [
            (WcaEvent::Cube3x3, "R banana"),
            (WcaEvent::Cube3x3, "3Rw"),
            (WcaEvent::Cube4x4, "4Rw"),
            (WcaEvent::Cube3x3, "R22"),
            (WcaEvent::Cube3x3, "R\u{1b}"),
            (WcaEvent::Megaminx, "R+ D--"),
            (WcaEvent::Pyraminx, "R2"),
            (WcaEvent::Skewb, "l"),
            (WcaEvent::Square1, "(7,0) /"),
            (WcaEvent::Square1, "(1,0) (0,1)"),
            (WcaEvent::Square1, "(1,x) /"),
            (WcaEvent::Clock, "UR12+"),
            (WcaEvent::Clock, "R U F2"),
        ];
        for (event, scramble) in invalid {
            assert!(validate_scramble(scramble, event).is_err(), "{scramble}");
        }
        assert!(validate_scramble("   ", WcaEvent::Cube3x3).is_err());
        assert!(validate_scramble(&"R ".repeat(9000), WcaEvent::Cube3x3).is_err());
    }
}
