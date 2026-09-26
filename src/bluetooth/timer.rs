use std::time::Duration;

use btleplug::platform::{Adapter, PeripheralId};
use btleplug::{
    api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter},
    platform::Manager,
};
use flume;
use futures_util::{Stream, StreamExt};
use uuid::Uuid;

pub use super::{BtTimerState as TimerState, DeviceInfo};

const GAN_TIMER_SERVICE: &str = "0000fff0-0000-1000-8000-00805f9b34fb";
const GAN_TIMER_TIME_CHARACTERISTIC: &str = "0000fff2-0000-1000-8000-00805f9b34fb";
const GAN_TIMER_STATE_CHARACTERISTIC: &str = "0000fff5-0000-1000-8000-00805f9b34fb";

/// Returns the first available Bluetooth adapter.
///
/// # Errors
/// - If no Bluetooth adapters are found.
/// - If the system Bluetooth manager cannot be queried.
pub async fn get_adapter() -> anyhow::Result<Adapter> {
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        return Err(anyhow::anyhow!("No Bluetooth adapters found"));
    }
    Ok(adapter_list[0].clone())
}

/// Starts a BLE scan and returns a stream of discovered/updated devices.
///
/// The stream yields lightweight `DeviceInfo` items suitable for populating a UI list.
/// Consumers can resolve the selected `id` into a `Peripheral` when the user chooses one.
///
/// # Errors
/// - If scanning cannot be started.
/// - If adapter events cannot be subscribed to.
pub async fn get_devices(
    adapter: &Adapter,
) -> anyhow::Result<impl Stream<Item = anyhow::Result<DeviceInfo>>> {
    adapter.start_scan(ScanFilter::default()).await?;
    let mut events = adapter.events().await?;
    let (tx, rx) = flume::bounded(32);
    let adapter = adapter.clone();
    tokio::spawn(async move {
        while let Some(event) = events.next().await {
            let (CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id)) = event
            else {
                continue;
            };
            let peripheral = match adapter.peripheral(&id).await {
                Ok(peripheral) => peripheral,
                Err(error) => {
                    if tx
                        .send_async(Err(anyhow::anyhow!(
                            "Could not inspect a Bluetooth device: {error}"
                        )))
                        .await
                        .is_err()
                    {
                        return;
                    }
                    continue;
                }
            };
            let props = match peripheral.properties().await {
                Ok(props) => props,
                Err(error) => {
                    if tx
                        .send_async(Err(anyhow::anyhow!(
                            "Could not read Bluetooth device properties: {error}"
                        )))
                        .await
                        .is_err()
                    {
                        return;
                    }
                    continue;
                }
            };
            let name = props.and_then(|props| props.local_name);
            if name
                .as_ref()
                .is_some_and(|name| name.to_lowercase().contains("gan"))
            {
                let device = DeviceInfo { id, name };
                if tx.send_async(Ok(device)).await.is_err() {
                    return; // The picker was closed or a connection was started.
                }
            }
        }
        let _ = tx
            .send_async(Err(anyhow::anyhow!(
                "Bluetooth discovery stopped unexpectedly"
            )))
            .await;
    });
    Ok(rx.into_stream())
}

/// Connects to a GAN timer peripheral and returns a stream of [`TimerState`] updates.
///
/// Resolves the peripheral by its ID, establishes a BLE connection, discovers services,
/// and subscribes to the state characteristic. State change notifications are mapped to
/// [`TimerState`] variants and streamed back to the caller. When the timer reports
/// `Finished` (byte `7`), the time characteristic is read to extract the solution time.
///
/// # Errors
/// - If the peripheral cannot be resolved, connected, or service discovery fails.
/// - If the required state or time characteristics are not found.
/// - If subscribing to notifications fails.
///
/// # Panics
/// - If the constants of the service are incorrect uuid (should never fail)
///
pub async fn connect(
    id: &PeripheralId,
    adapter: &Adapter,
) -> anyhow::Result<impl Stream<Item = TimerState>> {
    let timer_service_uuid =
        Uuid::parse_str(GAN_TIMER_SERVICE).expect("The constant is a parseable uuid");
    let state_uuid =
        Uuid::parse_str(GAN_TIMER_STATE_CHARACTERISTIC).expect("The constant is a parseable uuid");
    let time_uuid =
        Uuid::parse_str(GAN_TIMER_TIME_CHARACTERISTIC).expect("The constant is a parseable uuid");

    let peripheral = adapter.peripheral(id).await?;
    peripheral.connect().await?;
    adapter.stop_scan().await?;
    while !peripheral.is_connected().await? {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let mut retries = 5;
    while let Err(err) = peripheral.discover_services().await {
        if retries == 0 {
            return Err(err.into());
        }
        retries -= 1;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let mut characteristics = peripheral.characteristics();
    let mut char_retries = 10;
    while characteristics.is_empty() && char_retries > 0 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        characteristics = peripheral.characteristics();
        char_retries -= 1;
    }

    let state_characteristic = characteristics
        .iter()
        .find(|ch| ch.service_uuid == timer_service_uuid && ch.uuid == state_uuid)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("State characteristic not found"))?;

    let time_characteristic = characteristics
        .iter()
        .find(|ch| ch.service_uuid == timer_service_uuid && ch.uuid == time_uuid)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Time characteristic not found"))?;

    let mut notifications = peripheral.notifications().await?;
    peripheral.subscribe(&state_characteristic).await?;

    let (tx, rx) = flume::bounded(32);

    tokio::spawn(async move {
        while let Some(event) = notifications.next().await {
            if event.uuid != state_uuid {
                continue;
            }
            let Some(&state_byte) = event.value.get(3) else {
                let _ = tx
                    .send_async(TimerState::Warning(
                        "Received an incomplete Bluetooth timer update".into(),
                    ))
                    .await;
                continue;
            };
            let state = match state_byte {
                1 => TimerState::GetSet,
                2 => TimerState::HandsOff,
                3 => TimerState::Running,
                5 => TimerState::Idle,
                6 => TimerState::HandsOn,
                7 => match peripheral.read(&time_characteristic).await {
                    Ok(time) => match parse_time(&time) {
                        Some(time_ms) => TimerState::Finished(time_ms),
                        None => TimerState::Error(
                            "The timer returned an incomplete solve time. The solve was not saved."
                                .into(),
                        ),
                    },
                    Err(error) => TimerState::Error(
                        format!(
                            "Could not read the finished solve: {error}. The solve was not saved."
                        )
                        .into(),
                    ),
                },
                _ => {
                    let _ = tx
                        .send_async(TimerState::Warning(
                            format!("Received an unsupported Bluetooth timer state: {state_byte}")
                                .into(),
                        ))
                        .await;
                    continue;
                }
            };
            if tx.send_async(state).await.is_err() {
                return; // The connection was closed by the user.
            }
        }
        let _ = tx.send_async(TimerState::Disconnected).await;
    });

    Ok(rx.into_stream())
}

/// Disconnects from a BLE peripheral by its ID.
///
/// Resolves the peripheral from the adapter and issues a BLE disconnect.
/// Should be called when the user closes the app or manually disconnects.
///
/// # Errors
/// - If the peripheral cannot be resolved or the disconnect fails.
pub async fn disconnect(id: &PeripheralId, adapter: &Adapter) -> anyhow::Result<()> {
    let peripheral = adapter.peripheral(id).await?;
    peripheral.disconnect().await?;
    Ok(())
}

/// Converts a 4-byte time array from the GAN timer into milliseconds.
///
/// Format: `[minutes, seconds, ms_low, ms_high]` where the last two bytes
/// are a little-endian `u16` representing the millisecond fraction.
fn time_array_to_ms(t: [u8; 4]) -> u64 {
    (u64::from(t[0]) * 60_000)
        + (u64::from(t[1]) * 1_000)
        + u64::from(u16::from_le_bytes([t[2], t[3]]))
}

/// Validates the time payload before converting it to milliseconds.
fn parse_time(bytes: &[u8]) -> Option<u64> {
    let time: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
    Some(time_array_to_ms(time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_timer_payloads_are_rejected_without_panicking() {
        for bytes in [vec![], vec![1], vec![1, 2], vec![1, 2, 3]] {
            assert_eq!(parse_time(&bytes), None);
        }
        assert_eq!(parse_time(&[1, 2, 232, 3]), Some(63_000));
    }
}
