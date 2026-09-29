use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::model::keybinds::{Action, Keybinds};
use crate::model::settings::ThemeColors;
use crate::model::solve_editor::{EditorPage, FIELD_NAMES, SolveEditor, date_text, penalty_text};
use crate::scramble::WcaEvent;
use crate::widgets::history::{History, SolveSnapshot, format_millis};

pub fn render(
    editor: &mut SolveEditor,
    history: &History,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeColors,
    keys: &Keybinds,
) {
    let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(area);
    let body = layout[0];
    let navigation = format!(
        "{}/{}: select",
        keys.label(Action::SelectUp),
        keys.label(Action::SelectDown)
    );
    let help = match editor.page {
        EditorPage::Fields => format!(
            "{navigation} / Tab: next   Home/End: first/last   {}: edit / choose   Ctrl+S: save   {}: cancel draft",
            keys.label(Action::Enter),
            keys.label(Action::Back)
        ),
        EditorPage::Events => format!(
            "{navigation}   Home/End: first/last   {}: choose event   {}: cancel selection",
            keys.label(Action::Enter),
            keys.label(Action::Back)
        ),
        EditorPage::History => format!(
            "{navigation} / PgUp/PgDn: scroll   Home/End   {}: back to editor",
            keys.label(Action::Back)
        ),
    };
    Paragraph::new(help)
        .style(Style::default().fg(theme.text()))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .render(layout[1], buf);

    match editor.page {
        EditorPage::Fields => {
            let layout = Layout::vertical([
                Constraint::Min(5),
                Constraint::Length(5),
                Constraint::Length(if editor.error.is_some() { 3 } else { 0 }),
            ])
            .split(body);
            let changes = history
                .times()
                .get(editor.index)
                .map_or(0, |time| time.changes().len());
            let mut items: Vec<_> = FIELD_NAMES[..7]
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let value = if index == 6 {
                        format!("{changes} saved revisions")
                    } else {
                        editor.value(index)
                    };
                    ListItem::new(format!(
                        "{name}: {}",
                        value.replace(['\n', '\r', '\t'], " ")
                    ))
                })
                .collect();
            items.push(ListItem::new(Line::from(Span::styled(
                "─".repeat(usize::from(layout[0].width.saturating_sub(4))),
                Style::default().fg(theme.border()),
            ))));
            items.extend(FIELD_NAMES[7..].iter().map(|name| {
                ListItem::new(*name).style(Style::default().add_modifier(Modifier::BOLD))
            }));
            let title = format!(
                "Edit solve #{}{}",
                editor.index + 1,
                if editor.is_dirty() {
                    " • unsaved changes"
                } else {
                    ""
                }
            );
            render_list(
                items,
                Some(editor.selected + usize::from(editor.selected >= 7)),
                &title,
                layout[0],
                buf,
                theme,
            );
            let hint = match editor.selected {
                0 => {
                    "Raw time before +2; seconds, M:SS or H:MM:SS.mmm. Zero is valid only for DNF."
                }
                1 => {
                    "Choose from supported events. The scramble must match the chosen event before saving."
                }
                3 => "Enter or Left/Right: None, +2, DNF.",
                4 => "RFC 3339 with timezone, e.g. 2026-09-29T14:30:00.000Z. Blank means unknown.",
                5 => "Optional comment, up to 4096 characters. Enter opens the text editor.",
                6 => "Browse timestamped before/after values for every saved edit, newest first.",
                7 => "Validate and save all fields together. Statistics update immediately.",
                8 => "Discard this draft and return to solve details.",
                _ => {
                    "Enter opens the text editor. Scramble notation is validated for the selected event."
                }
            };
            Paragraph::new(format!("{hint}\n{}", editor.value(editor.selected)))
                .block(block(
                    if editor.selected >= 7 {
                        "Selected action"
                    } else {
                        "Selected field"
                    },
                    theme,
                ))
                .wrap(Wrap { trim: false })
                .render(layout[1], buf);
            if let Some(error) = &editor.error {
                Paragraph::new(error.as_str())
                    .block(block("Cannot save", theme))
                    .wrap(Wrap { trim: false })
                    .render(layout[2], buf);
            }
        }
        EditorPage::Events => {
            render_list(
                WcaEvent::ALL
                    .iter()
                    .map(|event| ListItem::new(event.name())),
                Some(editor.event_selection),
                "Choose solve event",
                body,
                buf,
                theme,
            );
        }
        EditorPage::History => {
            let mut lines = Vec::new();
            if let Some(time) = history.times().get(editor.index) {
                for (index, change) in time.changes().iter().enumerate().rev() {
                    lines.push(Line::from(format!(
                        "Revision {} • {}",
                        index + 1,
                        date_text(change.changed_at_unix_ms)
                    )));
                    let before = snapshot_values(&change.before);
                    let after = snapshot_values(&change.after);
                    for field in 0..before.len() {
                        if before[field] != after[field] {
                            lines.push(Line::from(format!("{}:", FIELD_NAMES[field])));
                            lines.extend(
                                format!("  Before: {}", display_value(&before[field]))
                                    .lines()
                                    .map(|line| Line::from(line.to_owned())),
                            );
                            lines.extend(
                                format!("  After:  {}", display_value(&after[field]))
                                    .lines()
                                    .map(|line| Line::from(line.to_owned())),
                            );
                        }
                    }
                    lines.push(Line::from(""));
                }
            }
            if lines.is_empty() {
                lines.push(Line::from("No saved changes for this solve."));
            }
            let frame = block("Solve change history • newest first", theme);
            let inner = frame.inner(body);
            frame.render(body, buf);
            let paragraph = Paragraph::new(lines)
                .style(Style::default().fg(theme.text()))
                .wrap(Wrap { trim: false });
            editor.history_max_scroll = u16::try_from(paragraph.line_count(inner.width))
                .unwrap_or(u16::MAX)
                .saturating_sub(inner.height);
            editor.history_scroll = editor.history_scroll.min(editor.history_max_scroll);
            paragraph
                .scroll((editor.history_scroll, 0))
                .render(inner, buf);
        }
    }
}

fn block<'a>(title: &'a str, theme: &ThemeColors) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(theme.text()).bg(theme.background()))
        .border_style(Style::default().fg(theme.border()))
}

fn render_list<'a>(
    items: impl IntoIterator<Item = ListItem<'a>>,
    selected: Option<usize>,
    title: &str,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeColors,
) {
    let list = List::new(items)
        .block(block(title, theme))
        .highlight_style(
            Style::default()
                .fg(theme.selection_text())
                .bg(theme.selection()),
        )
        .highlight_symbol("› ");
    let mut state = ListState::default().with_selected(selected);
    StatefulWidget::render(list, area, buf, &mut state);
}

fn snapshot_values(snapshot: &SolveSnapshot) -> [String; 6] {
    [
        format_millis(snapshot.time_ms),
        snapshot.event.name().to_owned(),
        snapshot.scramble.clone(),
        penalty_text(snapshot.modifier).to_owned(),
        date_text(snapshot.solved_at_unix_ms),
        snapshot.comment.clone(),
    ]
}

fn display_value(value: &str) -> &str {
    if value.is_empty() { "(empty)" } else { value }
}
