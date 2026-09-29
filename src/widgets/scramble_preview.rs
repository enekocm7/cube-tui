use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::model::settings::ThemeColors;
use crate::scramble::WcaEvent;
use crate::scramble::visualization::{Visualization, VisualizationError, visualize};

pub struct ScramblePreviewWidget {
    visualization: Result<Visualization, VisualizationError>,
}

impl ScramblePreviewWidget {
    pub fn new(event: WcaEvent, scramble: &str) -> Self {
        Self {
            visualization: visualize(event, scramble),
        }
    }

    pub fn width(&self) -> u16 {
        self.visualization
            .as_ref()
            .map_or(30, |net| net.width() + 2)
    }

    /// Leaves the complete stats table visible when the terminal is short.
    pub fn height_for(&self, available_height: u16) -> u16 {
        let desired = self
            .visualization
            .as_ref()
            .map_or(4, |net| net.height() + 2);
        if available_height >= desired {
            desired
        } else {
            available_height.min(3)
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeColors, toggle_key: &str) {
        let block = Block::default()
            .title(format!("Preview [{toggle_key}]"))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .style(Style::default().fg(theme.text()).bg(theme.background()));
        let inner = block.inner(area);
        match &self.visualization {
            Ok(net) if inner.width >= net.width() && inner.height >= net.height() => {
                let lines: Vec<_> = net
                    .cells
                    .iter()
                    .map(|row| {
                        Line::from(
                            row.iter()
                                .map(|cell| {
                                    Span::styled(
                                        cell.symbol.to_string(),
                                        cell.color.map_or_else(
                                            || {
                                                Style::default()
                                                    .fg(theme.text())
                                                    .bg(theme.background())
                                            },
                                            |color| {
                                                Style::default()
                                                    .fg(Color::Black)
                                                    .bg({
                                                        let [r, g, b] = color.rgb();
                                                        Color::Rgb(r, g, b)
                                                    })
                                                    .add_modifier(Modifier::BOLD)
                                            },
                                        ),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect();
                Paragraph::new(lines)
                    .block(block)
                    .alignment(Alignment::Center)
                    .render(area, buf);
            }
            Ok(_) => Paragraph::new("Resize to see preview")
                .block(block)
                .alignment(Alignment::Center)
                .render(area, buf),
            Err(error) => Paragraph::new(error.to_string())
                .block(block)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .render(area, buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_message_fits_the_preview_panel() {
        for event in [
            WcaEvent::Pyraminx,
            WcaEvent::Skewb,
            WcaEvent::Megaminx,
            WcaEvent::Fto,
            WcaEvent::Square1,
            WcaEvent::Clock,
        ] {
            let widget = ScramblePreviewWidget::new(event, "invalid scramble");
            let area = Rect::new(3, 5, widget.width(), widget.height_for(20));
            let mut buffer = Buffer::empty(area);
            widget.render(area, &mut buffer, &ThemeColors::default(), "v");
            let lines: Vec<_> = (area.y + 1..area.bottom() - 1)
                .map(|y| {
                    (area.x + 1..area.right() - 1)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                        .trim()
                        .to_owned()
                })
                .collect();
            assert_eq!(
                lines.join(" "),
                "Preview is not available for this puzzle",
                "{event:?}"
            );
        }
    }
}
