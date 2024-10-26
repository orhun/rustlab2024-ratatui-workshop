use std::net::SocketAddr;

use anyhow::Ok;
use common::{Command, RoomEvent, RoomName, ServerEvent, Username};
use crossterm::event::{Event, EventStream, KeyCode};
use futures::{SinkExt, StreamExt};
use ratatui::DefaultTerminal;
use tokio::net::{tcp::OwnedWriteHalf, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

use crate::message_list::MessageList;

pub struct App {
    addr: SocketAddr,
    term_stream: EventStream,
    is_running: bool,
    tcp_writer: Option<FramedWrite<OwnedWriteHalf, LinesCodec>>,
    // UI components (these need to be public as we define the draw_ui method not in a child module)
    pub message_list: MessageList,
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
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        self.is_running = true;

        let connection = TcpStream::connect(self.addr).await?;
        let (reader, writer) = connection.into_split();
        self.tcp_writer = Some(FramedWrite::new(writer, LinesCodec::new()));
        let mut tcp_reader = FramedRead::new(reader, LinesCodec::new());

        while self.is_running {
            terminal.draw(|frame| frame.render_widget(&mut self.message_list, frame.area()))?;
            tokio::select! {
                Some(crossterm_event) = self.term_stream.next() => {
                    let crossterm_event = crossterm_event?;
                    if let Event::Key(key_event) = crossterm_event {
                        if key_event.code == KeyCode::Esc {
                            let framed = self.tcp_writer.as_mut().unwrap();
                            let _ = framed.send(Command::Quit.to_string()).await;
                        }
                    }
                },
                Some(tcp_event) = tcp_reader.next() => self.handle_server_event(tcp_event?).await?,
            }
        }
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
