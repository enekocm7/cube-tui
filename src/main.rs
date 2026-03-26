use clap::{Parser, Subcommand};
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
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(feature = "bluetooth")]
pub mod bluetooth;
pub mod cstimer;
#[cfg(feature = "dashboard")]
mod dashboard;
mod model;
mod persistence;
mod scramble;
mod widgets;

use crate::model::{InspectionState, Model, TimerState};
#[cfg(feature = "bluetooth")]
use crate::widgets::bluetooth::BluetoothWidget;
use crate::widgets::detailed_stats::DetailedStatsWidget;
use crate::widgets::details::DetailsWidget;
use crate::widgets::help::HelpWidget;
use crate::widgets::mean_detail::MeanDetailWidget;
use crate::widgets::scramble::ScrambleWidget;
use crate::widgets::stats::StatsWidget;

fn main() {
    let cli = Cli::parse();

    match cli {
        Cli { data: true, .. } => {
            if let Some(dir) = persistence::data_dir() {
                println!("{}", dir.display());
            } else {
                eprintln!("Error: Could not determine data directory");
                std::process::exit(1);
            }
        }
        Cli {
            subcommand: Some(Command::Import { path }),
            ..
        } => {
            if !path.exists() {
                eprintln!("File does not exist: {}", path.display());
                std::process::exit(1);
            }
            match cstimer::import(&path) {
                Ok(histories) => {
                    let mut model = Model::new();
                    model.restore_from_history(histories);
                    persistence::save(&model);
                    println!("Imported successfully from: {}", path.display());
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("Import failed: {err}");
                    std::process::exit(1);
                }
            }
        }
        Cli {
            subcommand: Some(Command::Export { path }),
            ..
        } => {
            let histories = persistence::load().unwrap_or_default();
            let mut model = Model::new();
            model.restore_from_history(histories);
            match cstimer::export(&path, &model) {
                Ok(path) => {
                    println!("Exported successfully to: {}", path.display());
                }
                Err(err) => {
                    eprintln!("Export failed: {err}");
                }
            }
            std::process::exit(1);
        }
        #[cfg(feature = "dashboard")]
        Cli {
            subcommand: Some(Command::Dashboard { port }),
            ..
        } => {
            dashboard::run_dashboard(port);
        }
        _ => {
            #[cfg(feature = "wca-scrambles")]
            let _wca_scramble_server = match scramble::start_wca_scramble_server() {
                Ok(server) => Some(server),
                Err(error) => {
                    eprintln!(
                        "Warning: Could not enable WCA scrambles ({error}). Falling back to built-in random scrambles."
                    );
                    None
                }
            };

            ratatui::run(run);
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "cube", version, about = "A Rubik's Cube timer TUI application", long_about = None)]
struct Cli {
    #[arg(short, long, help = "Print the data directory and exit")]
    data: bool,
    #[command(subcommand)]
    subcommand: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(
        name = "import",
        alias = "i",
        about = "Imports a cstimer.json/cstimer.txt into the session history"
    )]
    Import {
        #[arg(value_name = "PATH", default_value = "cstimer.json")]
        path: PathBuf,
    },
    #[command(
        name = "export",
        alias = "e",
        about = "Exports the session history to a cstimer.json file"
    )]
    Export {
        #[arg(value_name = "PATH", default_value = "cstimer.json")]
        path: PathBuf,
    },
    #[cfg(feature = "dashboard")]
    #[command(
        name = "dashboard",
        alias = "d",
        about = "Starts a local dashboard for viewing data"
    )]
    Dashboard {
        #[arg(
            long,
            default_value_t = 7799,
            value_parser = clap::value_parser!(u16).range(1..=65535),
            help = "Port for the local dashboard server"
        )]
        port: u16,
    },
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
            .draw(|frame| view(frame.area(), frame.buffer_mut(), &mut model))
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
    OpenDetailedStats,
    DeleteTime,
    NavLeft,
    NavRight,
    ToggleFocus,
    #[cfg(feature = "bluetooth")]
    ToggleBluetooth,
    #[cfg(feature = "bluetooth")]
    DisconnectBluetooth,
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
        (KeyCode::Tab, KeyEventKind::Press) => Some(Msg::ToggleFocus),
        (KeyCode::Char('e'), KeyEventKind::Press) => Some(Msg::NextEvent),
        (KeyCode::Char('E'), KeyEventKind::Press) => Some(Msg::PrevEvent),
        (KeyCode::Char(']'), KeyEventKind::Press) => Some(Msg::NextSession),
        (KeyCode::Char('['), KeyEventKind::Press) => Some(Msg::PrevSession),
        (KeyCode::Char('s'), KeyEventKind::Press) => Some(Msg::NewSession),
        (KeyCode::Char('S'), KeyEventKind::Press) => Some(Msg::DeleteSession),
        (KeyCode::Char('n'), KeyEventKind::Press) => Some(Msg::NextScramble),
        (KeyCode::Char('?'), KeyEventKind::Press) => Some(Msg::Help),
        (KeyCode::Char('i'), KeyEventKind::Press) => Some(Msg::ToggleInspection),
        (KeyCode::Char('t'), KeyEventKind::Press) => Some(Msg::OpenDetailedStats),
        (KeyCode::Char('d'), KeyEventKind::Press) => Some(Msg::DeleteTime),
        #[cfg(feature = "bluetooth")]
        (KeyCode::Char('b'), KeyEventKind::Press) => Some(Msg::ToggleBluetooth),
        #[cfg(feature = "bluetooth")]
        (KeyCode::Char('x'), KeyEventKind::Press) => Some(Msg::DisconnectBluetooth),
        (KeyCode::Enter, KeyEventKind::Press) => Some(Msg::OpenDetails),
        (KeyCode::Esc, KeyEventKind::Press) => Some(Msg::CloseDetails),
        _ => None,
    }
}

const INSPECTION_LIMIT_MS: u64 = 15_000;

const fn allowed_msg(model: &Model, msg: Msg) -> bool {
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        return matches!(
            msg,
            Msg::SelectUp
                | Msg::SelectDown
                | Msg::OpenDetails
                | Msg::CloseDetails
                | Msg::ToggleBluetooth
                | Msg::DisconnectBluetooth
                | Msg::Tick
                | Msg::Quit
        );
    }
    if model.show_help() {
        return matches!(
            msg,
            Msg::SelectUp | Msg::SelectDown | Msg::Help | Msg::Tick | Msg::Quit
        );
    }
    if model.show_details() {
        return matches!(
            msg,
            Msg::SelectUp
                | Msg::SelectDown
                | Msg::NavLeft
                | Msg::NavRight
                | Msg::Press
                | Msg::Release
                | Msg::DeleteTime
                | Msg::CloseDetails
                | Msg::Tick
                | Msg::Quit
        );
    }
    if model.show_mean_detail() {
        return matches!(
            msg,
            Msg::SelectUp
                | Msg::SelectDown
                | Msg::OpenDetails
                | Msg::CloseDetails
                | Msg::Tick
                | Msg::Quit
        );
    }
    if model.show_detailed_stats() {
        return matches!(
            msg,
            Msg::SelectUp
                | Msg::SelectDown
                | Msg::NavLeft
                | Msg::NavRight
                | Msg::OpenDetails
                | Msg::CloseDetails
                | Msg::Tick
                | Msg::Quit
        );
    }
    true
}

fn update(model: &mut Model, msg: Msg) {
    if matches!(msg, Msg::Tick) {
        #[cfg(feature = "bluetooth")]
        {
            if model.show_bluetooth() {
                model.poll_bluetooth();
            }
            if model.bluetooth_timer_active() {
                model.poll_bluetooth_timer();
            }
        }
    }

    if !allowed_msg(model, msg) {
        return;
    }

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
        Msg::OpenDetailedStats => handle_open_detailed_stats(model),
        Msg::DeleteTime => handle_delete_time(model),
        Msg::NavLeft => handle_nav_left(model),
        Msg::NavRight => handle_nav_right(model),
        Msg::ToggleFocus => handle_toggle_focus(model),
        #[cfg(feature = "bluetooth")]
        Msg::ToggleBluetooth => handle_toggle_bluetooth(model),
        #[cfg(feature = "bluetooth")]
        Msg::DisconnectBluetooth => handle_disconnect_bluetooth(model),
        Msg::Quit => {}
    }
}

fn handle_press(model: &mut Model) {
    #[cfg(feature = "bluetooth")]
    if model.bluetooth_connected() {
        return;
    }
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
    #[cfg(feature = "bluetooth")]
    if model.bluetooth_connected() {
        return;
    }
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
    if model.show_help() {
        model.scroll_help_up();
        return;
    }
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        model.bluetooth_select_up();
        return;
    }
    if model.show_mean_detail() {
        model.mean_detail_select_up();
    } else if model.show_detailed_stats() {
        model.detailed_stats_select_up();
    } else if model.show_details() {
        model.prev_details_modifier();
    } else if model.main_focus_is_stats() {
        model.main_stats_select_up();
    } else {
        model.history_mut().select_previous();
    }
}

fn handle_select_down(model: &mut Model) {
    if model.show_help() {
        model.scroll_help_down();
        return;
    }
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        model.bluetooth_select_down();
        return;
    }
    if model.show_mean_detail() {
        model.mean_detail_select_down();
    } else if model.show_detailed_stats() {
        model.detailed_stats_select_down();
    } else if model.show_details() {
        model.next_details_modifier();
    } else if model.main_focus_is_stats() {
        model.main_stats_select_down();
    } else {
        model.history_mut().select_next();
    }
}

const fn handle_toggle_focus(model: &mut Model) {
    if model.show_help() || model.show_details() || model.show_detailed_stats() {
        return;
    }
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        return;
    }
    model.toggle_main_focus();
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

#[cfg(feature = "bluetooth")]
fn handle_toggle_bluetooth(model: &mut Model) {
    if model.show_help() || model.show_details() || model.show_detailed_stats() {
        return;
    }

    if let Some(tx) = model.toggle_bluetooth() {
        use crate::bluetooth::timer::{get_adapter, get_devices};
        use crate::model::BluetoothEvent;
        use futures_util::StreamExt;

        std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Runtime::new() else {
                let _ = tx.send(BluetoothEvent::Error("Failed to create runtime".into()));
                return;
            };

            runtime.block_on(async move {
                let adapter = match get_adapter().await {
                    Ok(adapter) => adapter,
                    Err(err) => {
                        let _ = tx.send(BluetoothEvent::Error(err.to_string()));
                        return;
                    }
                };

                let _ = tx.send(BluetoothEvent::Adapter(adapter.clone()));
                let _ = tx.send(BluetoothEvent::Status("Scanning for devices...".into()));

                let mut stream = match get_devices(&adapter).await {
                    Ok(stream) => stream,
                    Err(err) => {
                        let _ = tx.send(BluetoothEvent::Error(err.to_string()));
                        return;
                    }
                };

                while let Some(device) = stream.next().await {
                    if tx.send(BluetoothEvent::Device(device)).is_err() {
                        break;
                    }
                }
            });
        });
    }
}

#[cfg(feature = "bluetooth")]
fn handle_disconnect_bluetooth(model: &mut Model) {
    if (model.bluetooth_connected() || model.bluetooth_connecting())
        && let Some((tx, rx, adapter)) = model.disconnect_bluetooth()
    {
        restart_bluetooth_scan(tx, rx, adapter);
    }
}

#[cfg(feature = "bluetooth")]
fn restart_bluetooth_scan(
    tx: flume::Sender<crate::model::BluetoothEvent>,
    _rx: flume::Receiver<crate::model::BluetoothEvent>,
    adapter: btleplug::platform::Adapter,
) {
    use crate::bluetooth::timer::get_devices;
    use futures_util::StreamExt;

    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Runtime::new() else {
            return;
        };

        runtime.block_on(async move {
            let Ok(mut stream) = get_devices(&adapter).await else {
                return;
            };

            while let Some(device) = stream.next().await {
                if tx
                    .send(crate::model::BluetoothEvent::Device(device))
                    .is_err()
                {
                    break;
                }
            }
        });
    });
}

#[cfg(feature = "bluetooth")]
fn handle_bluetooth_connect(model: &mut Model) {
    use crate::bluetooth::timer::{TimerState as BtTimerState, connect, disconnect};
    use futures_util::StreamExt;

    let Some(device) = model.bluetooth_selected_device().cloned() else {
        return;
    };

    let Some((tx, cancel_rx, adapter, conn_tx)) = model.connect_bluetooth_device() else {
        return;
    };

    let device_id = device.id;
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Runtime::new() else {
            let _ = tx.send(BtTimerState::Disconnected);
            return;
        };

        runtime.block_on(async move {
            let mut stream = match connect(&device_id, &adapter).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(BtTimerState::Error(e.to_string()));
                    let _ = tx.send(BtTimerState::Disconnected);
                    return;
                }
            };

            let _ = conn_tx.send(());

            loop {
                tokio::select! {
                    state = stream.next() => match state {
                        Some(state) => { if tx.send(state).is_err() { break; } }
                        None => break,
                    },
                    _ = cancel_rx.recv_async() => break,
                }
            }

            let _ = disconnect(&device_id, &adapter).await;
            let _ = tx.send(BtTimerState::Disconnected);
        });
    });
}

fn handle_toggle_inspection(model: &mut Model) {
    model.toggle_inspection();
    persistence::save_settings(*model.settings());
}

fn handle_open_details(model: &mut Model) {
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        if model.bluetooth_connected() {
            handle_disconnect_bluetooth(model);
        } else {
            handle_bluetooth_connect(model);
        }
        return;
    }
    if model.show_mean_detail() {
        model.open_details_for_selected_mean_time();
        return;
    }
    if model.show_detailed_stats() && !model.show_mean_detail() {
        model.open_mean_detail();
        return;
    }
    if model.main_focus_is_stats() {
        model.open_mean_detail_from_stats();
        return;
    }
    if model.timer_state() == TimerState::Idle && !model.history().is_empty() {
        model.open_details();
    }
}

fn handle_open_detailed_stats(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && !model.history().is_empty() {
        model.open_detailed_stats();
    }
}

#[allow(clippy::missing_const_for_fn)]
fn handle_close_details(model: &mut Model) {
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        model.close_bluetooth();
        return;
    }
    if model.show_details() && model.can_return_to_mean_detail() {
        model.return_to_mean_detail();
    } else if model.show_mean_detail() {
        model.close_mean_detail();
    } else if model.show_detailed_stats() {
        model.close_detailed_stats();
    } else {
        model.close_details();
    }
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
    if model.show_detailed_stats() && !model.show_mean_detail() {
        model.detailed_stats_col_left();
    } else if model.main_focus_is_stats() {
        model.main_stats_col_left();
    } else if model.show_details() {
        model.details_nav_prev();
    }
}

fn handle_nav_right(model: &mut Model) {
    if model.show_detailed_stats() && !model.show_mean_detail() {
        model.detailed_stats_col_right();
    } else if model.main_focus_is_stats() {
        model.main_stats_col_right();
    } else if model.show_details() {
        model.details_nav_next();
    }
}

#[allow(clippy::too_many_lines)]
fn view(area: Rect, buf: &mut ratatui::buffer::Buffer, model: &mut Model) {
    if model.show_help() {
        model.set_help_max_scroll(HelpWidget::max_scroll_for_height(area.height));
        HelpWidget::new(model.help_scroll()).render(area, buf);
        return;
    }

    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        use crate::model::BluetoothScreenState;
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(area);

        BluetoothWidget::new(
            model.bluetooth_devices().to_vec(),
            model.bluetooth_selected_index(),
            model.bluetooth_status().map(str::to_string),
            model.connected_device_id(),
        )
        .render(layout[0], buf);

        let help_text = match model.bluetooth_screen_state() {
            BluetoothScreenState::Connected => Line::from(vec![
                Span::raw("↑/↓: select  "),
                Span::raw("Enter/x: disconnect  "),
                Span::raw("Esc: back to timer"),
            ]),
            BluetoothScreenState::Connecting => Line::from(vec![
                Span::raw("↑/↓: select device  "),
                Span::raw("Esc: back to timer"),
            ]),
            BluetoothScreenState::Searching => Line::from(vec![
                Span::raw("↑/↓: select device  "),
                Span::raw("Enter: connect  "),
                Span::raw("Esc: close"),
            ]),
        };
        Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .render(layout[1], buf);
        return;
    }

    if model.show_detailed_stats() && model.show_mean_detail() {
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
        widget.render(layout[0], buf);

        let help_text = Line::from(vec![
            Span::raw("↑/↓: select time  "),
            Span::raw("Enter: open details  "),
            Span::raw("Esc: back to stats"),
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
        .render(layout[0], buf);

        let help_text = Line::from(vec![
            Span::raw("↑/↓: navigate  "),
            Span::raw("←/→: mo3/ao5  "),
            Span::raw("Enter: view mean  "),
            Span::raw("Esc: back"),
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
        .render(details_layout[0], buf);

        let esc_label = if model.can_return_to_mean_detail() {
            "Esc: back to mean"
        } else {
            "Esc: close"
        };
        let details_help = Line::from(vec![
            Span::raw("Space: toggle modifier  "),
            Span::raw("↑/↓: select modifier  "),
            Span::raw("←/→: navigate times  "),
            Span::raw("d: delete  "),
            Span::raw(esc_label),
        ]);
        Paragraph::new(details_help)
            .alignment(Alignment::Center)
            .render(details_layout[1], buf);
        return;
    }

    let scramble_lines = get_scramble_lines(model.scramble().as_str(), area.width);

    let scramble_height = (scramble_lines + 2).min(area.height.saturating_sub(1));
    let constraints = (Constraint::Length(scramble_height), Constraint::Fill(1));

    let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([constraints.0, constraints.1, Constraint::Length(1)])
        .margin(1)
        .split(area);

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(24),
                Constraint::Min(10),
                Constraint::Length(30),
            ]
            .as_ref(),
        )
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
    if model.main_focus_is_stats() {
        model
            .history()
            .clone()
            .without_selection_highlight()
            .render(history_area, buf);
    } else {
        model.history().clone().render(history_area, buf);
    }

    #[cfg(feature = "bluetooth")]
    let bt_label = model
        .connected_device_name()
        .map_or_else(String::new, |name| format!(" | 🔗 {name}"));
    #[cfg(not(feature = "bluetooth"))]
    let bt_label = String::new();
    let timer_title = format!(
        "Timer - Inspection: {}{bt_label}",
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

    let stats_widget = if model.main_focus_is_stats() {
        StatsWidget::new(model.history().clone())
            .with_selection(model.main_stats_row(), model.main_stats_col())
    } else {
        StatsWidget::new(model.history().clone())
    };
    stats_widget.render(main_layout[2], buf);

    let help_text = Line::from(vec![
        Span::raw("Space: hold/release  "),
        Span::raw("Enter: details  "),
        Span::raw("Tab: history/stats  "),
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

fn get_scramble_lines(scramble: &str, width: u16) -> u16 {
    //10 is the padding (5 on each side) so the max chars are width - 10
    let chars_per_line = width as usize - 10;
    let num_lines = scramble.len().div_ceil(chars_per_line);
    num_lines as u16
}
