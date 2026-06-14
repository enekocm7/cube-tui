use crate::bluetooth::DeviceInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::model::settings::ThemeSettings;

pub struct BluetoothWidget<'a> {
    devices: Vec<DeviceInfo>,
    selected_index: usize,
    status: Option<&'a str>,
    connected_device_id: Option<btleplug::platform::PeripheralId>,
}

impl<'a> BluetoothWidget<'a> {
    pub const fn new(
        devices: Vec<DeviceInfo>,
        selected_index: usize,
        status: Option<&'a str>,
        connected_device_id: Option<btleplug::platform::PeripheralId>,
    ) -> Self {
        Self {
            devices,
            selected_index,
            status,
            connected_device_id,
        }
    }

    pub fn render_with_theme(self, area: Rect, buf: &mut Buffer, theme: &ThemeSettings) {
        let block = Block::default()
            .title("Bluetooth Devices")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()));

        let mut lines: Vec<Line> = Vec::new();
        if let Some(status) = self.status {
            let color = if status.contains("✓") {
                Color::Green
            } else if status.contains("⚠") || status.contains("Error") {
                Color::Red
            } else {
                Color::Yellow
            };
            lines.push(Line::from(Span::styled(status, Style::default().fg(color))));
            lines.push(Line::from(""));
        }

        if self.devices.is_empty() {
            lines.push(Line::from(Span::styled(
                "No devices found",
                Style::default().fg(theme.text()),
            )));
        } else {
            lines.extend(self.devices.into_iter().enumerate().map(|(index, device)| {
                let name = device.name.unwrap_or_else(|| "(unknown)".to_string());
                let is_connected = self
                    .connected_device_id
                    .as_ref()
                    .is_some_and(|id| *id == device.id);
                let prefix = if index == self.selected_index {
                    "> "
                } else {
                    "  "
                };
                let line = format!("{prefix}{name}");
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
                    Line::from(Span::styled(line, Style::default().fg(theme.text())))
                }
            }));
        }

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
