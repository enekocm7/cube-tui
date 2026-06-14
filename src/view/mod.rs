use std::borrow::Cow;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::model::{InspectionState, Model, TimerState};
use crate::utils::{format_elapsed, get_scramble_lines};
use crate::widgets::detailed_stats::DetailedStatsWidget;
use crate::widgets::details::DetailsWidget;
use crate::widgets::help::HelpWidget;
use crate::widgets::mean_detail::MeanDetailWidget;
use crate::widgets::scramble::ScrambleWidget;
use crate::widgets::stats::StatsWidget;

#[cfg(feature = "bluetooth")]
use crate::widgets::bluetooth::BluetoothWidget;

#[allow(clippy::too_many_lines)]
pub(crate) fn view(area: Rect, buf: &mut ratatui::buffer::Buffer, model: &mut Model) {
    let theme = *model.settings().theme();
    set_area_background(area, buf, theme.background());
    if model.show_help() {
        let help_widget = HelpWidget::new(model.help_scroll());
        model.set_help_max_scroll(HelpWidget::max_scroll_for_height(area.height));
        help_widget.render_with_theme(area, buf, &theme);
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
        widget.render_with_theme(layout[0], buf, &theme);

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
            model.history().clone(),
            model.detailed_stats_row(),
            model.detailed_stats_col(),
        )
        .render_with_theme(layout[0], buf, &theme);

        let help_text = Line::from(vec![
            Span::styled("↑/↓: navigate  ", Style::default().fg(theme.text())),
            Span::styled("←/→: mo3/ao5  ", Style::default().fg(theme.text())),
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
            Span::styled("Esc: close", Style::default().fg(theme.text())),
        ]);
        Paragraph::new(details_help)
            .alignment(Alignment::Center)
            .render(details_layout[1], buf);
        return;
    }

    if model.zen_enabled() && matches!(model.timer_state(), TimerState::Running(_)) {
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

    let main_area_index = if show_scramble { 1 } else { 0 };
    let help_area_index = if show_scramble { 2 } else { 1 };

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
        main_constraints.push(Constraint::Length(30));
    }

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(main_constraints)
        .split(outer_layout[main_area_index]);

    if show_scramble {
        ScrambleWidget::new(model.scramble(), model.event().name()).render_with_theme(
            outer_layout[0],
            buf,
            &theme,
        );
    }

    let history_title = format!(
        "Session: {:02}/{:02}{}",
        model.current_session_index() + 1,
        model.session_count(),
        if model.is_at_max_sessions() {
            " (max 99)"
        } else {
            ""
        }
    );
    if let Some(index) = history_area_index {
        let history_block = Block::default()
            .title(history_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()));
        history_block.render(main_layout[index], buf);
        let history_area = inner_area(main_layout[index]);
        if model.main_focus_is_stats() {
            model
                .history()
                .clone()
                .without_selection_highlight()
                .render_with_theme(history_area, buf, &theme);
        } else {
            model
                .history()
                .clone()
                .render_with_theme(history_area, buf, &theme);
        }
    }

    #[cfg(feature = "bluetooth")]
    let bt_label = model
        .connected_device_name()
        .map_or_else(String::new, |name| format!(" | 🔗 {name}"));
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
    let (timer_text, timer_style) = timer_display(model);
    Paragraph::new(Line::from(Span::styled(timer_text, timer_style)))
        .block(timer_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .render(main_layout[timer_area_index], buf);

    if let Some(index) = stats_area_index {
        let stats_widget = if model.main_focus_is_stats() {
            StatsWidget::new(model.history().clone())
                .with_selection(model.main_stats_row(), model.main_stats_col())
        } else {
            StatsWidget::new(model.history().clone())
        };
        stats_widget.render_with_theme(main_layout[index], buf, &theme);
    }

    let mut help_spans = vec![
        Span::styled("Space: hold/release  ", Style::default().fg(theme.text())),
        Span::styled("Enter: details  ", Style::default().fg(theme.text())),
        Span::styled("r: reset  ", Style::default().fg(theme.text())),
        Span::styled("q: quit  ", Style::default().fg(theme.text())),
        Span::styled("?: help", Style::default().fg(theme.text())),
    ];
    if show_history && show_stats {
        help_spans.insert(
            2,
            Span::styled("Tab: history/stats  ", Style::default().fg(theme.text())),
        );
    }
    Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .render(outer_layout[help_area_index], buf);
}

fn set_area_background(area: Rect, buf: &mut ratatui::buffer::Buffer, color: Color) {
    let style = Style::default().bg(color);
    let spaces = " ".repeat(area.width as usize);
    for y in area.top()..area.bottom() {
        buf.set_string(area.x, y, &spaces, style);
    }
}

const fn inner_area(area: Rect) -> Rect {
    Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn timer_display(model: &Model) -> (Cow<'static, str>, Style) {
    let theme = model.settings().theme();
    let style = match model.timer_state() {
        TimerState::Idle => Style::default().fg(theme.text()),
        TimerState::Pulsed | TimerState::Inspection(InspectionState::Pulsed(_)) => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
        TimerState::Running(_) => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        TimerState::Inspection(InspectionState::Running(_)) => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    };

    let text = match model.timer_state() {
        TimerState::Pulsed => format_elapsed(0),
        TimerState::Inspection(_) => {
            let elapsed_ms = model.elapsed_ms();
            let remaining_ms = 15_000_u64.saturating_sub(elapsed_ms);
            Cow::Owned(format!("Inspect: {}", format_elapsed(remaining_ms)))
        }
        _ => format_elapsed(model.elapsed_ms()),
    };

    (text, style)
}
