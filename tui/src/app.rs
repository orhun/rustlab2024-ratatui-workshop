use std::io;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use common::{RoomEvent, ServerCommand, ServerEvent};
use crossterm::event::Event as CrosstermEvent;
use futures::SinkExt;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType};
use ratatui_explorer::{File, FileExplorer, Theme};
use ratatui_image::picker::{Picker, ProtocolType};
use tokio::{
    net::tcp::WriteHalf,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_util::codec::{FramedWrite, LinesCodec};
use tui_textarea::{Input, Key, TextArea};

use crate::message_list::MessageList;
use crate::popup::Popup;
use crate::room_list::RoomList;

pub struct App {
    pub is_running: bool,
    pub message_list: MessageList,
    pub room_list: RoomList,
    pub text_area: TextArea<'static>,
    pub file_explorer: Option<FileExplorer>,
    pub popup: Option<Popup>,
    pub event_sender: UnboundedSender<Event>,
    pub event_receiver: UnboundedReceiver<Event>,
}

#[derive(Clone)]
pub enum Event {
    Terminal(CrosstermEvent),
    FileSelected(File),
}

impl App {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = unbounded_channel();
        Self {
            is_running: true,
            message_list: MessageList::default(),
            room_list: RoomList::default(),
            text_area: create_text_area(),
            file_explorer: None,
            popup: None,
            event_sender,
            event_receiver,
        }
    }

    pub async fn handle_event(
        &mut self,
        event: Event,
        tcp_writer: &mut FramedWrite<WriteHalf<'_>, LinesCodec>,
    ) -> anyhow::Result<()> {
        match event {
            Event::Terminal(raw_event) => {
                let input = Input::from(raw_event.clone());
                // Handle popup
                match self.popup {
                    Some(Popup::FileExplorer) => {
                        match input {
                            Input { key: Key::Esc, .. } => {
                                self.popup = None;
                            }
                            Input {
                                key: Key::Enter, ..
                            } => {
                                self.popup = None;
                                let file = self.file_explorer.as_ref().unwrap().current();
                                self.event_sender.send(Event::FileSelected(file.clone()))?;
                            }
                            _ => {
                                let explorer = self.file_explorer.as_mut().unwrap();
                                explorer.handle(&raw_event)?;
                            }
                        }
                        return Ok(());
                    }
                    Some(Popup::ImagePreview(_)) => {
                        if matches!(input, Input { key: Key::Esc, .. }) {
                            self.popup = None;
                        }
                        return Ok(());
                    }
                    _ => {}
                }

                // Handle key input
                match input {
                    // Esc
                    Input { key: Key::Esc, .. } => self.is_running = false,
                    // Enter
                    Input {
                        key: Key::Enter, ..
                    } => {
                        if !self.text_area.is_empty() {
                            for line in self.text_area.clone().into_lines() {
                                tcp_writer.send(line).await?;
                            }
                            self.text_area.select_all();
                            self.text_area.delete_line_by_end();
                        }
                    }
                    // Down
                    Input { key: Key::Down, .. } => {
                        self.message_list.state.select_previous();
                    }
                    // Up
                    Input { key: Key::Up, .. } => {
                        self.message_list.state.select_next();
                    }
                    // Show explorer
                    Input {
                        key: Key::Char('e'),
                        ctrl: true,
                        ..
                    } => {
                        let file_explorer = create_file_explorer()?;
                        // todo store in Popup
                        self.file_explorer = Some(file_explorer);
                        self.popup = Some(Popup::FileExplorer);
                    }
                    // Preview file
                    Input {
                        key: Key::Char('p'),
                        ctrl: true,
                        ..
                    } => {
                        let selected_event = self.message_list.selected_event();
                        if let Some(ServerEvent::RoomEvent(_, RoomEvent::File(_, contents))) =
                            selected_event
                        {
                            let data = BASE64_STANDARD.decode(contents.as_bytes())?;
                            let img = image::load_from_memory(&data)?;
                            let user_fontsize = (7, 14);
                            let user_protocol = ProtocolType::Halfblocks;
                            let mut picker = Picker::new(user_fontsize);
                            picker.protocol_type = user_protocol;
                            let image = picker.new_resize_protocol(img);
                            self.popup = Some(Popup::ImagePreview(image));
                        }
                    }
                    // Other key presses
                    input => {
                        self.text_area.input_without_shortcuts(input);
                    }
                }
            }
            // Send file to server
            Event::FileSelected(file) => {
                if file.is_dir() {
                    return Ok(());
                }
                let contents = tokio::fs::read(file.path()).await?;
                let base64 = BASE64_STANDARD.encode(contents);
                tcp_writer
                    .send(ServerCommand::File(file.name().to_string(), base64).to_string())
                    .await?;
            }
        }

        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn handle_tcp_event(
        &mut self,
        event: String,
        tcp_writer: &mut FramedWrite<WriteHalf<'_>, LinesCodec>,
    ) -> anyhow::Result<()> {
        let event = ServerEvent::from_json_str(&event)?;
        self.message_list.events.push(event.clone());
        match event {
            ServerEvent::Help(username, help) => {
                self.message_list.username = username;
            }
            ServerEvent::RoomEvent(username, RoomEvent::Message(message)) => {}
            ServerEvent::RoomEvent(username, RoomEvent::Joined(room))
            | ServerEvent::RoomEvent(username, RoomEvent::Left(room)) => {
                self.message_list.room = room.clone();
                self.room_list.room = room;
                tcp_writer.send(ServerCommand::Users.to_string()).await?;
                tcp_writer.send(ServerCommand::Rooms.to_string()).await?;
            }
            ServerEvent::RoomEvent(username, RoomEvent::NameChange(new_username)) => {
                if username == self.message_list.username {
                    self.message_list.username = new_username;
                }
            }
            ServerEvent::RoomEvent(username, RoomEvent::File(name, contents)) => {}
            ServerEvent::Error(error) => {}
            ServerEvent::Rooms(rooms) => {
                self.room_list.rooms = rooms;
            }
            ServerEvent::Users(users) => {
                self.room_list.users = users;
            }
        }
        Ok(())
    }
}

fn create_text_area() -> TextArea<'static> {
    let mut text_area = TextArea::default();
    text_area.set_cursor_line_style(Style::default());
    text_area.set_placeholder_text("Start typing...");
    text_area
}

fn create_file_explorer() -> io::Result<FileExplorer> {
    FileExplorer::with_theme(
        Theme::default()
            .add_default_title()
            .with_title_bottom(|fe| format!("[ {} files ]", fe.files().len()).into())
            .with_style(Style::default().fg(Color::Yellow))
            .with_highlight_item_style(Style::default().add_modifier(Modifier::BOLD))
            .with_highlight_dir_style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .with_highlight_symbol("> ")
            .with_block(Block::bordered().border_type(BorderType::Rounded)),
    )
}
