use std::{cmp::Ordering, fmt, net::SocketAddr, sync::Arc};

use common::{RoomEvent, RoomName, ServerEvent, Username};
use dashmap::{DashMap, DashSet, Entry};
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
            let mut connection =
                Connection::new(stream, self.users.clone(), self.rooms.clone(), addr);
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
}

#[derive(Clone, Debug)]
pub struct Rooms {
    rooms: Arc<DashMap<RoomName, Room>>,
    events: Sender<ServerEvent>,
}

impl Rooms {
    fn new(events: Sender<ServerEvent>) -> Self {
        let rooms = Arc::new(DashMap::new());
        let lobby = Room::new("lobby".into());
        rooms.insert(lobby.name.clone(), lobby);
        Self { rooms, events }
    }

    pub fn lobby() -> Room {
        let name = RoomName::new("lobby".to_string());
        Room::new(name)
    }

    pub fn join(&self, room_name: &RoomName, username: &Username) -> (Room, Receiver<ServerEvent>) {
        let room = match self.rooms.entry(room_name.clone()) {
            Entry::Occupied(entry) => entry,
            Entry::Vacant(entry) => {
                let room = Room::new(room_name.clone());
                let room = entry.insert_entry(room);
                self.send_room_event(username, RoomEvent::created(room_name));
                room
            }
        };
        let room = room.get();
        let events = room.subscribe();
        room.join(username);
        (room.clone(), events)
    }

    pub fn leave(&self, room_name: &RoomName, username: &Username) {
        tracing::debug!("User {username} leaving room {room_name}");
        if let Some(room) = self.rooms.get_mut(room_name) {
            room.users.remove(username);
            if room.events.receiver_count() <= 1 && room.name.as_str() != "lobby" {
                // remove the room if we're the last user in the room
                self.rooms.remove(room_name);
                self.send_room_event(username, RoomEvent::deleted(room_name));
            } else {
                tracing::debug!("receiver count: {}", room.events.receiver_count());
                room.send_event(username, RoomEvent::left(room_name));
            }
        }
    }

    pub fn change(
        &self,
        prev_room: &RoomName,
        next_room: &RoomName,
        username: &Username,
    ) -> (Room, Receiver<ServerEvent>) {
        if prev_room == next_room {
            let event = ServerEvent::error("You are already in that room");
            self.send_server_event(event);
        }
        self.leave(prev_room, username);
        self.join(next_room, username)
    }

    pub fn list(&self) -> Vec<(RoomName, usize)> {
        let mut list: Vec<_> = self
            .rooms
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().events.receiver_count()))
            .collect();
        list.sort_by(|a, b| match b.1.cmp(&a.1) {
            Ordering::Equal => a.0.cmp(&b.0),
            ordering => ordering,
        });
        list
    }

    pub fn send_room_event(&self, username: &Username, event: RoomEvent) {
        let event = ServerEvent::room_event(username, event);
        let _ = self.events.send(event);
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
    fn new(name: RoomName) -> Self {
        let (events, _) = broadcast::channel(Self::ROOM_CHANNEL_CAPACITY);
        Self {
            name,
            events,
            users: Users::default(),
        }
    }

    /// Returns the name of the room
    pub fn name(&self) -> &RoomName {
        &self.name
    }

    /// Adds the specified user to the room
    pub fn join(&self, username: &Username) {
        self.users.insert(username);
        self.send_event(username, RoomEvent::joined(&self.name));
    }

    pub fn leave(&self, username: &Username) {
        self.users.remove(username);
        self.send_event(username, RoomEvent::left(&self.name));
    }

    pub fn list_users(&self) -> Vec<Username> {
        self.users.iter().sorted().collect()
    }

    pub fn change_user_name(&self, old_name: &Username, new_name: &Username) {
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

    pub fn subscribe(&self) -> Receiver<ServerEvent> {
        self.events.subscribe()
    }
}
