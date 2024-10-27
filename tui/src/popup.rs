use std::io;

use crossterm::event::Event as CrosstermEvent;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, BorderType, Clear, Paragraph, Widget, Wrap},
};
use ratatui_explorer::{FileExplorer, Theme};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::{Input, Key};

use crate::app::Event;

pub enum Popup {
    Help(String, UnboundedSender<Event>),
    FileExplorer(FileExplorer, UnboundedSender<Event>),
}

impl Popup {
    pub fn help(key_bindings: String, event_sender: UnboundedSender<Event>) -> Self {
        Self::Help(key_bindings, event_sender)
    }

    pub fn file_explorer(event_sender: UnboundedSender<Event>) -> io::Result<Self> {
        let theme = Theme::default()
            .add_default_title()
            .with_title_bottom(|fe| format!("[ {} files ]", fe.files().len()).into())
            .with_style(Color::Yellow)
            .with_highlight_item_style(Modifier::BOLD)
            .with_highlight_dir_style(Style::new().blue().bold())
            .with_highlight_symbol("> ")
            .with_block(Block::bordered().border_type(BorderType::Rounded));
        let file_explorer = FileExplorer::with_theme(theme)?;
        Ok(Self::FileExplorer(file_explorer, event_sender))
    }

    pub async fn handle_input(
        &mut self,
        input: Input,
        raw_event: CrosstermEvent,
    ) -> anyhow::Result<()> {
        match self {
            Popup::Help(_, ref event_sender) if input.key == Key::Esc => {
                let _ = event_sender.send(Event::PopupClosed);
            }
            Popup::FileExplorer(ref mut explorer, ref mut event_sender) => match input.key {
                Key::Esc => {
                    let _ = event_sender.send(Event::PopupClosed);
                }
                Key::Enter => {
                    let file = explorer.current().clone();
                    if file.is_dir() {
                        return Ok(());
                    }
                    let event = Event::FileSelected(file);
                    let _ = event_sender.send(event);
                    let _ = event_sender.send(Event::PopupClosed);
                }
                _ => explorer.handle(&raw_event)?,
            },
            _ => {}
        }
        Ok(())
    }
}

impl Widget for &mut Popup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Popup::Help(ref key_bindings, ..) => render_help(key_bindings, area, buf),
            Popup::FileExplorer(explorer, _) => render_explorer(area, buf, explorer),
        }
    }
}

fn render_help(key_bindings: &str, area: Rect, buf: &mut Buffer) {
    let popup_area = popup_area(area, 30, 30);
    Clear.render(popup_area, buf);
    Paragraph::new(key_bindings.trim())
        .wrap(Wrap { trim: false })
        .block(
            Block::bordered()
                .title("Help")
                .title_style(Style::new().bold()),
        )
        .render(popup_area, buf);
}

fn render_explorer(area: Rect, buf: &mut Buffer, explorer: &mut FileExplorer) {
    let popup_area = popup_area(area, 50, 50);
    Clear.render(popup_area, buf);
    explorer.widget().render(popup_area, buf);
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
