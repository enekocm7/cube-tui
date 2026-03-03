use btleplug::platform::{Adapter, PeripheralId};
use btleplug::{
    api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter},
    platform::Manager,
};
use futures_util::{Stream, StreamExt};
use flume;
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
pub async fn get_devices(adapter: &Adapter) -> anyhow::Result<impl Stream<Item = DeviceInfo>> {
    adapter.start_scan(ScanFilter::default()).await?;

    let (tx, rx) = flume::bounded(32);
    let adapter = adapter.clone();

    tokio::spawn(async move {
        let Ok(mut events) = adapter.events().await else {
            return;
        };

        while let Some(event) = events.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                    if let Ok(peripheral) = adapter.peripheral(&id).await {
                        let props = peripheral.properties().await.unwrap_or(None);
                        let device = DeviceInfo {
                            id,
                            name: props.as_ref().and_then(|p| p.local_name.clone()),
                            rssi: props.and_then(|p| p.rssi),
                        };
                        let is_gan = device
                            .name
                            .as_ref()
                            .is_some_and(|n| n.to_lowercase().contains("gan"));

                        if is_gan && tx.send_async(device).await.is_err() {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
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
    peripheral.discover_services().await?;

    let characteristics = peripheral.characteristics();

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
            match event.value[3] {
                1 => {
                    if tx.send_async(TimerState::GetSet).await.is_err() {
                        break;
                    }
                }
                2 => {
                    if tx.send_async(TimerState::HandsOff).await.is_err() {
                        break;
                    }
                }
                3 => {
                    if tx.send_async(TimerState::Running).await.is_err() {
                        break;
                    }
                }
                5 => {
                    if tx.send_async(TimerState::Idle).await.is_err() {
                        break;
                    }
                }
                6 => {
                    if tx.send_async(TimerState::HandsOn).await.is_err() {
                        break;
                    }
                }
                7 => {
                    if let Ok(time) = peripheral.read(&time_characteristic).await
                        && let Ok(bytes) = <[u8; 4]>::try_from(&time[0..4])
                    {
                        let time_ms = time_array_to_ms(bytes);
                        if tx.send_async(TimerState::Finished(time_ms)).await.is_err() {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
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
