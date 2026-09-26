use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::model::{
    settings::ThemeColors,
    toast::{ToastBuffer, ToastType},
};

/// Renders buffered toasts from the bottom right upward, oldest first.
/// Overflow waits in the buffer until a later frame has room for it.
pub fn render_toasts(area: Rect, buf: &mut Buffer, toasts: &mut ToastBuffer, theme: &ThemeColors) {
    let width = area.width.min(48);
    if width < 3 {
        return;
    }
    let now = Instant::now();
    let mut bottom = area.bottom();
    for toast in toasts.iter_mut() {
        let available = bottom.saturating_sub(area.y);
        if available < 3 {
            break;
        }
        let (title, color) = match toast.kind {
            ToastType::Info => (" Info ", Color::Cyan),
            ToastType::Warning => (" Warning ", Color::Yellow),
            ToastType::Error => (" Error ", Color::Red),
        };
        let paragraph = Paragraph::new(toast.message.as_str())
            .style(Style::default().fg(theme.text()).bg(theme.background()))
            .wrap(Wrap { trim: false });
        let required_height = paragraph.line_count(width - 2).max(1).saturating_add(2);
        // Wait for enough room to show the whole message. A single message
        // taller than the terminal must be clipped to the viewport.
        if required_height > usize::from(available) && bottom != area.bottom() {
            break;
        }
        let height = required_height.min(usize::from(available)) as u16;
        let toast_area = Rect::new(area.right() - width, bottom - height, width, height);
        Clear.render(toast_area, buf);
        paragraph
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(color)),
            )
            .render(toast_area, buf);
        toast.mark_visible(now);
        bottom = toast_area.y.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::model::{settings::Settings, toast::ToastDuration};

    #[test]
    fn stacks_at_bottom_right_and_clears_underlying_content() {
        let area = Rect::new(5, 7, 80, 20);
        let mut buf = Buffer::empty(area);
        for cell in &mut buf.content {
            cell.set_symbol("x");
        }
        let mut toasts = ToastBuffer::default();
        toasts.push("first", ToastType::Info, ToastDuration::Short);
        toasts.push("second", ToastType::Warning, ToastDuration::Long);
        toasts.push("third", ToastType::Error, ToastDuration::Short);
        render_toasts(area, &mut buf, &mut toasts, Settings::default().theme());
        let x = area.right() - 48;
        for (offset, color, first_letter) in [
            (0, Color::Cyan, "f"),
            (4, Color::Yellow, "s"),
            (8, Color::Red, "t"),
        ] {
            let y = area.bottom() - 3 - offset;
            assert_eq!(buf[(x, y)].fg, color);
            assert_eq!(buf[(x + 1, y + 1)].symbol(), first_letter);
            assert_eq!(buf[(area.right() - 2, y + 1)].symbol(), " ");
        }
        assert_eq!(buf[(area.x, area.y)].symbol(), "x");
    }

    #[test]
    fn overflow_waits_for_room_before_starting_its_duration() {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        let mut toasts = ToastBuffer::default();
        toasts.push("first", ToastType::Info, ToastDuration::Short);
        toasts.push("second", ToastType::Info, ToastDuration::Long);
        render_toasts(area, &mut buf, &mut toasts, Settings::default().theme());
        assert!(toasts.remove_expired(Instant::now() + Duration::from_secs(60)));
        assert_eq!(toasts.next_expiration(Instant::now()), None);
        assert_eq!(toasts.iter_mut().count(), 1);
        render_toasts(area, &mut buf, &mut toasts, Settings::default().theme());
        assert_eq!(buf[(1, 1)].symbol(), "s");
        assert!(toasts.next_expiration(Instant::now()).unwrap() > Duration::from_secs(5));
    }

    #[test]
    fn wrapped_messages_wait_for_enough_room() {
        let area = Rect::new(2, 3, 10, 7);
        let mut buf = Buffer::empty(area);
        let mut toasts = ToastBuffer::default();
        toasts.push("first", ToastType::Info, ToastDuration::Short);
        toasts.push("abcdefgh ijklmnop", ToastType::Warning, ToastDuration::Long);
        render_toasts(area, &mut buf, &mut toasts, Settings::default().theme());
        assert!(toasts.remove_expired(Instant::now() + Duration::from_secs(60)));
        assert_eq!(toasts.iter_mut().count(), 1);
        assert_eq!(toasts.next_expiration(Instant::now()), None);
        render_toasts(area, &mut buf, &mut toasts, Settings::default().theme());
        assert_eq!(buf[(3, area.bottom() - 3)].symbol(), "a");
        assert_eq!(buf[(3, area.bottom() - 2)].symbol(), "i");
    }

    #[test]
    fn tiny_areas_leave_toasts_pending() {
        for (width, height) in [(0, 0), (1, 10), (10, 2), (2, 2)] {
            let area = Rect::new(4, 5, width, height);
            let mut buf = Buffer::empty(area);
            let mut toasts = ToastBuffer::default();
            toasts.push("pending", ToastType::Info, ToastDuration::Short);
            render_toasts(area, &mut buf, &mut toasts, Settings::default().theme());
            assert_eq!(toasts.next_expiration(Instant::now()), None);
        }
    }
}
