#[cfg(feature = "bluetooth")]
use futures_util::StreamExt;

#[cfg(feature = "bluetooth")]
use crate::model::bluetooth::BluetoothEvent;
use crate::model::confirmation::ConfirmationAction;
use crate::model::{InspectionState, Model, TimerState};
use crate::msg::{INSPECTION_LIMIT_MS, Msg, allowed_msg};
use crate::persistence;
#[cfg(feature = "bluetooth")]
use crate::utils::runtime::runtime;

/// Applies a message to the model and reports whether the UI should redraw.
///
/// Accepted non-tick messages always request a redraw. Tick messages request
/// one only when timer state or Bluetooth-backed state actually changes.
pub fn update(model: &mut Model, msg: Msg) -> bool {
    #[cfg(feature = "bluetooth")]
    let background_changed = matches!(msg, Msg::Tick) && {
        let scan_changed = model.show_bluetooth() && model.poll_bluetooth();
        let timer_changed = model.bluetooth_timer_active() && model.poll_bluetooth_timer();
        scan_changed || timer_changed
    };
    #[cfg(not(feature = "bluetooth"))]
    let background_changed = false;

    if !allowed_msg(model, msg) {
        return background_changed;
    }

    let timer_state = model.timer_state();
    msg.apply(model);
    background_changed || !matches!(msg, Msg::Tick) || model.timer_state() != timer_state
}

impl Msg {
    /// Dispatches this message to its state-transition handler.
    fn apply(self, model: &mut Model) {
        match self {
            Self::Press => handle_press(model),
            Self::Release => handle_release(model),
            Self::Reset => handle_reset(model),
            Self::Tick => handle_tick(model),
            Self::SelectUp => handle_select_up(model),
            Self::SelectDown => handle_select_down(model),
            Self::NextEventOpenEditor => handle_next_event_open_editor(model),
            Self::PrevEvent => handle_prev_event(model),
            Self::NextSession => handle_next_session(model),
            Self::PrevSession => handle_prev_session(model),
            Self::NewSession => handle_new_session(model),
            Self::DeleteSession => handle_delete_session(model),
            Self::NextScramble => handle_next_scramble(model),
            Self::Help => handle_help(model),
            Self::ToggleInspection => handle_toggle_inspection(model),
            Self::Enter => handle_enter(model),
            Self::Esc => handle_esc(model),
            Self::OpenDetailedStats => handle_open_detailed_stats(model),
            Self::OpenThemeSelector => handle_open_theme_selector(model),
            Self::DeleteTime => handle_delete_time(model),
            Self::NavLeft => handle_nav_left(model),
            Self::NavRight => handle_nav_right(model),
            Self::ToggleFocus => handle_toggle_focus(model),
            #[cfg(feature = "bluetooth")]
            Self::ToggleBluetooth => handle_toggle_bluetooth(model),
            #[cfg(feature = "bluetooth")]
            Self::DisconnectBluetooth => handle_disconnect_bluetooth(model),
            Self::ToggleZen => handle_toggle_zen(model),
            Self::Quit => {}
        }
    }
}

/// Handles the initial timer-key press for the active timer state.
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

/// Handles release of the timer key, starting or stopping as appropriate.
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

/// Resets the timer and advances to a fresh scramble.
fn handle_reset(model: &mut Model) {
    model.reset_timer();
}

/// Advances time-dependent and asynchronous model state by one UI tick.
fn handle_tick(model: &mut Model) {
    if let TimerState::Inspection(InspectionState::Running(start)) = model.timer_state() {
        let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap();
        if elapsed_ms >= INSPECTION_LIMIT_MS {
            model.set_last_time_ms(INSPECTION_LIMIT_MS);
            model.set_timer_state(TimerState::Inspection(InspectionState::Pulsed(start)));
        }
    }
}

/// Moves the active screen's selection upward.
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

    if model.theme_selector.is_some() {
        model.theme_selector_up();
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

/// Moves the active screen's selection downward.
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

    if model.theme_selector.is_some() {
        model.theme_selector_down();
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

/// Switches keyboard focus between the history and statistics panes.
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

/// Advances the puzzle event, opening settings when event editing is active.
fn handle_next_event_open_editor(model: &mut Model) {
    if model.theme_selector.is_some() {
        model.open_theme_in_editor();
        return;
    }

    if model.timer_state() == TimerState::Idle {
        model.next_event();
    }
}

/// Selects the previous puzzle event when the main timer is active.
fn handle_prev_event(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.prev_event();
    }
}

/// Activates the next session and prepares it for display.
fn handle_next_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.next_session();
        if model.current_session().scramble.is_none() {
            model.next_scramble();
        }
    }
}

/// Activates the previous session and prepares it for display.
fn handle_prev_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.prev_session();
        if model.current_session().scramble.is_none() {
            model.next_scramble();
        }
    }
}

/// Creates and activates a session when the configured limit permits it.
fn handle_new_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.add_session();
        model.next_scramble();
        persistence::save(model);
    }
}

/// Opens confirmation before deleting the active session.
fn handle_delete_session(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && model.session_count() > 1 {
        model.open_confirmation(ConfirmationAction::DeleteSession);
    }
}

/// Replaces the current scramble while the main timer is idle.
fn handle_next_scramble(model: &mut Model) {
    if model.timer_state() == TimerState::Idle {
        model.next_scramble();
    }
}

/// Shows or hides the keyboard-help overlay.
const fn handle_help(model: &mut Model) {
    model.toggle_help();
}

#[cfg(feature = "bluetooth")]
/// Opens or closes the Bluetooth device picker and starts discovery.
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
/// Requests disconnection from the active Bluetooth timer.
fn handle_disconnect_bluetooth(model: &mut Model) {
    if (model.bluetooth_connected() || model.bluetooth_connecting())
        && let Some((tx, rx, adapter)) = model.disconnect_bluetooth()
    {
        restart_bluetooth_scan(tx, rx, adapter);
    }
}

#[cfg(feature = "bluetooth")]
/// Launches a scanner task and forwards its status through `sender`.
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
/// Starts a connection to the selected Bluetooth timer.
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

/// Toggles inspection timing and persists the new setting.
fn handle_toggle_inspection(model: &mut Model) {
    model.toggle_inspection();
    persistence::save_config(model.settings());
}

/// Toggles zen mode and persists the new setting.
fn handle_toggle_zen(model: &mut Model) {
    model.toggle_zen();
    persistence::save_config(model.settings());
}

/// Confirms the context-sensitive action for the active screen.
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
    if let Some(confirmation) = model.confirmation() {
        let confirmed = confirmation.selection == crate::widgets::confirmation::Selection::Yes;
        let action = confirmation.action;
        model.close_confirmation();

        if confirmed {
            match action {
                ConfirmationAction::DeleteSession => {
                    if model.delete_current_session() {
                        if model.current_session().scramble.is_none() {
                            model.next_scramble();
                        }
                        persistence::save(model);
                    }
                }
                ConfirmationAction::DeleteTime => {
                    model.history_mut().delete_selected();
                    persistence::save(model);
                    if model.show_details() && model.history().is_empty() {
                        model.close_current_screen();
                    }
                }
            }
        }
        return;
    }
    if model.theme_selector.is_some() {
        model.close_theme_selector();
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

/// Opens the detailed statistics screen for the active session.
fn handle_open_detailed_stats(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && !model.history().is_empty() {
        model.open_detailed_stats();
    }
}

/// Opens the theme selector and initializes its current selection.
fn handle_open_theme_selector(model: &mut Model) {
    if model.theme_selector.is_some() {
        model.close_theme_selector();
    } else {
        model.open_theme_selector();
    }
}

#[allow(clippy::missing_const_for_fn)]
/// Closes the topmost modal or returns to the main screen.
fn handle_esc(model: &mut Model) {
    #[cfg(feature = "bluetooth")]
    if model.show_bluetooth() {
        model.close_bluetooth();
        return;
    }
    if model.show_help() {
        model.toggle_help();
        return;
    }
    if model.confirmation().is_some() {
        model.close_confirmation();
        return;
    }
    if model.theme_selector.is_some() {
        model.close_theme_selector();
        return;
    }
    model.close_current_screen();
}

/// Opens confirmation before deleting the selected solve.
fn handle_delete_time(model: &mut Model) {
    if model.timer_state() == TimerState::Idle && !model.history().is_empty() {
        model.open_confirmation(ConfirmationAction::DeleteTime);
    }
}

/// Moves selection left or performs the screen-specific previous action.
fn handle_nav_left(model: &mut Model) {
    if model.confirmation().is_some() {
        model.confirmation_selection_left();
    } else if model.show_detailed_stats() && !model.show_mean_detail() {
        model.detailed_stats_col_left();
    } else if model.main_focus_is_stats() {
        model.main_stats_col_left();
    } else if model.show_details() {
        model.details_nav_prev();
    }
}

/// Moves selection right or performs the screen-specific next action.
fn handle_nav_right(model: &mut Model) {
    if model.confirmation().is_some() {
        model.confirmation_selection_right();
    } else if model.show_detailed_stats() && !model.show_mean_detail() {
        model.detailed_stats_col_right();
    } else if model.main_focus_is_stats() {
        model.main_stats_col_right();
    } else if model.show_details() {
        model.details_nav_next();
    }
}
