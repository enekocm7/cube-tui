use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

mod model;
mod scramble;
mod widgets;

use crate::model::{InspectionState, Model, TimerState};
use crate::widgets::scramble::Scramble;

fn main() {
    ratatui::run(run);
}

fn run(terminal: &mut DefaultTerminal) {
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
}

const fn map_key_to_msg(code: KeyCode, kind: KeyEventKind) -> Option<Msg> {
    match (code, kind) {
        (KeyCode::Char('q'), KeyEventKind::Press) => Some(Msg::Quit),
        (KeyCode::Char('r'), KeyEventKind::Press) => Some(Msg::Reset),
        (KeyCode::Char(' '), KeyEventKind::Press) => Some(Msg::Press),
        (KeyCode::Char(' '), KeyEventKind::Release) => Some(Msg::Release),
        (KeyCode::Up, KeyEventKind::Press) => Some(Msg::SelectUp),
        (KeyCode::Down, KeyEventKind::Press) => Some(Msg::SelectDown),
        _ => None,
    }
}

fn update(model: &mut Model, msg: Msg) {
    const INSPECTION_LIMIT_MS: u64 = 15_000;

    match msg {
        Msg::Press => match model.timer_state {
            TimerState::Idle => model.start_inspection(),
            TimerState::Inspection(InspectionState::Running(_)) => model.pulse_timer(),
            TimerState::Inspection(InspectionState::Pulsed(_)) => {}
            TimerState::Running(start) => {
                let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap();
                model.last_time_ms = elapsed_ms;
                model.history.add_ms(elapsed_ms);
                model.stop_timer();
                model.next_scramble();
            }
        },
        Msg::Release => {
            if let TimerState::Inspection(
                InspectionState::Pulsed(_) | InspectionState::Running(_),
            ) = model.timer_state
            {
                model.start_timer();
            }
        }
        Msg::Reset => {
            model.reset_timer();
        }
        Msg::Tick => {
            if let TimerState::Inspection(InspectionState::Running(start)) = model.timer_state {
                let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap();
                if elapsed_ms >= INSPECTION_LIMIT_MS {
                    model.last_time_ms = INSPECTION_LIMIT_MS;
                    model.timer_state = TimerState::Inspection(InspectionState::Pulsed(start));
                }
            }
        }
        Msg::SelectUp => model.history.select_previous(),
        Msg::SelectDown => model.history.select_next(),
        Msg::Quit => {}
    }
}

fn view(area: Rect, buf: &mut ratatui::buffer::Buffer, model: &Model) {
    let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .margin(1)
        .split(area);

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(10)].as_ref())
        .split(outer_layout[1]);

    Scramble::new(model.scramble).render(outer_layout[0], buf);

    let history_block = Block::default().title("History").borders(Borders::ALL);
    history_block.render(main_layout[0], buf);
    let history_area = inner_area(main_layout[0]);
    model.history.clone().render(history_area, buf);

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
    let style = match model.timer_state {
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

    let text = match model.timer_state {
        TimerState::Inspection(_) => {
            let elapsed_ms = model.elapsed_ms();
            let remaining_ms = 15_000_u64.saturating_sub(elapsed_ms);
            format!("Inspect: {}", format_elapsed(remaining_ms))
        }
        _ => format_elapsed(model.elapsed_ms()),
    };

    (text, style)
}
