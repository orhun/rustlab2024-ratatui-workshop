use std::fmt;

use common::{RoomName, ServerEvent, Username};
use itertools::Itertools;
use tokio::sync::broadcast::{self, Receiver, Sender};

use common::RoomEvent;

use crate::users::Users;

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
    pub(crate) const ROOM_CHANNEL_CAPACITY: usize = 1024;

    /// Create a new room with the given name
    pub(crate) fn new(room_name: RoomName) -> Self {
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

    pub fn user_count(&self) -> usize {
        self.users.len()
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
        let event = ServerEvent::room_event(&self.name, username, event);
        let _ = self.events.send(event);
    }
}
