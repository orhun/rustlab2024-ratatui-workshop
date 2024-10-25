use std::{cmp::Ordering, sync::Arc};

use common::{RoomName, ServerEvent, Username};
use dashmap::DashMap;
use tokio::sync::broadcast::{Receiver, Sender};

use crate::room::Room;

#[derive(Clone, Debug)]
pub struct Rooms {
    rooms: Arc<DashMap<RoomName, Room>>,
    events: Sender<ServerEvent>,
}

impl Rooms {
    pub fn new(events: Sender<ServerEvent>) -> Self {
        let rooms = Arc::new(DashMap::new());
        let lobby = Room::new(RoomName::lobby());
        rooms.insert(lobby.name().clone(), lobby);
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
            .map(|entry| (entry.key().clone(), entry.value().user_count()))
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
