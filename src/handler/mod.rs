#[cfg(feature = "bluetooth")]
use futures_util::StreamExt;

#[cfg(feature = "bluetooth")]
use crate::model::bluetooth::BluetoothEvent;
use crate::model::{InspectionState, Model, TimerState};
use crate::msg::{INSPECTION_LIMIT_MS, Msg, allowed_msg};
use crate::persistence;
#[cfg(feature = "bluetooth")]
use crate::utils::runtime::runtime;

pub(crate) fn update(model: &mut Model, msg: Msg) {
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

    msg.apply(model);
}

impl Msg {
    fn apply(self, model: &mut Model) {
        match self {
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
            Msg::Enter => handle_enter(model),
            Msg::Esc => handle_esc(model),
            Msg::OpenDetailedStats => handle_open_detailed_stats(model),
            Msg::DeleteTime => handle_delete_time(model),
            Msg::NavLeft => handle_nav_left(model),
            Msg::NavRight => handle_nav_right(model),
            Msg::ToggleFocus => handle_toggle_focus(model),
            #[cfg(feature = "bluetooth")]
            Msg::ToggleBluetooth => handle_toggle_bluetooth(model),
            #[cfg(feature = "bluetooth")]
            Msg::DisconnectBluetooth => handle_disconnect_bluetooth(model),
            Msg::ToggleZen => handle_toggle_zen(model),
            Msg::Quit => {}
        }
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

    #[cfg(feature = "bluetooth")]
    if model.bluetooth_connected() {
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
            model.record_solve(elapsed_ms);
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

fn handle_toggle_focus(model: &mut Model) {
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
        if model.current_session().scramble.is_none() {
            model.next_scramble();
        }
    }
}

fn handle_prev_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.prev_session();
        if model.current_session().scramble.is_none() {
            model.next_scramble();
        }
    }
}

fn handle_new_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.add_session();
        model.next_scramble();
        persistence::save(model);
    }
}

fn handle_delete_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && model.session_count() > 1 {
        model.open_confirm_delete_session();
    }
}

fn handle_next_scramble(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.next_scramble();
    }
}

fn handle_help(model: &mut Model) {
    model.toggle_help();
}

#[cfg(feature = "bluetooth")]
fn handle_toggle_bluetooth(model: &mut Model) {
    if model.show_help() || model.show_details() || model.show_detailed_stats() {
        return;
    }

    if let Some(tx) = model.toggle_bluetooth() {
        use std::borrow::Cow;

        use crate::bluetooth::timer::{get_adapter, get_devices};

        runtime().spawn(async move {
            let adapter = match get_adapter().await {
                Ok(adapter) => adapter,
                Err(err) => {
                    let _ = tx.send(BluetoothEvent::Error(Cow::Owned(err.to_string())));
                    return;
                }
            };

            let _ = tx.send(BluetoothEvent::Adapter(adapter.clone()));
            let _ = tx.send(BluetoothEvent::Status("Scanning for devices...".into()));

            let mut stream = match get_devices(&adapter).await {
                Ok(stream) => stream,
                Err(err) => {
                    let _ = tx.send(BluetoothEvent::Error(Cow::Owned(err.to_string())));
                    return;
                }
            };

            while let Some(device) = stream.next().await {
                if tx.send(BluetoothEvent::Device(device)).is_err() {
                    break;
                }
            }
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
    tx: flume::Sender<BluetoothEvent>,
    _rx: flume::Receiver<BluetoothEvent>,
    adapter: btleplug::platform::Adapter,
) {
    use crate::bluetooth::timer::get_devices;

    runtime().spawn(async move {
        let Ok(mut stream) = get_devices(&adapter).await else {
            return;
        };

        while let Some(device) = stream.next().await {
            if tx.send(BluetoothEvent::Device(device)).is_err() {
                break;
            }
        }
    });
}

#[cfg(feature = "bluetooth")]
fn handle_bluetooth_connect(model: &mut Model) {
    use std::borrow::Cow;

    use crate::bluetooth::timer::{TimerState as BtTimerState, connect, disconnect};

    let Some(device) = model.bluetooth_selected_device().cloned() else {
        return;
    };

    let Some((tx, adapter, conn_tx)) = model.connect_bluetooth_device() else {
        return;
    };

    let device_id = device.id;
    runtime().spawn(async move {
        let mut stream = match connect(&device_id, &adapter).await {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(BtTimerState::Error(Cow::Owned(e.to_string())));
                let _ = tx.send(BtTimerState::Disconnected);
                return;
            }
        };

        let _ = conn_tx.send(());

        loop {
            if let Some(state) = stream.next().await
                && tx.send(state).is_err()
            {
                break;
            }
        }

        let _ = disconnect(&device_id, &adapter).await;
        let _ = tx.send(BtTimerState::Disconnected);
    });
}

fn handle_toggle_inspection(model: &mut Model) {
    model.toggle_inspection();
    persistence::save_config(*model.settings());
}

fn handle_toggle_zen(model: &mut Model) {
    model.toggle_zen();
    persistence::save_config(*model.settings());
}

fn handle_enter(model: &mut Model) {
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        if model.bluetooth_connected() {
            handle_disconnect_bluetooth(model);
        } else {
            handle_bluetooth_connect(model);
        }
        return;
    }
    if model.screen.show_confirm_delete_session() {
        let confirmed = model.get_confirm_delete_session_selection()
            == crate::widgets::confirmation::Selection::Yes;

        if confirmed && model.delete_current_session() {
            if model.current_session().scramble.is_none() {
                model.next_scramble();
            }
            persistence::save(model);
        }

        model.close_confirm_delete_session();
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
fn handle_esc(model: &mut Model) {
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        model.close_bluetooth();
        return;
    }
    if model.screen.show_confirm_delete_session() {
        model.close_confirm_delete_session();
        return;
    }
    model.close_current_screen();
}

fn handle_delete_time(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && !model.history().is_empty() {
        model.history_mut().delete_selected();
        persistence::save(model);
        if model.show_details() && model.history().is_empty() {
            model.close_current_screen();
        }
    }
}

fn handle_nav_left(model: &mut Model) {
    if model.screen.show_confirm_delete_session() {
        model.confirm_delete_session_left();
    } else if model.show_detailed_stats() && !model.show_mean_detail() {
        model.detailed_stats_col_left();
    } else if model.main_focus_is_stats() {
        model.main_stats_col_left();
    } else if model.show_details() {
        model.details_nav_prev();
    }
}

fn handle_nav_right(model: &mut Model) {
    if model.screen.show_confirm_delete_session() {
        model.confirm_delete_session_right();
    } else if model.show_detailed_stats() && !model.show_mean_detail() {
        model.detailed_stats_col_right();
    } else if model.main_focus_is_stats() {
        model.main_stats_col_right();
    } else if model.show_details() {
        model.details_nav_next();
    }
}
