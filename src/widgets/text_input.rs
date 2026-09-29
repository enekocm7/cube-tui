//! Reusable single-line text editor and themed modal rendering.
//!
//! For application actions, use [`crate::model::Model::request_text_input`].
//! To use the editor independently, construct [`TextInput`], pass input to
//! [`TextInput::handle_event`], and paint it last with
//! [`TextInput::render_with_theme`]. Discard the editor after completion.
//!
//! # Controls
//!
//! - Enter submits; Esc or Ctrl+C cancels.
//! - Left/Right move by Unicode grapheme, including combined emoji.
//! - Home/End move to the beginning/end.
//! - Backspace/Delete remove the grapheme before/at the cursor.
//! - Paste inserts at the cursor, flattening newlines and tabs to spaces and
//!   removing terminal control characters.
//!
//! Editing supports held-key repeats and ignores key releases. Long values
//! scroll horizontally to keep the highlighted cursor visible. The host must
//! capture all keyboard/paste events while the editor is open, even ignored
//! ones, and resume global keybindings only after closing it.

use std::ops::ControlFlow;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::model::settings::ThemeColors;

/// The terminal outcome of a text prompt. Empty submissions are valid text.
#[derive(Debug, PartialEq, Eq)]
pub enum TextInputResult {
    /// The owned text accepted with Enter, including an empty string.
    Submitted(String),
    /// The prompt was dismissed without submitting its text.
    Cancelled,
}

/// Single-line editor state, independent of application actions and screens.
pub struct TextInput {
    title: String,
    text: String,
    // Byte offset, always at an extended grapheme boundary.
    cursor: usize,
}

impl TextInput {
    /// Creates a prompt with the cursor at the end of its initial text.
    /// Newlines and tabs in `initial` become spaces; control characters are removed.
    pub fn new(title: impl Into<String>, initial: &str) -> Self {
        let text = single_line(initial);
        Self {
            title: title.into(),
            cursor: text.len(),
            text,
        }
    }

    /// Handles editing and paste, returning the outcome or whether to redraw.
    ///
    /// [`ControlFlow::Break`] signals submission or cancellation; discard the
    /// editor afterward. [`ControlFlow::Continue`] reports whether editing
    /// changed the visible state and needs a redraw.
    ///
    /// The caller must capture every keyboard/paste event while the prompt is
    /// open, including events this editor ignores, to prevent shortcut leakage.
    pub fn handle_event(&mut self, event: &Event) -> ControlFlow<TextInputResult, bool> {
        let key = match event {
            Event::Paste(text) => return ControlFlow::Continue(self.insert(text)),
            Event::Key(key) if key.kind != KeyEventKind::Release => key,
            _ => return ControlFlow::Continue(false),
        };

        if key.kind == KeyEventKind::Press {
            if key.code == KeyCode::Esc
                || (key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL)
            {
                return ControlFlow::Break(TextInputResult::Cancelled);
            }
            if key.code == KeyCode::Enter && key.modifiers.is_empty() {
                self.cursor = 0;
                return ControlFlow::Break(TextInputResult::Submitted(self.text.clone()));
            }
        }
        if key.modifiers.intersects(
            KeyModifiers::CONTROL
                | KeyModifiers::ALT
                | KeyModifiers::SUPER
                | KeyModifiers::HYPER
                | KeyModifiers::META,
        ) {
            return ControlFlow::Continue(false);
        }

        let old_cursor = self.cursor;
        let old_len = self.text.len();
        match key.code {
            KeyCode::Char(character) if !character.is_control() => {
                return ControlFlow::Continue(self.insert(character.encode_utf8(&mut [0; 4])));
            }
            KeyCode::Left => self.cursor = self.previous_boundary(),
            KeyCode::Right => self.cursor = self.next_boundary(),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.text.len(),
            KeyCode::Backspace => {
                let start = self.previous_boundary();
                self.text.replace_range(start..self.cursor, "");
                self.cursor = start;
                self.snap_cursor_forward();
            }
            KeyCode::Delete => {
                self.text
                    .replace_range(self.cursor..self.next_boundary(), "");
                self.snap_cursor_forward();
            }
            _ => {}
        }
        ControlFlow::Continue(self.cursor != old_cursor || self.text.len() != old_len)
    }

    fn previous_boundary(&self) -> usize {
        self.text[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map_or(0, |(index, _)| index)
    }

    fn next_boundary(&self) -> usize {
        self.text[self.cursor..]
            .graphemes(true)
            .next()
            .map_or(self.cursor, |grapheme| self.cursor + grapheme.len())
    }

    fn insert(&mut self, text: &str) -> bool {
        let text = single_line(text);
        if text.is_empty() {
            return false;
        }
        self.text.insert_str(self.cursor, &text);
        self.cursor += text.len();
        self.snap_cursor_forward();
        true
    }

    // Inserting/deleting can join adjacent graphemes (for example emoji joined
    // by a ZWJ). Re-establish the boundary before the next edit or render.
    fn snap_cursor_forward(&mut self) {
        self.cursor = self
            .text
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .find(|&index| index >= self.cursor)
            .unwrap_or(self.text.len());
    }

    /// Paints an opaque, centered popup. Call last to place it over other UI.
    pub fn render_with_theme(&self, area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
        let width = area.width.min(64);
        let height = area.height.min(5);
        let popup = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        Clear.render(popup, buf);
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .style(Style::default().fg(theme.text()).bg(theme.background()))
            .border_style(Style::default().fg(theme.border()));
        let inner = block.inner(popup);
        block.render(popup, buf);
        if inner.is_empty() {
            return;
        }

        let cursor_end = self.next_boundary();
        let cursor_text = if self.cursor == self.text.len() {
            " "
        } else {
            &self.text[self.cursor..cursor_end]
        };
        let cursor_width = cursor_text.width().max(1);
        let available = usize::from(inner.width).saturating_sub(cursor_width);
        let mut start = self.cursor;
        let mut used = 0;
        for (index, grapheme) in self.text[..self.cursor].grapheme_indices(true).rev() {
            used += grapheme.width();
            if used > available {
                break;
            }
            start = index;
        }
        // A wide character cannot fit into a one-column terminal; keep a visible
        // caret in that case rather than letting the renderer clip it away.
        let cursor_text = if cursor_width > usize::from(inner.width) {
            " "
        } else {
            cursor_text
        };
        let line = Line::from(vec![
            Span::raw(&self.text[start..self.cursor]),
            Span::styled(
                cursor_text,
                Style::default()
                    .fg(theme.selection_text())
                    .bg(theme.selection()),
            ),
            Span::raw(&self.text[cursor_end..]),
        ]);
        Paragraph::new(line).render(Rect::new(inner.x, inner.y, inner.width, 1), buf);
        if inner.height >= 3 {
            Paragraph::new("Enter: submit  Esc: cancel  \u{2190}/\u{2192}: move")
                .render(Rect::new(inner.x, inner.y + 2, inner.width, 1), buf);
        }
    }
}

/// Flattens pasted lines/tabs and strips terminal control characters.
fn single_line(text: &str) -> String {
    text.replace("\r\n", "\n")
        .chars()
        .filter_map(|character| match character {
            '\n' | '\r' | '\t' | '\u{2028}' | '\u{2029}' => Some(' '),
            character if character.is_control() => None,
            character => Some(character),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyEvent;

    use super::*;

    fn key(input: &mut TextInput, code: KeyCode) -> ControlFlow<TextInputResult, bool> {
        input.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
    }

    #[test]
    fn editing_preserves_combined_characters_and_emoji() {
        let mut input = TextInput::new("Name", "ae\u{301}👩‍💻界");
        let _ = key(&mut input, KeyCode::Left);
        let _ = key(&mut input, KeyCode::Backspace);
        assert_eq!(input.text, "ae\u{301}界");
        let _ = key(&mut input, KeyCode::Left);
        let _ = key(&mut input, KeyCode::Delete);
        assert_eq!(input.text, "a界");
        let _ = key(&mut input, KeyCode::Char('é'));
        assert_eq!(input.text, "aé界");
        assert_eq!(
            key(&mut input, KeyCode::Enter),
            ControlFlow::Break(TextInputResult::Submitted("aé界".into()))
        );
    }

    #[test]
    fn insertion_that_joins_graphemes_keeps_cursor_at_a_boundary() {
        let mut input = TextInput::new("Emoji", "👩💻");
        let _ = key(&mut input, KeyCode::Left);
        let _ = key(&mut input, KeyCode::Char('\u{200d}'));
        assert_eq!(input.text, "👩‍💻");
        let _ = key(&mut input, KeyCode::Backspace);
        assert_eq!(input.text, "");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn paste_is_single_line_and_inserts_at_the_cursor() {
        let mut input = TextInput::new("Paste", "ab");
        let _ = key(&mut input, KeyCode::Left);
        assert_eq!(
            input.handle_event(&Event::Paste("界\r\nq\t?\u{1b}\u{7}".into())),
            ControlFlow::Continue(true)
        );
        assert_eq!(input.text, "a界 q ?b");
        let _ = key(&mut input, KeyCode::Home);
        assert_eq!(
            key(&mut input, KeyCode::Backspace),
            ControlFlow::Continue(false)
        );
        let _ = key(&mut input, KeyCode::End);
        assert_eq!(
            key(&mut input, KeyCode::Delete),
            ControlFlow::Continue(false)
        );
    }

    #[test]
    fn repeats_edit_but_releases_and_modified_shortcuts_do_not() {
        let mut input = TextInput::new("Input", "");
        let mut event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        event.kind = KeyEventKind::Repeat;
        assert_eq!(
            input.handle_event(&Event::Key(event)),
            ControlFlow::Continue(true)
        );
        event.kind = KeyEventKind::Release;
        assert_eq!(
            input.handle_event(&Event::Key(event)),
            ControlFlow::Continue(false)
        );
        event.kind = KeyEventKind::Press;
        event.modifiers = KeyModifiers::CONTROL;
        assert_eq!(
            input.handle_event(&Event::Key(event)),
            ControlFlow::Continue(false)
        );
        assert_eq!(input.text, "q");
        event.code = KeyCode::Enter;
        event.modifiers = KeyModifiers::NONE;
        event.kind = KeyEventKind::Repeat;
        assert_eq!(
            input.handle_event(&Event::Key(event)),
            ControlFlow::Continue(false)
        );
    }

    #[test]
    fn rendering_scrolls_to_caret_and_handles_tiny_offset_areas() {
        let theme = crate::model::settings::Settings::default();
        let input = TextInput::new("Input", "abcdefghijklmnopqrstuvwxyz界");
        for width in 0..20 {
            for height in 0..7 {
                let area = Rect::new(3, 2, width, height);
                let mut buf = Buffer::empty(area);
                input.render_with_theme(area, &mut buf, theme.theme());
                if width >= 3 && height >= 3 {
                    let row = area.y + (height - height.min(5)) / 2 + 1;
                    assert!(
                        (area.x + 1..area.right() - 1).any(|x| {
                            let cell = &buf[(x, row)];
                            cell.symbol() == " " && cell.bg == theme.theme().selection()
                        }),
                        "caret missing at {width}x{height}"
                    );
                }
            }
        }
    }
}
