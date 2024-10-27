use ratatui::{
    style::{Color, Style},
    widgets::Widget,
};
use tokio::sync::mpsc::UnboundedSender;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerSmartWidget, TuiWidgetEvent, TuiWidgetState};
use tui_textarea::{Input, Key};

use crate::app::Event;

pub struct Logger {
    pub state: TuiWidgetState,
    pub event_sender: UnboundedSender<Event>,
}

impl Logger {
    pub fn new(event_sender: UnboundedSender<Event>) -> Self {
        Self {
            state: TuiWidgetState::default(),
            event_sender,
        }
    }

    pub async fn handle_input(&mut self, input: Input) -> anyhow::Result<()> {
        // TODO: print log about pressed key
        match (input.ctrl, input.key) {
            (true, Key::Char('l')) => {
                let _ = self.event_sender.send(Event::LoggerClosed);
            }
            (false, Key::Char(' ')) => self.state.transition(TuiWidgetEvent::SpaceKey),
            (false, Key::Esc) => self.state.transition(TuiWidgetEvent::EscapeKey),
            (false, Key::PageUp) => self.state.transition(TuiWidgetEvent::PrevPageKey),
            (false, Key::PageDown) => self.state.transition(TuiWidgetEvent::NextPageKey),
            (false, Key::Up) => self.state.transition(TuiWidgetEvent::UpKey),
            (false, Key::Down) => self.state.transition(TuiWidgetEvent::DownKey),
            (false, Key::Left) => self.state.transition(TuiWidgetEvent::LeftKey),
            (false, Key::Right) => self.state.transition(TuiWidgetEvent::RightKey),
            (false, Key::Char('+')) => self.state.transition(TuiWidgetEvent::PlusKey),
            (false, Key::Char('-')) => self.state.transition(TuiWidgetEvent::MinusKey),
            (false, Key::Char('h')) => self.state.transition(TuiWidgetEvent::HideKey),
            (false, Key::Char('f')) => self.state.transition(TuiWidgetEvent::FocusKey),
            _ => {}
        }
        Ok(())
    }
}

impl Widget for &Logger {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // TODO: render TuiLoggerSmartWidget: <https://docs.rs/tui-logger/latest/tui_logger/struct.TuiLoggerSmartWidget.html>
    }
}
