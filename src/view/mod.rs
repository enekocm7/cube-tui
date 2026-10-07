use std::borrow::Cow;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::model::keybinds::Action;
use crate::model::settings::{Settings, ThemeColors};
use crate::model::toast::ToastBuffer;
use crate::model::{Model, TimerState};
use crate::utils::{format_elapsed, get_scramble_lines};
use crate::widgets::big_timer::BigTimerWidget;
use crate::widgets::confirmation::ConfirmationWidget;
use crate::widgets::detailed_stats::DetailedStatsWidget;
use crate::widgets::details::DetailsWidget;
use crate::widgets::help::HelpWidget;
use crate::widgets::history::Modifier as SolveModifier;
use crate::widgets::mean_detail::MeanDetailWidget;
use crate::widgets::scramble::ScrambleWidget;
use crate::widgets::scramble_preview::ScramblePreviewWidget;
use crate::widgets::stats::StatsWidget;

#[cfg(feature = "bluetooth")]
use crate::widgets::bluetooth::BluetoothWidget;
use crate::widgets::toast::render_toasts;

/// Renders the complete application view into the supplied terminal buffer.
pub fn view(area: Rect, buf: &mut ratatui::buffer::Buffer, model: &mut Model) {
    render_screen(area, buf, model);
    if model.settings().toasts() {
        let theme = *model.settings().theme();
        render_toasts(area, buf, &mut model.toasts, &theme);
    } else {
        model.toasts = ToastBuffer::default();
    }
    if let Some(prompt) = &model.text_input {
        prompt
            .input
            .render_with_theme(area, buf, model.settings().theme());
    }
}

#[allow(clippy::too_many_lines)]
fn render_screen(area: Rect, buf: &mut ratatui::buffer::Buffer, model: &mut Model) {
    let settings = model.settings();
    let theme = *settings.theme();
    let keybinds = settings.keybinds().clone();

    Block::default()
        .style(Style::new().bg(theme.background()))
        .render(area, buf);

    if area.width < settings.minimum_terminal_width()
        || area.height < settings.minimum_terminal_height()
    {
        render_terminal_size_error(area, buf, settings, &theme);
        return;
    }

    if model.show_help() {
        let help_widget = HelpWidget::new(model.help_scroll());
        model.set_help_max_scroll(HelpWidget::max_scroll_for_height(area.height));
        help_widget.render_with_theme(area, buf, &theme, &keybinds);
        return;
    }

    if let Some(editor) = &mut model.solve_editor {
        crate::widgets::solve_editor::render(
            editor,
            &model.session_state.sessions[editor.session].history,
            area,
            buf,
            &theme,
            &keybinds,
        );
        return;
    }

    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        use crate::model::bluetooth::BluetoothScreenState;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(area);

        BluetoothWidget::new(
            model.bluetooth_devices().to_vec(),
            model.bluetooth_selected_index(),
            model.bluetooth_status(),
            model.connected_device_id(),
        )
        .render_with_theme(layout[0], buf, &theme);

        let help_text = match model.bluetooth_screen_state() {
            BluetoothScreenState::Connected => Line::from(vec![
                Span::styled("↑/↓: select  ", Style::default().fg(theme.text())),
                Span::styled("Enter/x: disconnect  ", Style::default().fg(theme.text())),
                Span::styled("Esc: back to timer", Style::default().fg(theme.text())),
            ]),
            BluetoothScreenState::Connecting => Line::from(vec![
                Span::styled("↑/↓: select device  ", Style::default().fg(theme.text())),
                Span::styled("Esc: back to timer", Style::default().fg(theme.text())),
            ]),
            BluetoothScreenState::Searching => Line::from(vec![
                Span::styled("↑/↓: select device  ", Style::default().fg(theme.text())),
                Span::styled("Enter: connect  ", Style::default().fg(theme.text())),
                Span::styled("Esc: close", Style::default().fg(theme.text())),
            ]),
        };
        Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .render(layout[1], buf);
        return;
    }

    if model.show_mean_detail() {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(area);

        let widget = MeanDetailWidget::new(
            model.history(),
            model.detailed_stats_row(),
            model.detailed_stats_col(),
            model.mean_detail_selected_index(),
        );
        widget.render(layout[0], buf, &theme);

        let help_text = Line::from(vec![
            Span::styled("↑/↓: select time  ", Style::default().fg(theme.text())),
            Span::styled("Enter: open details  ", Style::default().fg(theme.text())),
            Span::styled("Esc: back", Style::default().fg(theme.text())),
        ]);
        Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .render(layout[1], buf);
        return;
    }

    if model.show_detailed_stats() {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(area);

        DetailedStatsWidget::new(
            model.history(),
            model.detailed_stats_row(),
            model.detailed_stats_col(),
        )
        .render(layout[0], buf, &theme);

        let help_text = Line::from(vec![
            Span::styled("↑/↓: navigate  ", Style::default().fg(theme.text())),
            Span::styled("←/→: column  ", Style::default().fg(theme.text())),
            Span::styled("Enter: view mean  ", Style::default().fg(theme.text())),
            Span::styled("Esc: back", Style::default().fg(theme.text())),
        ]);
        Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .render(layout[1], buf);
        return;
    }

    if model.show_details() {
        let details_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(area);

        DetailsWidget::new(
            model.history().selected_time(),
            model.selected_details_modifier_index(),
        )
        .render_with_theme(details_layout[0], buf, &theme);

        let details_help = Line::from(vec![
            Span::styled(
                "Space: toggle modifier  ",
                Style::default().fg(theme.text()),
            ),
            Span::styled("↑/↓: select modifier  ", Style::default().fg(theme.text())),
            Span::styled("←/→: navigate times  ", Style::default().fg(theme.text())),
            Span::styled("d: delete  ", Style::default().fg(theme.text())),
            Span::styled(
                format!("{}: edit  ", keybinds.label(Action::EditSolve)),
                Style::default().fg(theme.text()),
            ),
            Span::styled("Esc: close", Style::default().fg(theme.text())),
        ]);
        Paragraph::new(details_help)
            .alignment(Alignment::Center)
            .render(details_layout[1], buf);
        return;
    }

    if model.zen_enabled() && matches!(model.timer_state(), TimerState::Running { .. }) {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(area);
        Paragraph::new(Line::from(Span::styled(
            "Solving...",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
                .bg(theme.background()),
        )))
        .alignment(Alignment::Center)
        .render(vertical[1], buf);
        return;
    }

    let settings = model.settings();
    let show_scramble = settings.scramble();
    let show_history = settings.history();
    let show_stats = settings.stats();
    let preview_widget = settings
        .scramble_preview()
        .then(|| ScramblePreviewWidget::new(model.event(), model.scramble()));

    let outer_constraints = if show_scramble {
        let scramble_lines = get_scramble_lines(model.scramble(), area.width);
        let scramble_height = (scramble_lines + 2).min(area.height.saturating_sub(1));
        vec![
            Constraint::Length(scramble_height),
            Constraint::Fill(1),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Fill(1), Constraint::Length(1)]
    };
    let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(outer_constraints)
        .margin(1)
        .split(area);

    let main_area_index = usize::from(show_scramble);
    let help_area_index = main_area_index + 1;

    let mut main_constraints = Vec::new();
    let mut history_area_index = None;
    let mut stats_area_index = None;

    if show_history {
        history_area_index = Some(main_constraints.len());
        main_constraints.push(Constraint::Length(24));
    }

    let timer_area_index = main_constraints.len();
    main_constraints.push(Constraint::Min(10));

    if show_stats {
        stats_area_index = Some(main_constraints.len());
        let desired_width = preview_widget
            .as_ref()
            .map_or(StatsWidget::WIDTH, |widget| {
                widget.width().max(StatsWidget::WIDTH)
            });
        let available_width = outer_layout[main_area_index]
            .width
            .saturating_sub(if show_history { 24 } else { 0 })
            .saturating_sub(10);
        main_constraints.push(Constraint::Length(desired_width.min(available_width)));
    }

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(main_constraints)
        .split(outer_layout[main_area_index]);

    let mut timer_area = main_layout[timer_area_index];
    let mut stats_area = stats_area_index.map(|index| main_layout[index]);
    let preview = preview_widget.map(|widget| {
        // Use the existing right column when stats are visible. Otherwise,
        // reserve the bottom of the timer pane and align the net to its right.
        let host_area = stats_area.unwrap_or(timer_area);
        let reserved_height = if stats_area.is_some() { 9 } else { 3 };
        let preview_height = widget.height_for(host_area.height.saturating_sub(reserved_height));
        let split = Layout::vertical([Constraint::Fill(1), Constraint::Length(preview_height)])
            .split(host_area);
        if stats_area.is_some() {
            stats_area = Some(split[0]);
        } else {
            timer_area = split[0];
        }
        let width = split[1].width.min(widget.width().max(30));
        let preview_area = Rect::new(split[1].right() - width, split[1].y, width, split[1].height);
        (widget, preview_area)
    });

    if show_scramble {
        ScrambleWidget::new(model.scramble(), model.event().name()).render_with_theme(
            outer_layout[0],
            buf,
            &theme,
        );
    }

    let mut history_title = format!(
        "{}{:02}/{:02}{}",
        if model.history().session_name().is_empty() {
            "Session: "
        } else {
            ""
        },
        model.current_session_index() + 1,
        model.session_count(),
        if model.is_at_max_sessions() {
            " (max 99)"
        } else {
            ""
        }
    );
    let session_name = model.history().session_name();
    if !session_name.is_empty() {
        history_title = format!("{session_name} | {history_title}");
    }
    if let Some(index) = history_area_index {
        let history_block = Block::default()
            .title(history_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()));
        history_block.render(main_layout[index], buf);
        let history_area = inner_area(main_layout[index]);
        let history = model.history();
        let highlight = !model.main_focus_is_stats();
        history.render_with_theme(history_area, buf, &theme, Some(highlight));
    }

    #[cfg(feature = "bluetooth")]
    let bt_label = model
        .connected_device_name()
        .map_or_else(String::new, |name| format!(" | {name}"));
    #[cfg(not(feature = "bluetooth"))]
    let bt_label = String::new();
    let timer_title = format!(
        "Timer{}{}{bt_label}",
        if model.inspection_enabled() {
            " | Inspection: On"
        } else {
            ""
        },
        if model.zen_enabled() {
            " | Zen: On"
        } else {
            ""
        }
    );
    let timer_block = Block::default()
        .title(timer_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border()));
    let (timer_label, timer_text, timer_style) = timer_display(model);
    if settings.big_timer() {
        let timer_inner = timer_block.inner(timer_area);
        timer_block.render(timer_area, buf);
        BigTimerWidget::new(&timer_text, timer_style)
            .label(timer_label)
            .render(timer_inner, buf);
    } else {
        let text = match timer_label {
            Some(label) => Cow::Owned(format!("{label}: {timer_text}")),
            None => timer_text,
        };
        Paragraph::new(Line::from(Span::styled(text, timer_style)))
            .block(timer_block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(timer_area, buf);
    }

    if let Some(stats_area) = stats_area {
        let history = model.history();
        let stats_widget = if model.main_focus_is_stats() {
            StatsWidget::new(history).with_selection(model.main_stats_row(), model.main_stats_col())
        } else {
            StatsWidget::new(history)
        };
        stats_widget.render(stats_area, buf, &theme);
    }

    if let Some((widget, preview_area)) = preview {
        widget.render(
            preview_area,
            buf,
            &theme,
            &keybinds.label(Action::ToggleScramblePreview),
        );
    }

    let mut help_spans = vec![
        Span::styled(
            format!("{}: hold/release  ", keybinds.label(Action::Timer)),
            Style::default().fg(theme.text()),
        ),
        Span::styled(
            format!("{}: details  ", keybinds.label(Action::Enter)),
            Style::default().fg(theme.text()),
        ),
        Span::styled(
            format!("{}: reset  ", keybinds.label(Action::ResetTimer)),
            Style::default().fg(theme.text()),
        ),
        Span::styled(
            format!("{}: quit  ", keybinds.label(Action::Quit)),
            Style::default().fg(theme.text()),
        ),
        Span::styled(
            format!(
                "{}: preview  ",
                keybinds.label(Action::ToggleScramblePreview)
            ),
            Style::default().fg(theme.text()),
        ),
        Span::styled(
            format!("{}: help", keybinds.label(Action::Help)),
            Style::default().fg(theme.text()),
        ),
    ];
    if show_history && show_stats {
        help_spans.insert(
            2,
            Span::styled(
                format!("{}: history/stats  ", keybinds.label(Action::ToggleFocus)),
                Style::default().fg(theme.text()),
            ),
        );
    }
    Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .render(outer_layout[help_area_index], buf);

    if let Some(confirmation) = model.confirmation() {
        let widget = ConfirmationWidget::new(&confirmation.message, confirmation.selection);
        widget.render_with_theme(area, buf, &theme);
    }

    if let Some(theme_selector) = &mut model.theme_selector {
        theme_selector.render(area, buf, &theme, &keybinds);
    }
}

/// Renders the minimum-size warning in place of the normal application UI.
fn render_terminal_size_error(
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    settings: &Settings,
    theme: &ThemeColors,
) {
    let min_width = settings.minimum_terminal_width();
    let min_height = settings.minimum_terminal_height();
    let text = Text::from(vec![
        Line::from(Span::styled(
            "Terminal too small",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "Resize to at least {min_width} columns x {min_height} rows."
        )),
        Line::from(format!("Current size: {} x {}", area.width, area.height)),
        Line::from(format!(
            "Press {} to quit.",
            settings.keybinds().label(Action::Quit)
        )),
    ]);
    let top_padding = if usize::from(area.width) >= text.width() {
        area.height.saturating_sub(text.height() as u16) / 2
    } else {
        0
    };
    Paragraph::new(text)
        .style(Style::default().fg(theme.text()))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .render(
            Rect::new(
                area.x,
                area.y + top_padding,
                area.width,
                area.height.saturating_sub(top_padding),
            ),
            buf,
        );
}

/// Returns an area's interior after removing a one-cell border.
const fn inner_area(area: Rect) -> Rect {
    Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

/// Chooses the timer caption, text and style for the model's current state.
fn timer_display(model: &Model) -> (Option<&'static str>, Cow<'static, str>, Style) {
    let theme = model.settings().theme();
    let inspection_limit = model.settings().inspection_limit();
    let style = match model.timer_state() {
        TimerState::Idle if model.displayed_modifier() == SolveModifier::DNF => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
        TimerState::Idle => Style::default().fg(theme.text()),
        TimerState::Inspection { time, .. }
            if time.elapsed().as_millis() as u64 >= inspection_limit =>
        {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
        TimerState::Pulsed | TimerState::Inspection { pulsed: true, .. } => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
        TimerState::Running { .. } => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        TimerState::Inspection { pulsed: false, .. } => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    };

    let (label, text) = match model.timer_state() {
        TimerState::Pulsed => (None, format_elapsed(0)),
        TimerState::Inspection { .. } => (Some("Inspection"), format_elapsed(model.elapsed_ms())),
        _ => (None, format_elapsed(model.elapsed_ms())),
    };

    (label, text, style)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn preview_model() -> Model {
        let mut model = Model::new();
        model.settings.set_scramble_preview(true);
        model.current_session_mut().scramble = Some(crate::scramble::Scramble::new("R"));
        model
    }

    fn render_model(model: &mut Model, area: Rect) -> ratatui::buffer::Buffer {
        let mut buf = ratatui::buffer::Buffer::empty(area);
        view(area, &mut buf, model);
        buf
    }

    fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
        buf.content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn preview_renders_current_scramble_at_bottom_right_below_stats() {
        let mut model = preview_model();
        let area = Rect::new(4, 2, 100, 32);
        let buf = render_model(&mut model, area);
        let text = buffer_text(&buf);
        assert!(text.contains("Preview [v]"));
        assert!(!text.contains("U / L F R B / D"));
        assert!(text.contains("ao100"));
        // The U face's right column turns green after R. The last panel border
        // ends above the footer, at the right edge of the main layout.
        let net_y = area.bottom() - 12;
        assert_eq!(buf[(area.right() - 22, net_y)].symbol(), "W");
        assert_eq!(buf[(area.right() - 18, net_y)].symbol(), "G");
        assert_eq!(buf[(area.right() - 2, area.bottom() - 3)].symbol(), "┘");

        model.settings.set_scramble_preview(false);
        let hidden = render_model(&mut model, area);
        assert!(!buffer_text(&hidden).contains("Preview ["));
        assert!(buffer_text(&hidden).contains("ao100"));
        assert_ne!(hidden[(area.right() - 18, net_y)].symbol(), "G");
    }

    #[test]
    fn preview_tracks_new_scrambles_sessions_and_events() {
        let mut model = preview_model();
        let area = Rect::new(0, 0, 100, 32);
        let sticker = (area.right() - 18, area.bottom() - 12);
        assert_eq!(render_model(&mut model, area)[sticker].symbol(), "G");
        model.current_session_mut().scramble = Some(crate::scramble::Scramble::new("U"));
        assert_eq!(render_model(&mut model, area)[sticker].symbol(), "W");

        model.add_session();
        model.current_session_mut().scramble = Some(crate::scramble::Scramble::new("R"));
        assert_eq!(render_model(&mut model, area)[sticker].symbol(), "G");
        model.prev_session();
        assert_eq!(render_model(&mut model, area)[sticker].symbol(), "W");

        model.current_session_mut().event = crate::scramble::WcaEvent::Cube2x2;
        let smaller_cube = render_model(&mut model, area);
        assert!(buffer_text(&smaller_cube).contains("Preview [v]"));
        assert!(!buffer_text(&smaller_cube).contains("not available"));
        assert_ne!(smaller_cube[sticker].symbol(), "W");

        model.current_session_mut().event = crate::scramble::WcaEvent::Pyraminx;
        assert!(buffer_text(&render_model(&mut model, area)).contains("not available"));
        model.current_session_mut().event = crate::scramble::WcaEvent::Cube3x3;
        assert!(!buffer_text(&render_model(&mut model, area)).contains("not available"));
    }

    #[test]
    fn small_terminal_keeps_stats_and_shows_resize_hint() {
        let mut model = preview_model();
        let area = Rect::new(0, 0, model.settings().minimum_terminal_width(), 20);
        let buf = render_model(&mut model, area);
        let text = buffer_text(&buf);
        assert!(text.contains("Resize to see preview"));
        assert!(text.contains("ao100"));
    }

    #[rstest::rstest]
    #[case(true, true)]
    #[case(true, false)]
    #[case(false, true)]
    #[case(false, false)]
    fn preview_handles_panel_combinations_and_minimum_sizes(
        #[case] history: bool,
        #[case] stats: bool,
    ) {
        let mut model = preview_model();
        model.set_settings(
            toml::from_str(&format!(
                "[display]\nhistory = {history}\nstats = {stats}\nscramble_preview = true"
            ))
            .unwrap(),
        );
        let min_area = Rect::new(
            3,
            5,
            model.settings().minimum_terminal_width(),
            model.settings().minimum_terminal_height(),
        );
        assert!(buffer_text(&render_model(&mut model, min_area)).contains("Preview [v]"));
        let large_area = Rect::new(3, 5, 100, 32);
        let buf = render_model(&mut model, large_area);
        assert_eq!(
            buf[(large_area.right() - 18, large_area.bottom() - 12)].symbol(),
            "G"
        );
        assert_eq!(
            buf[(large_area.right() - 2, large_area.bottom() - 3)].symbol(),
            "┘"
        );
    }

    #[test]
    fn every_event_renders_at_bottom_right_and_large_puzzles_widen_the_column() {
        let mut model = preview_model();
        let area = Rect::new(2, 3, 100, 65);
        for event in crate::scramble::WcaEvent::ALL {
            model.current_session_mut().event = event;
            model.current_session_mut().scramble = Some(crate::scramble::generate_scramble(event));
            let buf = render_model(&mut model, area);
            let text = buffer_text(&buf);
            assert!(text.contains("Preview [v]"), "{event:?}");
            assert!(!text.contains("Resize to see preview"), "{event:?}");
            assert!(text.contains("ao100"), "{event:?}");
            if matches!(
                event,
                crate::scramble::WcaEvent::Pyraminx
                    | crate::scramble::WcaEvent::Skewb
                    | crate::scramble::WcaEvent::Megaminx
                    | crate::scramble::WcaEvent::Fto
                    | crate::scramble::WcaEvent::Square1
                    | crate::scramble::WcaEvent::Clock
            ) {
                assert!(text.contains("Preview is not available for"), "{event:?}");
                assert!(text.contains("this puzzle"), "{event:?}");
            } else {
                assert!(!text.contains("not available"), "{event:?}");
            }
            assert_eq!(
                buf[(area.right() - 2, area.bottom() - 3)].symbol(),
                "┘",
                "{event:?}"
            );
            if matches!(event, crate::scramble::WcaEvent::Cube7x7) {
                let left_border =
                    area.right() - 2 - ScramblePreviewWidget::new(event, model.scramble()).width()
                        + 1;
                assert_eq!(
                    buf[(left_border, area.bottom() - 3)].symbol(),
                    "└",
                    "{event:?}"
                );
            }
        }
    }

    #[test]
    fn text_prompt_renders_above_help_and_toasts() {
        let mut model = Model::new();
        model.toggle_help();
        model.toast_info("Background notification");
        model.request_text_input("Topmost prompt", "visible input", |_, _| {});
        let area = Rect::new(0, 0, 70, 9);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        view(area, &mut buf, &mut model);
        let mut expected = ratatui::buffer::Buffer::empty(area);
        model.text_input.as_ref().unwrap().input.render_with_theme(
            area,
            &mut expected,
            model.settings().theme(),
        );
        for y in 2..7 {
            for x in 3..67 {
                assert_eq!(buf[(x, y)], expected[(x, y)]);
            }
        }
    }

    fn inspecting_for(elapsed: Duration) -> Model {
        let mut model = Model::new();
        model.set_timer_state(TimerState::Inspection {
            time: Instant::now().checked_sub(elapsed).unwrap(),
            pulsed: false,
            first_audio_played: false,
            second_audio_played: false,
        });
        model
    }

    #[test]
    fn inspection_text_turns_red_at_dnf_limit() {
        let model = inspecting_for(Duration::from_secs(18));

        let (label, text, style) = timer_display(&model);

        assert_eq!(label, Some("Inspection"));
        assert!(text.starts_with("00:18."));
        assert_eq!(style.fg, Some(Color::Red));
    }

    #[test]
    fn inspection_text_stays_yellow_before_dnf_limit() {
        let model = inspecting_for(Duration::from_secs(14));

        let (_, _, style) = timer_display(&model);

        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn expired_inspection_displays_zero_in_red_until_reset() {
        let mut model = inspecting_for(Duration::from_secs(18));
        assert!(!model.start_timer());

        let (_, text, style) = timer_display(&model);
        assert_eq!(text, "00:00.000");
        assert_eq!(style.fg, Some(Color::Red));

        model.reset_timer();
        let (_, _, reset_style) = timer_display(&model);
        assert_eq!(reset_style.fg, Some(model.settings().theme().text()));
    }
}
