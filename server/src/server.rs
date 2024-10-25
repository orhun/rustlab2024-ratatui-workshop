use std::{cmp::Ordering, fmt, net::SocketAddr, sync::Arc};

use common::{RoomEvent, RoomName, ServerEvent, Username};
use dashmap::{DashMap, DashSet};
use itertools::Itertools;
use tokio::{
    net::TcpListener,
    sync::broadcast::{self, Receiver, Sender},
};

use crate::connection::Connection;

pub const COMMANDS: &str =
    "/help | /name {name} | /rooms | /join {room} | /users | /nudge {name} | /quit";

pub struct Server {
    listener: TcpListener,
    users: Users,
    rooms: Rooms,
    event_tx: Sender<ServerEvent>,
}

impl Server {
    pub async fn listen(addr: SocketAddr) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!("Listening on {local_addr}");
        let (event_tx, _) = broadcast::channel(1024);

        Ok(Self {
            listener,
            users: Users::default(),
            rooms: Rooms::new(event_tx.clone()),
            event_tx,
        })
    }

    pub async fn run(&self) {
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok(ok) => ok,
                Err(err) => {
                    tracing::error!("Failed to accept connection: {err}");
                    continue;
                }
            };
            let users = self.users.clone();
            let rooms = self.rooms.clone();
            let events = self.event_tx.subscribe();
            let mut connection = Connection::new(stream, events, users, rooms, addr);
            tokio::spawn(async move {
                connection.handle().await;
            });
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Users {
    inner: Arc<DashSet<Username>>,
}

impl Users {
    pub fn insert(&self, username: &Username) -> bool {
        self.inner.insert(username.clone())
    }

    pub fn remove(&self, username: &Username) -> bool {
        self.inner.remove(username).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = Username> + '_ {
        self.inner.iter().map(|username| username.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

#[derive(Clone, Debug)]
pub struct Rooms {
    rooms: Arc<DashMap<RoomName, Room>>,
    events: Sender<ServerEvent>,
}

impl Rooms {
    fn new(events: Sender<ServerEvent>) -> Self {
        let rooms = Arc::new(DashMap::new());
        let lobby = Room::new(RoomName::lobby());
        rooms.insert(lobby.name.clone(), lobby);
        Self { rooms, events }
    }

    pub fn join(&self, username: &Username, room_name: &RoomName) -> (Room, Receiver<ServerEvent>) {
        let room = self
            .rooms
            .entry(room_name.clone())
            .or_insert_with(|| self.create_room(room_name));
        let events = room.join(username);
        (room.clone(), events)
    }

    fn create_room(&self, room_name: &RoomName) -> Room {
        tracing::debug!("Creating room {room_name}");
        let room = Room::new(room_name.clone());
        self.send_server_event(ServerEvent::room_created(room_name));
        room
    }

    pub fn leave(&self, username: &Username, room: &Room) {
        room.leave(username);
        if room.is_empty() {
            self.delete_room(&room);
        }
    }

    fn delete_room(&self, room: &Room) {
        if room.is_lobby() {
            tracing::debug!("no users in the lobby, not deleting");
            return;
        }
        tracing::debug!("Deleting room {room}");
        self.rooms.remove(room.name());
        self.send_server_event(ServerEvent::room_deleted(room.name()));
    }

    pub fn change(
        &self,
        username: &Username,
        previous: &Room,
        next: &RoomName,
    ) -> (Room, Receiver<ServerEvent>) {
        if next == previous.name() {
            let event = ServerEvent::error("You are already in that room");
            self.send_server_event(event);
        }
        self.leave(username, previous);
        self.join(username, next)
    }

    pub fn list(&self) -> Vec<(RoomName, usize)> {
        let mut list: Vec<_> = self
            .rooms
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().users.len()))
            .collect();
        list.sort_by(|a, b| match b.1.cmp(&a.1) {
            Ordering::Equal => a.0.cmp(&b.0),
            ordering => ordering,
        });
        list
    }

    pub fn send_server_event(&self, event: ServerEvent) {
        let _ = self.events.send(event);
    }
}

#[derive(Debug, Clone)]
pub struct Room {
    name: RoomName,
    events: Sender<ServerEvent>,
    users: Users,
}

impl fmt::Display for Room {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Room {
    const ROOM_CHANNEL_CAPACITY: usize = 1024;

    /// Create a new room with the given name
    fn new(room_name: RoomName) -> Self {
        tracing::debug!("Creating room {room_name}");
        let (events, _) = broadcast::channel(Self::ROOM_CHANNEL_CAPACITY);
        Self {
            name: room_name,
            events,
            users: Users::default(),
        }
    }

    /// Returns the name of the room
    pub fn name(&self) -> &RoomName {
        &self.name
    }

    /// Adds the specified user to the room
    pub fn join(&self, username: &Username) -> Receiver<ServerEvent> {
        tracing::debug!("User {username} joining room {self}");
        self.users.insert(username);
        let events = self.events.subscribe();
        self.send_event(username, RoomEvent::joined(&self.name));
        events
    }

    /// Removes the specified user from the room
    pub fn leave(&self, username: &Username) {
        tracing::debug!(
            "User {username} leaving room {self} with {count} users",
            count = self.users.len()
        );
        self.users.remove(username);
        self.send_event(username, RoomEvent::left(&self.name));
    }

    pub fn list_users(&self) -> Vec<Username> {
        self.users.iter().sorted().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    pub fn is_lobby(&self) -> bool {
        self.name.as_str() == "lobby"
    }

    pub fn change_user_name(&self, old_name: &Username, new_name: &Username) {
        tracing::debug!("User {old_name} changing name to {new_name} in room {self}");
        self.users.remove(old_name);
        self.users.insert(new_name);
        self.send_event(old_name, RoomEvent::name_change(new_name));
    }

    pub fn send_message(&self, username: &Username, message: &str) {
        self.send_event(username, RoomEvent::message(message));
    }

    pub fn send_event(&self, username: &Username, event: RoomEvent) {
        let _ = self.events.send(ServerEvent::room_event(username, event));
    }
}
