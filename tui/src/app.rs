use std::net::SocketAddr;

use anyhow::Ok;
use base64::{prelude::BASE64_STANDARD, Engine};
use common::{RoomEvent, ServerCommand, ServerEvent, Username};
use crossterm::event::{Event as CrosstermEvent, EventStream};
use futures::{SinkExt, StreamExt};
use ratatui::{style::Style, DefaultTerminal};
use ratatui_explorer::File;
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
    addr: SocketAddr,
    term_stream: EventStream,
    is_running: bool,
    event_sender: UnboundedSender<Event>,
    event_receiver: UnboundedReceiver<Event>,
    tcp_writer: Option<FramedWrite<OwnedWriteHalf, LinesCodec>>,
    // UI components (these need to be public as we define the draw_ui method not in a child module)
    pub message_list: MessageList,
    pub room_list: RoomList,
    pub text_area: TextArea<'static>,
    pub popup: Option<Popup>,
}

#[derive(Clone)]
pub enum Event {
    Terminal(CrosstermEvent),
    FileSelected(File),
    PopupClosed,
}

impl From<CrosstermEvent> for Event {
    fn from(event: CrosstermEvent) -> Self {
        Event::Terminal(event)
    }
}

impl App {
    pub fn new(addr: SocketAddr) -> Self {
        let (event_sender, event_receiver) = unbounded_channel();
        let term_stream = EventStream::new();
        Self {
            addr,
            term_stream,
            is_running: false,
            event_sender,
            event_receiver,
            tcp_writer: None,
            message_list: MessageList::default(),
            room_list: RoomList::default(),
            text_area: create_text_area(),
            popup: None,
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
                Some(tcp_event) = tcp_reader.next() => self.handle_server_event(tcp_event?).await?,
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
                if let Some(ref mut popup) = self.popup {
                    popup.handle_input(input, raw_event).await?;
                    return Ok(());
                }
                self.handle_key_input(input).await?;
            }
            // Send file to server
            Event::FileSelected(file) => {
                self.popup = None;
                let contents = tokio::fs::read(file.path()).await?;
                let base64 = BASE64_STANDARD.encode(contents);
                let command = ServerCommand::File(file.name().to_string(), base64);
                self.send(command).await;
            }
            Event::PopupClosed => {
                self.popup = None;
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
        let popup = Popup::file_explorer(self.event_sender.clone())?;
        self.popup = Some(popup);
        Ok(())
    }

    fn preview_file(&mut self) -> Result<(), anyhow::Error> {
        let selected_event = self.message_list.selected_event();
        let event_sender = self.event_sender.clone();
        if let Some(ServerEvent::RoomEvent(_, RoomEvent::File(_, contents))) = selected_event {
            let image_preview = Popup::image_preview(contents, event_sender)?;
            self.popup = Some(image_preview);
        }
        Ok(())
    }

    pub async fn handle_server_event(&mut self, event: String) -> anyhow::Result<()> {
        let event = ServerEvent::from_json_str(&event)?;
        self.message_list.events.push(event.clone());
        match event {
            ServerEvent::Help(username, _help) => self.message_list.username = username,
            ServerEvent::RoomEvent(username, room_event) => {
                self.handle_room_event(username, room_event).await
            }
            ServerEvent::Error(_error) => {}
            ServerEvent::Rooms(rooms) => self.room_list.rooms = rooms,
            ServerEvent::Users(users) => self.room_list.users = users,
        }
        Ok(())
    }

    async fn handle_room_event(&mut self, username: Username, room_event: RoomEvent) {
        match room_event {
            RoomEvent::Message(_message) => {}
            RoomEvent::Joined(room) | RoomEvent::Left(room) => {
                self.message_list.room_name = room.clone();
                self.room_list.room_name = room;
                self.send(ServerCommand::Users).await;
                self.send(ServerCommand::Rooms).await;
            }
            RoomEvent::NameChange(new_username) => {
                if username == self.message_list.username {
                    self.message_list.username = new_username;
                }
            }
            RoomEvent::File(_name, _contents) => {}
        }
    }
}

fn create_text_area() -> TextArea<'static> {
    let mut text_area = TextArea::default();
    text_area.set_cursor_line_style(Style::default());
    text_area.set_placeholder_text("Start typing...");
    text_area
}
