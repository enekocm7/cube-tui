use std::borrow::Cow;

#[cfg(any(feature = "bluetooth", feature = "dashboard"))]
pub mod runtime;

pub fn get_scramble_lines(scramble: &str, width: u16) -> u16 {
    //10 is the padding (5 on each side) so the max chars are width - 10
    let chars_per_line = width as usize - 10;
    let num_lines = scramble.len().div_ceil(chars_per_line);
    u16::try_from(num_lines).unwrap_or(5)
}

pub fn print_as_link(path: &std::path::Path) {
    let display = path.display();
    let url = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
    println!("\x1b]8;;{url}\x1b\\{display}\x1b]8;;\x1b\\");
}

pub fn format_elapsed(ms: u64) -> Cow<'static, str> {
    if ms == 0 {
        return Cow::Borrowed("00:00.000");
    }
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;
    Cow::Owned(format!("{minutes:02}:{seconds:02}.{millis:03}"))
}
