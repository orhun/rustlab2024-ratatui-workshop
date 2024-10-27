use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, Clear, Paragraph, Widget, Wrap},
};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::{Input, Key};

use crate::app::Event;

pub struct HelpPopup {
    key_bindings: String,
    event_sender: UnboundedSender<Event>,
}

impl HelpPopup {
    pub fn new(key_bindings: String, event_sender: UnboundedSender<Event>) -> Self {
        Self {
            key_bindings,
            event_sender,
        }
    }

    pub async fn handle_input(&mut self, input: Input) -> anyhow::Result<()> {
        if input.key == Key::Esc {
            let _ = self.event_sender.send(Event::PopupClosed);
        }
        Ok(())
    }
}

impl Widget for &mut HelpPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_area = popup_area(area, 30, 30);
        Clear.render(popup_area, buf);
        Paragraph::new(self.key_bindings.trim())
            .wrap(Wrap { trim: false })
            .block(
                Block::bordered()
                    .title("Help")
                    .title_style(Style::new().bold()),
            )
            .render(popup_area, buf);
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
