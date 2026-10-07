use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

/// A block-letter font: glyph rows for each supported character.
struct Font {
    height: u16,
    glyph: fn(char) -> Option<&'static [&'static str]>,
}

const LARGE: Font = Font {
    height: 4,
    glyph: large_glyph,
};

const SMALL: Font = Font {
    height: 3,
    glyph: small_glyph,
};

/// Columns left blank between adjacent glyphs.
const GLYPH_GAP: u16 = 1;

fn large_glyph(c: char) -> Option<&'static [&'static str]> {
    Some(match c {
        '0' => &["██▀██", "██ ██", "██ ██", "▀▀▀▀▀"],
        '1' => &[" ▀██ ", "  ██ ", "  ██ ", "▀▀▀▀▀"],
        '2' => &["▀▀▀██", "▄▄▄██", "██   ", "▀▀▀▀▀"],
        '3' => &["▀▀▀██", " ▄▄██", "   ██", "▀▀▀▀▀"],
        '4' => &["██ ██", "██▄██", "   ██", "   ▀▀"],
        '5' => &["██▀▀▀", "██▄▄▄", "   ██", "▀▀▀▀▀"],
        '6' => &["██▀▀▀", "██▄▄▄", "██ ██", "▀▀▀▀▀"],
        '7' => &["▀▀▀██", "   ██", "   ██", "   ▀▀"],
        '8' => &["██▀██", "██▄██", "██ ██", "▀▀▀▀▀"],
        '9' => &["██▀██", "██▄██", "   ██", "▀▀▀▀▀"],
        ':' => &["  ", "▀▀", "▀▀", "  "],
        '.' => &["  ", "  ", "  ", "▀▀"],
        _ => return None,
    })
}

fn small_glyph(c: char) -> Option<&'static [&'static str]> {
    Some(match c {
        '0' => &["█▀█", "█ █", "▀▀▀"],
        '1' => &["▀█ ", " █ ", "▀▀▀"],
        '2' => &["▀▀█", "█▀▀", "▀▀▀"],
        '3' => &["▀▀█", " ▀█", "▀▀▀"],
        '4' => &["█ █", "▀▀█", "  ▀"],
        '5' => &["█▀▀", "▀▀█", "▀▀▀"],
        '6' => &["█▀▀", "█▀█", "▀▀▀"],
        '7' => &["▀▀█", "  █", "  ▀"],
        '8' => &["█▀█", "█▀█", "▀▀▀"],
        '9' => &["█▀█", "▀▀█", "▀▀▀"],
        ':' => &["▄", " ", "▀"],
        '.' => &[" ", " ", "▀"],
        _ => return None,
    })
}

impl Font {
    /// Returns the glyphs for `text`, or `None` if any character is unsupported.
    fn glyphs(&self, text: &str) -> Option<Vec<&'static [&'static str]>> {
        text.chars().map(self.glyph).collect()
    }

    fn width(glyphs: &[&[&str]]) -> u16 {
        let glyph_widths: usize = glyphs.iter().map(|g| g[0].chars().count()).sum();
        let gaps = glyphs.len().saturating_sub(1) * usize::from(GLYPH_GAP);
        u16::try_from(glyph_widths + gaps).unwrap_or(u16::MAX)
    }
}

/// Drops leading zero minutes so `00:07.123` reads as `7.123` and
/// `01:02.345` as `1:02.345`.
fn compact_time(text: &str) -> &str {
    let text = text.strip_prefix("00:").unwrap_or(text);
    match text.strip_prefix('0') {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_digit()) => rest,
        _ => text,
    }
}

/// Renders a time in large block digits, centered in its area, with an
/// optional caption above it. Falls back to smaller digits, then to plain
/// text, when the area is too small.
pub struct BigTimerWidget<'a> {
    time: &'a str,
    label: Option<&'a str>,
    style: Style,
}

impl<'a> BigTimerWidget<'a> {
    pub fn new(time: &'a str, style: Style) -> Self {
        Self {
            time,
            label: None,
            style,
        }
    }

    pub const fn label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }

    pub fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let time = compact_time(self.time);
        let label_height = if self.label.is_some() { 2 } else { 0 };

        for font in [&LARGE, &SMALL] {
            let Some(glyphs) = font.glyphs(time) else {
                break;
            };
            let width = Font::width(&glyphs);
            let height = font.height + label_height;
            if width <= area.width && height <= area.height {
                let top = area.y + (area.height - height) / 2;
                self.render_label(area, top, buf);
                let mut x = area.x + (area.width - width) / 2;
                for glyph in glyphs {
                    for (row, line) in (top + label_height..).zip(glyph.iter()) {
                        buf.set_string(x, row, line, self.style);
                    }
                    x += u16::try_from(glyph[0].chars().count()).unwrap_or(0) + GLYPH_GAP;
                }
                return;
            }
        }

        // Plain-text fallback for tiny panes.
        if self.label.is_some() && area.height > 1 {
            let top = area.y + (area.height - 2) / 2;
            self.render_label(area, top, buf);
            Self::centered(area, top + 1, time, self.style, buf);
        } else {
            let line = self
                .label
                .map_or_else(|| time.to_owned(), |label| format!("{label}: {time}"));
            Self::centered(area, area.y + (area.height - 1) / 2, &line, self.style, buf);
        }
    }

    fn render_label(&self, area: Rect, y: u16, buf: &mut Buffer) {
        if let Some(label) = self.label {
            Self::centered(area, y, label, self.style, buf);
        }
    }

    fn centered(area: Rect, y: u16, text: &str, style: Style, buf: &mut Buffer) {
        let width = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
        let x = area.x + area.width.saturating_sub(width) / 2;
        buf.set_stringn(x, y, text, usize::from(area.right() - x), style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(buf: &Buffer) -> Vec<String> {
        let area = buf.area;
        (area.top()..area.bottom())
            .map(|y| {
                (area.left()..area.right())
                    .map(|x| buf[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn compact_time_drops_leading_zero_minutes() {
        assert_eq!(compact_time("00:00.000"), "0.000");
        assert_eq!(compact_time("00:07.123"), "7.123");
        assert_eq!(compact_time("00:12.345"), "12.345");
        assert_eq!(compact_time("01:02.345"), "1:02.345");
        assert_eq!(compact_time("12:02.345"), "12:02.345");
    }

    #[test]
    fn large_digits_are_centered_both_ways() {
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        BigTimerWidget::new("00:00.000", Style::default()).render(area, &mut buf);
        let rows = rows(&buf);
        // "0.000" is 4 digits + a dot: 4*5 + 2 + 4 gaps = 26 wide, 4 tall,
        // leaving 7 columns on each side and 3 rows above and below.
        assert_eq!(rows[2].trim(), "");
        assert_eq!(rows[7].trim(), "");
        assert_eq!(
            rows[3],
            format!("{:7}██▀██    ██▀██ ██▀██ ██▀██{:7}", "", "")
        );
        assert_eq!(
            rows[6],
            format!("{:7}▀▀▀▀▀ ▀▀ ▀▀▀▀▀ ▀▀▀▀▀ ▀▀▀▀▀{:7}", "", "")
        );
    }

    #[test]
    fn falls_back_to_small_then_plain_text() {
        let area = Rect::new(0, 0, 21, 3);
        let mut buf = Buffer::empty(area);
        BigTimerWidget::new("00:12.345", Style::default()).render(area, &mut buf);
        assert_eq!(rows(&buf)[0], "▀█  ▀▀█   ▀▀█ █ █ █▀▀");

        let area = Rect::new(0, 0, 12, 3);
        let mut buf = Buffer::empty(area);
        BigTimerWidget::new("00:12.345", Style::default())
            .label(Some("Inspect"))
            .render(area, &mut buf);
        let rows = rows(&buf);
        assert_eq!(rows[0].trim(), "Inspect");
        assert_eq!(rows[1].trim(), "12.345");
    }
}
