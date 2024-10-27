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
        // TODO: handle the popup input
        Ok(())
    }
}

impl Widget for &mut HelpPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // TODO: render a popup with the key bindings
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
