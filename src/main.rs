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
mod scramble;
mod widgets;

use crate::model::{InspectionState, Model, TimerState};
use crate::scramble::WcaEvent;
use crate::widgets::scramble::ScrambleWidget;

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
    let tick_rate = Duration::from_millis(30);
    let mut last_tick = Instant::now();

    loop {
        if last_tick.elapsed() >= tick_rate {
            update(&mut model, Msg::Tick);
            last_tick = Instant::now();
        }

        if event::poll(Duration::from_millis(10)).unwrap_or(false)
            && let Ok(Event::Key(key)) = event::read()
            && let Some(msg) = map_key_to_msg(key.code, key.kind)
        {
            if matches!(msg, Msg::Quit) {
                execute!(stdout, PopKeyboardEnhancementFlags).ok();
                return;
            }
            update(&mut model, msg);
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
    NextEvent,
    PrevEvent,
    NextSession,
    PrevSession,
    NewSession,
}

const fn map_key_to_msg(code: KeyCode, kind: KeyEventKind) -> Option<Msg> {
    match (code, kind) {
        (KeyCode::Char('q'), KeyEventKind::Press) => Some(Msg::Quit),
        (KeyCode::Char('r'), KeyEventKind::Press) => Some(Msg::Reset),
        (KeyCode::Char(' '), KeyEventKind::Press) => Some(Msg::Press),
        (KeyCode::Char(' '), KeyEventKind::Release) => Some(Msg::Release),
        (KeyCode::Up, KeyEventKind::Press) => Some(Msg::SelectUp),
        (KeyCode::Down, KeyEventKind::Press) => Some(Msg::SelectDown),
        (KeyCode::Char('e'), KeyEventKind::Press) => Some(Msg::NextEvent),
        (KeyCode::Char('E'), KeyEventKind::Press) => Some(Msg::PrevEvent),
        (KeyCode::Char(']'), KeyEventKind::Press) => Some(Msg::NextSession),
        (KeyCode::Char('['), KeyEventKind::Press) => Some(Msg::PrevSession),
        (KeyCode::Char('n'), KeyEventKind::Press) => Some(Msg::NewSession),
        _ => None,
    }
}

fn update(model: &mut Model, msg: Msg) {
    const INSPECTION_LIMIT_MS: u64 = 15_000;

    match msg {
        Msg::Press => match model.timer_state() {
            TimerState::Idle => model.start_inspection(),
            TimerState::Inspection(InspectionState::Running(_)) => model.pulse_timer(),
            TimerState::Inspection(InspectionState::Pulsed(_)) => {}
            TimerState::Running(start) => {
                let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap();
                model.set_last_time_ms(elapsed_ms);
                model.history_mut().add_ms(elapsed_ms);
                model.stop_timer();
                model.next_scramble();
            }
        },
        Msg::Release => {
            if let TimerState::Inspection(InspectionState::Pulsed(_)) = model.timer_state() {
                model.start_timer();
            }
        }
        Msg::Reset => {
            model.reset_timer();
        }
        Msg::Tick => {
            if let TimerState::Inspection(InspectionState::Running(start)) = model.timer_state() {
                let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap();
                if elapsed_ms >= INSPECTION_LIMIT_MS {
                    model.set_last_time_ms(INSPECTION_LIMIT_MS);
                    model.set_timer_state(TimerState::Inspection(InspectionState::Pulsed(start)));
                }
            }
        }
        Msg::SelectUp => model.history_mut().select_previous(),
        Msg::SelectDown => model.history_mut().select_next(),
        Msg::NextEvent => model.next_event(),
        Msg::PrevEvent => model.prev_event(),
        Msg::NextSession => model.next_session(),
        Msg::PrevSession => model.prev_session(),
        Msg::NewSession => {
            model.add_session();
        }
        Msg::Quit => {}
    }
}

fn view(area: Rect, buf: &mut ratatui::buffer::Buffer, model: &Model) {
    let constraints = match model.event() {
        WcaEvent::Cube2x2
        | WcaEvent::Pyraminx
        | WcaEvent::Skewb
        | WcaEvent::Clock
        | WcaEvent::Cube3x3 => (Constraint::Percentage(12), Constraint::Percentage(88)),
        WcaEvent::Cube4x4 | WcaEvent::Square1 | WcaEvent::Cube5x5 => {
            (Constraint::Percentage(16), Constraint::Percentage(84))
        }
        WcaEvent::Cube6x6 | WcaEvent::Megaminx => {
            (Constraint::Percentage(20), Constraint::Percentage(80))
        }
        WcaEvent::Cube7x7 => (Constraint::Percentage(25), Constraint::Percentage(75)),
    };

    let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([constraints.0, constraints.1, Constraint::Length(1)])
        .margin(1)
        .split(area);

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(10)].as_ref())
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

    let timer_block = Block::default().title("Timer").borders(Borders::ALL);
    let (timer_text, timer_style) = timer_display(model);
    Paragraph::new(Line::from(Span::styled(timer_text, timer_style)))
        .block(timer_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .render(main_layout[1], buf);

    let help_text = Line::from(vec![
        Span::raw("Space: hold/release  "),
        Span::raw("r: reset  "),
        Span::raw("q: quit  "),
        Span::raw("e/E: event  "),
        Span::raw("n: new session  "),
        Span::raw("[/]: prev/next session  "),
        Span::raw("Up/Down: select"),
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
        TimerState::Running(_) => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        TimerState::Inspection(InspectionState::Running(_)) => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        TimerState::Inspection(InspectionState::Pulsed(_)) => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
    };

    let text = match model.timer_state() {
        TimerState::Inspection(_) => {
            let elapsed_ms = model.elapsed_ms();
            let remaining_ms = 15_000_u64.saturating_sub(elapsed_ms);
            format!("Inspect: {}", format_elapsed(remaining_ms))
        }
        _ => format_elapsed(model.elapsed_ms()),
    };

    (text, style)
}
