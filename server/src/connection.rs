use std::net::SocketAddr;

use anyhow::Context;
use common::{RoomEvent, ServerCommand, ServerEvent, Username};
use futures::SinkExt;
use tokio::{net::TcpStream, sync::broadcast::Receiver};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tracing::instrument;

use crate::server::{Room, Rooms, Users, COMMANDS};

pub struct Connection {
    user_events: Framed<TcpStream, LinesCodec>,
    users: Users,
    rooms: Rooms,
    username: Username,
    addr: SocketAddr,
    state: ConnectionState,
    room: Room,
    room_events: Receiver<ServerEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionState {
    Connected,
    Disconnected,
}

impl Connection {
    pub fn new(tcp: TcpStream, users: Users, rooms: Rooms, addr: SocketAddr) -> Self {
        let username = Username::random();
        tracing::info!("{addr} connected with the name: {username}");
        let user_events = Framed::new(tcp, LinesCodec::new());
        let room = Rooms::lobby();
        let room_events = room.subscribe();
        Self {
            user_events,
            users,
            rooms,
            username,
            addr,
            state: ConnectionState::Connected,
            room,
            room_events,
        }
    }

    async fn send_event(&mut self, event: ServerEvent) {
        tracing::debug!(?event, "Sending event");
        if let Err(err) = self.user_events.send(event.as_json_str()).await {
            tracing::error!("Failed to send event: {err}");
            self.state = ConnectionState::Disconnected;
        }
    }

    #[instrument(skip(self), fields(addr = %self.addr, username = %self.username))]
    pub async fn handle(&mut self) {
        let help = ServerEvent::help(&self.username, COMMANDS);
        self.send_event(help).await;

        (self.room, self.room_events) = self.rooms.join(&Rooms::lobby().name, &self.username);

        let rooms = self.rooms.list();
        self.send_event(ServerEvent::rooms(rooms)).await;

        let users = self.rooms.list_users(&Rooms::lobby().name).unwrap();
        self.send_event(ServerEvent::users(users)).await;

        if let Err(err) = self.run().await {
            tracing::error!("Connection error: {err}");
        }

        self.rooms.leave(&self.room.name, &self.username);
        self.users.remove(&self.username);
        tracing::info!("disconnected");
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        while self.state == ConnectionState::Connected {
            tokio::select! {
                Some(message) = self.user_events.next() => {
                    let message = message.context("failed to read from stream")?;
                    self.handle_message(message).await;
                },
                event = self.room_events.recv() => {
                    let event = event.context("failed to read from room events")?;
                    self.send_event(event).await;
                },
                else => {
                    tracing::error!("Connection closed");
                    break;
                },
            }
        }
        Ok(())
    }

    async fn handle_message(&mut self, message: String) {
        tracing::info!("Received message: {:?}", message);
        if !message.starts_with("/") {
            self.room.send_message(&self.username, &message);
            return;
        }
        match ServerCommand::try_from(message) {
            Ok(command) => self.handle_command(command).await,
            Err(err) => {
                let event = ServerEvent::error(&format!("{err}, try /help"));
                self.send_event(event).await;
            }
        }
    }

    async fn handle_command(&mut self, command: ServerCommand) {
        match command {
            ServerCommand::Help => {
                let help = ServerEvent::help(&self.username, COMMANDS);
                self.send_event(help).await;
            }
            ServerCommand::ChangeUsername(new_name) => {
                let changed_name = self.users.insert(&new_name);
                if changed_name {
                    self.room.change_user_name(&self.username, &new_name);
                    self.username = new_name;
                } else {
                    let message = format!("{new_name} is already taken");
                    self.send_event(ServerEvent::error(&message)).await;
                }
            }
            ServerCommand::Join(new_room) => {
                (self.room, self.room_events) =
                    self.rooms
                        .change(&self.room.name, &new_room, &self.username);
            }
            ServerCommand::ListRooms => {
                let rooms_list = self.rooms.list();
                self.send_event(ServerEvent::rooms(rooms_list)).await;
            }
            ServerCommand::ListUsers => {
                if let Some(users_list) = self.rooms.list_users(&self.room.name) {
                    self.send_event(ServerEvent::users(users_list)).await;
                }
            }
            ServerCommand::SendFile(filename, contents) => {
                self.room
                    .send_event(&self.username, RoomEvent::file(&filename, &contents));
            }
            ServerCommand::Nudge(username) => {
                if let Some(users_list) = self.rooms.list_users(&self.room.name) {
                    if users_list.contains(&username) {
                        let nudge = RoomEvent::Nudge(username);
                        self.room.send_event(&self.username, nudge);
                    } else {
                        self.send_event(ServerEvent::error("user not found")).await;
                    }
                }
            }
            ServerCommand::Quit => {
                self.room.leave(&self.username);
                self.send_event(ServerEvent::Disconnect).await;
                self.state = ConnectionState::Disconnected;
            }
        }
    }
}
