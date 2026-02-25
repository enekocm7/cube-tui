use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

mod model;
mod persistence;
mod scramble;
mod widgets;

use crate::model::{InspectionState, Model, TimerState};
use crate::scramble::WcaEvent;
use crate::widgets::details::DetailsWidget;
use crate::widgets::help::HelpWidget;
use crate::widgets::scramble::ScrambleWidget;
use crate::widgets::stats::StatsWidget;

fn main() {
    ratatui::run(run);
}

fn run(terminal: &mut DefaultTerminal) {
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )
    .ok();

    let mut model = Model::new();
    if let Some(data) = persistence::load() {
        model.restore_from_history(data);
    }
    if let Some(settings) = persistence::load_settings() {
        model.set_settings(settings);
    }
    let tick_rate = Duration::from_millis(30);
    let mut last_tick = Instant::now();

    loop {
        if last_tick.elapsed() >= tick_rate {
            update(&mut model, Msg::Tick);
            last_tick = Instant::now();
        }

        if event::poll(Duration::from_millis(10)).unwrap_or(false)
            && let Ok(Event::Key(key)) = event::read()
        {
            let msg = map_key_to_msg(key.code, key.kind);

            if let Some(msg) = msg {
                if matches!(msg, Msg::Quit) {
                    execute!(stdout, PopKeyboardEnhancementFlags).ok();
                    return;
                }
                update(&mut model, msg);
            }
        }

        terminal
            .draw(|frame| view(frame.area(), frame.buffer_mut(), &model))
            .ok();
    }
}

#[derive(Copy, Clone, Debug)]
enum Msg {
    Press,
    Release,
    Reset,
    Tick,
    SelectUp,
    SelectDown,
    Quit,
    Help,
    NextEvent,
    PrevEvent,
    NextSession,
    PrevSession,
    NewSession,
    DeleteSession,
    ToggleInspection,
    NextScramble,
    OpenDetails,
    CloseDetails,
    DeleteTime,
    NavLeft,
    NavRight,
}

const fn map_key_to_msg(code: KeyCode, kind: KeyEventKind) -> Option<Msg> {
    match (code, kind) {
        (KeyCode::Char('q'), KeyEventKind::Press) => Some(Msg::Quit),
        (KeyCode::Char('r'), KeyEventKind::Press) => Some(Msg::Reset),
        (KeyCode::Char(' '), KeyEventKind::Press) => Some(Msg::Press),
        (KeyCode::Char(' '), KeyEventKind::Release) => Some(Msg::Release),
        (KeyCode::Up, KeyEventKind::Press) => Some(Msg::SelectUp),
        (KeyCode::Down, KeyEventKind::Press) => Some(Msg::SelectDown),
        (KeyCode::Left, KeyEventKind::Press) => Some(Msg::NavLeft),
        (KeyCode::Right, KeyEventKind::Press) => Some(Msg::NavRight),
        (KeyCode::Char('e'), KeyEventKind::Press) => Some(Msg::NextEvent),
        (KeyCode::Char('E'), KeyEventKind::Press) => Some(Msg::PrevEvent),
        (KeyCode::Char(']'), KeyEventKind::Press) => Some(Msg::NextSession),
        (KeyCode::Char('['), KeyEventKind::Press) => Some(Msg::PrevSession),
        (KeyCode::Char('s'), KeyEventKind::Press) => Some(Msg::NewSession),
        (KeyCode::Char('S'), KeyEventKind::Press) => Some(Msg::DeleteSession),
        (KeyCode::Char('n'), KeyEventKind::Press) => Some(Msg::NextScramble),
        (KeyCode::Char('?'), KeyEventKind::Press) => Some(Msg::Help),
        (KeyCode::Char('i'), KeyEventKind::Press) => Some(Msg::ToggleInspection),
        (KeyCode::Char('d'), KeyEventKind::Press) => Some(Msg::DeleteTime),
        (KeyCode::Enter, KeyEventKind::Press) => Some(Msg::OpenDetails),
        (KeyCode::Esc, KeyEventKind::Press) => Some(Msg::CloseDetails),
        _ => None,
    }
}

const INSPECTION_LIMIT_MS: u64 = 15_000;

fn update(model: &mut Model, msg: Msg) {
    match msg {
        Msg::Press => handle_press(model),
        Msg::Release => handle_release(model),
        Msg::Reset => handle_reset(model),
        Msg::Tick => handle_tick(model),
        Msg::SelectUp => handle_select_up(model),
        Msg::SelectDown => handle_select_down(model),
        Msg::NextEvent => handle_next_event(model),
        Msg::PrevEvent => handle_prev_event(model),
        Msg::NextSession => handle_next_session(model),
        Msg::PrevSession => handle_prev_session(model),
        Msg::NewSession => handle_new_session(model),
        Msg::DeleteSession => handle_delete_session(model),
        Msg::NextScramble => handle_next_scramble(model),
        Msg::Help => handle_help(model),
        Msg::ToggleInspection => handle_toggle_inspection(model),
        Msg::OpenDetails => handle_open_details(model),
        Msg::CloseDetails => handle_close_details(model),
        Msg::DeleteTime => handle_delete_time(model),
        Msg::NavLeft => handle_nav_left(model),
        Msg::NavRight => handle_nav_right(model),
        Msg::Quit => {}
    }
}

fn handle_press(model: &mut Model) {
    if model.show_details() {
        if model.timer_state() == TimerState::Idle {
            let modifier = model.selected_details_modifier();
            model.history_mut().set_modifier(modifier);
            persistence::save(model);
        }
        return;
    }

    match model.timer_state() {
        TimerState::Idle => {
            if model.inspection_enabled() {
                model.start_inspection();
            } else {
                model.set_timer_state(TimerState::Pulsed);
            }
        }
        TimerState::Pulsed | TimerState::Inspection(InspectionState::Pulsed(_)) => {}
        TimerState::Inspection(InspectionState::Running(_)) => model.pulse_timer(),
        TimerState::Running(start) => {
            let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap();
            model.set_last_time_ms(elapsed_ms);
            let event = model.event();
            let scramble = model.scramble().as_str().to_string();
            model.history_mut().add_ms(elapsed_ms, event, scramble);
            model.stop_timer();
            model.next_scramble();
            persistence::save(model);
        }
    }
}

fn handle_release(model: &mut Model) {
    if model.show_details() {
        return;
    }
    if matches!(
        model.timer_state(),
        TimerState::Pulsed | TimerState::Inspection(InspectionState::Pulsed(_))
    ) {
        model.start_timer();
    }
}

fn handle_reset(model: &mut Model) {
    model.reset_timer();
}

fn handle_tick(model: &mut Model) {
    if let TimerState::Inspection(InspectionState::Running(start)) = model.timer_state() {
        let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap();
        if elapsed_ms >= INSPECTION_LIMIT_MS {
            model.set_last_time_ms(INSPECTION_LIMIT_MS);
            model.set_timer_state(TimerState::Inspection(InspectionState::Pulsed(start)));
        }
    }
}

fn handle_select_up(model: &mut Model) {
    if model.show_details() {
        model.prev_details_modifier();
    } else {
        model.history_mut().select_previous();
    }
}

fn handle_select_down(model: &mut Model) {
    if model.show_details() {
        model.next_details_modifier();
    } else {
        model.history_mut().select_next();
    }
}

fn handle_next_event(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.next_event();
    }
}

fn handle_prev_event(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.prev_event();
    }
}

fn handle_next_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.next_session();
    }
}

fn handle_prev_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.prev_session();
    }
}

fn handle_new_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.add_session();
        persistence::save(model);
    }
}

fn handle_delete_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && model.delete_current_session() {
        persistence::save(model);
    }
}

fn handle_next_scramble(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.next_scramble();
    }
}

const fn handle_help(model: &mut Model) {
    model.toggle_help();
}

fn handle_toggle_inspection(model: &mut Model) {
    model.toggle_inspection();
    persistence::save_settings(*model.settings());
}

fn handle_open_details(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && !model.history().is_empty() {
        model.open_details();
    }
}

const fn handle_close_details(model: &mut Model) {
    model.close_details();
}

fn handle_delete_time(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && !model.history().is_empty() {
        model.history_mut().delete_selected();
        persistence::save(model);
        if model.show_details() && model.history().is_empty() {
            model.close_details();
        }
    }
}

fn handle_nav_left(model: &mut Model) {
    if model.show_details() {
        model.details_nav_prev();
    }
}

fn handle_nav_right(model: &mut Model) {
    if model.show_details() {
        model.details_nav_next();
    }
}

fn view(area: Rect, buf: &mut ratatui::buffer::Buffer, model: &Model) {
    if model.show_help() {
        HelpWidget.render(area, buf);
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
        .render(details_layout[0], buf);

        let details_help = Line::from(vec![
            Span::raw("Space: toggle modifier  "),
            Span::raw("↑/↓: select modifier  "),
            Span::raw("←/→: navigate times  "),
            Span::raw("d: delete  "),
            Span::raw("Esc: close"),
        ]);
        Paragraph::new(details_help)
            .alignment(Alignment::Center)
            .render(details_layout[1], buf);
        return;
    }

    let scramble_lines: u16 = match model.event() {
        WcaEvent::Cube2x2
        | WcaEvent::Pyraminx
        | WcaEvent::Skewb
        | WcaEvent::Clock
        | WcaEvent::Cube4x4
        | WcaEvent::Square1
        | WcaEvent::Cube3x3 => 1,
        WcaEvent::Cube5x5 | WcaEvent::Cube6x6 => 2,
        WcaEvent::Cube7x7 => 3,
        WcaEvent::Megaminx => 7,
    };

    let scramble_height = (scramble_lines + 2).min(area.height.saturating_sub(1));
    let constraints = (Constraint::Length(scramble_height), Constraint::Fill(1));

    let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([constraints.0, constraints.1, Constraint::Length(1)])
        .margin(1)
        .split(area);

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(10), Constraint::Length(30)].as_ref())
        .split(outer_layout[1]);

    ScrambleWidget::new(model.scramble().as_str(), model.event().name())
        .render(outer_layout[0], buf);

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
    let history_block = Block::default().title(history_title).borders(Borders::ALL);
    history_block.render(main_layout[0], buf);
    let history_area = inner_area(main_layout[0]);
    model.history().clone().render(history_area, buf);

    let timer_title = format!(
        "Timer - Inspection: {}",
        if model.inspection_enabled() {
            "On"
        } else {
            "Off"
        }
    );
    let timer_block = Block::default().title(timer_title).borders(Borders::ALL);
    let (timer_text, timer_style) = timer_display(model);
    Paragraph::new(Line::from(Span::styled(timer_text, timer_style)))
        .block(timer_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .render(main_layout[1], buf);

    StatsWidget::new(model.history().clone()).render(main_layout[2], buf);

    let help_text = Line::from(vec![
        Span::raw("Space: hold/release  "),
        Span::raw("Enter: details  "),
        Span::raw("r: reset  "),
        Span::raw("q: quit  "),
        Span::raw("?: help"),
    ]);
    Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .render(outer_layout[2], buf);
}

const fn inner_area(area: Rect) -> Rect {
    Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn format_elapsed(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;
    format!("{minutes:02}:{seconds:02}.{millis:03}")
}

fn timer_display(model: &Model) -> (String, Style) {
    let style = match model.timer_state() {
        TimerState::Idle => Style::default().fg(Color::White),
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
            format!("Inspect: {}", format_elapsed(remaining_ms))
        }
        _ => format_elapsed(model.elapsed_ms()),
    };

    (text, style)
}
