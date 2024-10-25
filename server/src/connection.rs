use std::net::SocketAddr;

use anyhow::Context;
use common::{RoomEvent, RoomName, ServerCommand, ServerEvent, Username};
use futures::SinkExt;
use tokio::{net::TcpStream, sync::broadcast::Receiver};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tracing::instrument;

use crate::{room::Room, rooms::Rooms, server::COMMANDS, users::Users};

pub struct Connection {
    /// The events that are come from the user
    user_events: Framed<TcpStream, LinesCodec>,
    /// The events that are broadcasted to all users
    server_events: Receiver<ServerEvent>,
    /// The events that are broadcasted to the user's current room
    room_events: Receiver<ServerEvent>,
    /// The users that are connected to the server
    users: Users,
    /// The rooms that are available on the server
    rooms: Rooms,
    /// The username of the connected user
    username: Username,
    /// The address of the connected user
    addr: SocketAddr,
    /// The current state of the connection
    state: ConnectionState,
    /// The room that the user is currently in
    room: Room,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionState {
    Connected,
    Disconnected,
}

impl Connection {
    pub fn new(
        tcp: TcpStream,
        server_events: Receiver<ServerEvent>,
        users: Users,
        rooms: Rooms,
        addr: SocketAddr,
    ) -> Self {
        let username = Username::random();
        tracing::info!("{addr} connected with the name: {username}");
        let user_events = Framed::new(tcp, LinesCodec::new());
        let (room, room_events) = rooms.join(&username, &RoomName::lobby());
        Self {
            user_events,
            server_events,
            room_events,
            users,
            rooms,
            username,
            addr,
            state: ConnectionState::Connected,
            room,
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

        let rooms = self.rooms.list();
        self.send_event(ServerEvent::rooms(rooms)).await;

        let users = self.room.list_users();
        self.send_event(ServerEvent::users(users)).await;

        if let Err(err) = self.run().await {
            tracing::error!("Connection error: {err}");
        }

        self.rooms.leave(&self.username, &self.room);
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
                event = self.server_events.recv() => {
                    let event = event.context("failed to read from server events")?;
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
        if !message.starts_with("/") {
            tracing::info!("Received message: {:?}", message);
            self.room.send_message(&self.username, &message);
            return;
        }
        match ServerCommand::try_from(message) {
            Ok(command) => {
                self.log_command(&command);
                self.handle_command(command).await
            }
            Err(err) => {
                tracing::error!("Invalid command: {err}");
                let event = ServerEvent::error(&format!("{err}, try /help"));
                self.send_event(event).await;
            }
        }
    }

    fn log_command(&self, command: &ServerCommand) {
        if let ServerCommand::SendFile(filename, contents) = &command {
            tracing::info!("Received file: {filename}");
            tracing::trace!("Received file contents: {contents}");
        } else {
            tracing::info!("Received command: {command:?}");
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
                    self.rooms.change(&self.username, &self.room, &new_room);
                let users = self.room.list_users();
                self.send_event(ServerEvent::users(users)).await;
            }
            ServerCommand::ListRooms => {
                let rooms_list = self.rooms.list();
                self.send_event(ServerEvent::rooms(rooms_list)).await;
            }
            ServerCommand::ListUsers => {
                let users = self.room.list_users();
                self.send_event(ServerEvent::users(users)).await;
            }
            ServerCommand::SendFile(filename, contents) => {
                self.room
                    .send_event(&self.username, RoomEvent::file(&filename, &contents));
            }
            ServerCommand::Nudge(username) => {
                let users = self.room.list_users();
                if users.contains(&username) {
                    let nudge = RoomEvent::Nudge(username);
                    self.room.send_event(&self.username, nudge);
                } else {
                    self.send_event(ServerEvent::error("user not found")).await;
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
