use common::{Command, RoomEvent, RoomName, ServerEvent, Username};
use crossterm::event::EventStream;
use futures::{SinkExt, StreamExt};
use ratatui::{style::Style, DefaultTerminal};
use std::net::SocketAddr;
use tokio::net::{tcp::OwnedWriteHalf, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use tui_textarea::{Input, Key, TextArea};

use crate::message_list::MessageList;

fn create_text_area() -> TextArea<'static> {
    todo!("return a TextArea")
}

pub struct App {
    addr: SocketAddr,
    term_stream: EventStream,
    is_running: bool,
    tcp_writer: Option<FramedWrite<OwnedWriteHalf, LinesCodec>>,
    // UI components (these need to be public as we define the draw_ui method not in a child module)
    pub message_list: MessageList,
    pub text_area: TextArea<'static>,
}

impl App {
    pub fn new(addr: SocketAddr) -> Self {
        let term_stream = EventStream::new();
        Self {
            addr,
            term_stream,
            is_running: false,
            tcp_writer: None,
            message_list: MessageList::default(),
            text_area: create_text_area(),
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        self.is_running = true;

        let connection = TcpStream::connect(self.addr).await?;
        let (reader, writer) = connection.into_split();
        let mut tcp_reader = FramedRead::new(reader, LinesCodec::new());
        self.tcp_writer = Some(FramedWrite::new(writer, LinesCodec::new()));

        while self.is_running {
            terminal.draw(|frame| self.draw_ui(frame))?;
            tokio::select! {
                Some(crossterm_event) = self.term_stream.next() => {
                    let crossterm_event = crossterm_event?;
                    let input = Input::from(crossterm_event.clone());
                    self.handle_key_input(input).await?;
                },
                Some(tcp_event) = tcp_reader.next() => self.handle_server_event(tcp_event?).await?,
            }
        }
        Ok(())
    }

    async fn handle_key_input(&mut self, input: Input) -> anyhow::Result<()> {
        // TODO: handle key input
        Ok(())
    }

    async fn send_message(&mut self) -> anyhow::Result<()> {
        // TODO: send the message in the text area to the server
        Ok(())
    }

    pub async fn handle_server_event(&mut self, event: String) -> anyhow::Result<()> {
        let event = ServerEvent::from_json_str(&event)?;
        self.message_list.events.push(event.clone());
        match event {
            ServerEvent::CommandHelp(username, _help) => self.message_list.username = username,
            ServerEvent::RoomEvent {
                room_name,
                username,
                event,
                ..
            } => self.handle_room_event(room_name, username, event).await,
            ServerEvent::Error(_error) => {}
            ServerEvent::Disconnect => {
                self.is_running = false;
            }
            ServerEvent::RoomCreated(_) => {}
            ServerEvent::RoomDeleted(_) => {}
            ServerEvent::Rooms(_) => {}
            ServerEvent::Users(_) => {}
        }
        Ok(())
    }

    async fn handle_room_event(
        &mut self,
        _room_name: RoomName,
        username: Username,
        room_event: RoomEvent,
    ) {
        match room_event {
            RoomEvent::Message(_message) => {}
            RoomEvent::Joined(room) | RoomEvent::Left(room) => {
                self.message_list.room_name = room.clone();
            }
            RoomEvent::NameChange(new_username) => {
                if username == self.message_list.username {
                    self.message_list.username = new_username;
                }
            }
            RoomEvent::Nudge(_) => {}
            RoomEvent::File { .. } => {}
        }
    }
}
