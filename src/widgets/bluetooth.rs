use crate::bluetooth::DeviceInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct BluetoothWidget {
    devices: Vec<DeviceInfo>,
    selected_index: usize,
    status: Option<String>,
    connected_device_id: Option<btleplug::platform::PeripheralId>,
}

impl BluetoothWidget {
    pub const fn new(
        devices: Vec<DeviceInfo>,
        selected_index: usize,
        status: Option<String>,
        connected_device_id: Option<btleplug::platform::PeripheralId>,
    ) -> Self {
        Self {
            devices,
            selected_index,
            status,
            connected_device_id,
        }
    }
}

impl Widget for BluetoothWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::default()
            .title("Bluetooth Devices")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL);

        let mut lines: Vec<Line> = Vec::new();
        if let Some(ref status) = self.status {
            let color = if status.contains("✓") {
                Color::Green
            } else if status.contains("⚠") || status.contains("Error") {
                Color::Red
            } else {
                Color::Yellow
            };
            lines.push(Line::from(Span::styled(
                status.clone(),
                Style::default().fg(color),
            )));
            lines.push(Line::from(""));
        }

        if self.devices.is_empty() {
            lines.push(Line::from(Span::raw("No devices found")));
        } else {
            lines.extend(self.devices.into_iter().enumerate().map(|(index, device)| {
                let name = device.name.unwrap_or_else(|| "(unknown)".to_string());
                let is_connected = self
                    .connected_device_id
                    .as_ref()
                    .is_some_and(|id| *id == device.id);
                let rssi = device
                    .rssi
                    .map_or_else(|| "? dBm".to_string(), |value| format!("{value} dBm"));
                let prefix = if index == self.selected_index {
                    "> "
                } else {
                    "  "
                };
                let line = format!("{prefix}{name}  [{rssi}]");
                if index == self.selected_index {
                    Line::from(Span::styled(
                        line,
                        Style::default()
                            .fg(if is_connected {
                                Color::Green
                            } else {
                                Color::Cyan
                            })
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if is_connected {
                    Line::from(Span::styled(line, Style::default().fg(Color::Green)))
                } else {
                    Line::from(Span::raw(line))
                }
            }));
        }

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
