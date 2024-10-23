use std::{io, net::SocketAddr};

use anyhow::Ok;
use base64::{prelude::BASE64_STANDARD, Engine};
use common::{RoomEvent, ServerCommand, ServerEvent};
use crossterm::event::Event as CrosstermEvent;
use futures::{SinkExt, StreamExt};
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType},
    DefaultTerminal,
};
use ratatui_explorer::{File, FileExplorer, Theme};
use ratatui_image::picker::{Picker, ProtocolType};
use tokio::{
    net::{tcp::OwnedWriteHalf, TcpStream},
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use tui_textarea::{Input, Key, TextArea};

use crate::message_list::MessageList;
use crate::popup::Popup;
use crate::room_list::RoomList;

pub struct App {
    pub addr: SocketAddr,
    pub term_stream: crossterm::event::EventStream,
    pub is_running: bool,
    pub message_list: MessageList,
    pub room_list: RoomList,
    pub text_area: TextArea<'static>,
    pub popup: Option<Popup>,
    pub event_sender: UnboundedSender<Event>,
    pub event_receiver: UnboundedReceiver<Event>,
    pub tcp_writer: Option<FramedWrite<OwnedWriteHalf, LinesCodec>>,
}

#[derive(Clone)]
pub enum Event {
    Terminal(CrosstermEvent),
    FileSelected(File),
}

impl From<CrosstermEvent> for Event {
    fn from(event: CrosstermEvent) -> Self {
        Event::Terminal(event)
    }
}

impl App {
    pub fn new(addr: SocketAddr) -> Self {
        let (event_sender, event_receiver) = unbounded_channel();
        let term_stream = crossterm::event::EventStream::new();
        Self {
            addr,
            term_stream,
            is_running: false,
            message_list: MessageList::default(),
            room_list: RoomList::default(),
            text_area: create_text_area(),
            popup: None,
            event_sender,
            event_receiver,
            tcp_writer: None,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        self.is_running = true;

        let connection = TcpStream::connect(self.addr).await?;
        let (reader, writer) = connection.into_split();
        self.tcp_writer = Some(FramedWrite::new(writer, LinesCodec::new()));
        let mut tcp_reader = FramedRead::new(reader, LinesCodec::new());

        while self.is_running {
            terminal.draw(|frame| self.draw_ui(frame))?;
            tokio::select! {
                Some(crossterm_event) = self.term_stream.next() => {
                    let crossterm_event = crossterm_event?;
                    self.handle_event(Event::from(crossterm_event)).await?;
                },
                Some(event) = self.event_receiver.recv() => self.handle_event(event).await?,
                Some(tcp_event) = tcp_reader.next() => self.handle_tcp_event(tcp_event?).await?,
            }
        }
        Ok(())
    }

    pub async fn send(&mut self, command: ServerCommand) {
        let framed = self.tcp_writer.as_mut().unwrap();
        let _ = framed.send(command.to_string()).await;
    }

    pub async fn handle_event(&mut self, event: Event) -> anyhow::Result<()> {
        match event {
            Event::Terminal(raw_event) => {
                let input = Input::from(raw_event.clone());
                if self.popup.is_some() {
                    self.handle_popup_input(input, raw_event).await?;
                    return Ok(());
                }
                self.handle_key_input(input).await?;
            }
            // Send file to server
            Event::FileSelected(file) => {
                let contents = tokio::fs::read(file.path()).await?;
                let base64 = BASE64_STANDARD.encode(contents);
                let command = ServerCommand::File(file.name().to_string(), base64);
                self.send(command).await;
            }
        }

        Ok(())
    }

    async fn handle_key_input(&mut self, input: Input) -> anyhow::Result<(), anyhow::Error> {
        match (input.ctrl, input.key) {
            (_, Key::Esc) => self.is_running = false,
            (_, Key::Enter) => self.send_message().await?,
            (_, Key::Down) => self.message_list.state.select_previous(),
            (_, Key::Up) => self.message_list.state.select_next(),
            (true, Key::Char('e')) => self.show_file_explorer()?,
            (true, Key::Char('p')) => self.preview_file()?,
            (_, _) => {
                let _ = self.text_area.input_without_shortcuts(input);
            }
        }
        Ok(())
    }

    async fn send_message(&mut self) -> Result<(), anyhow::Error> {
        let sink = self.tcp_writer.as_mut().unwrap();
        if !self.text_area.is_empty() {
            for line in self.text_area.clone().into_lines() {
                sink.send(line).await?;
            }
            self.text_area.select_all();
            self.text_area.delete_line_by_end();
        }
        Ok(())
    }

    fn show_file_explorer(&mut self) -> Result<(), anyhow::Error> {
        let file_explorer = create_file_explorer()?;
        self.popup = Some(Popup::FileExplorer(file_explorer));
        Ok(())
    }

    fn preview_file(&mut self) -> Result<(), anyhow::Error> {
        let selected_event = self.message_list.selected_event();
        if let Some(ServerEvent::RoomEvent(_, RoomEvent::File(_, contents))) = selected_event {
            let data = BASE64_STANDARD.decode(contents.as_bytes())?;
            let img = image::load_from_memory(&data)?;
            let user_fontsize = (7, 14);
            let user_protocol = ProtocolType::Halfblocks;
            let mut picker = Picker::new(user_fontsize);
            picker.protocol_type = user_protocol;
            let image = picker.new_resize_protocol(img);
            self.popup = Some(Popup::ImagePreview(image));
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn handle_tcp_event(&mut self, event: String) -> anyhow::Result<()> {
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
                self.send(ServerCommand::Users).await;
                self.send(ServerCommand::Rooms).await;
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

    async fn handle_popup_input(
        &mut self,
        input: Input,
        raw_event: CrosstermEvent,
    ) -> anyhow::Result<()> {
        match self.popup {
            Some(Popup::FileExplorer(ref mut explorer)) => match input.key {
                Key::Esc => self.popup = None,
                Key::Enter => {
                    let file = explorer.current().clone();
                    if file.is_dir() {
                        return Ok(());
                    }
                    let event = Event::FileSelected(file);
                    let _ = self.event_sender.send(event);
                    self.popup = None;
                }
                _ => explorer.handle(&raw_event)?,
            },
            Some(Popup::ImagePreview(_)) if input.key == Key::Esc => {
                self.popup = None;
            }
            _ => {}
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
