use serde::{Deserialize, Serialize};
use strum_macros::Display;

use crate::{RoomName, Username};

#[derive(Debug, Clone, Serialize, Deserialize, Display)]
pub enum ServerEvent {
    #[strum(to_string = "Help({0}, {1})")]
    CommandHelp(Username, String),
    #[strum(to_string = "{username} {event}")]
    RoomEvent {
        room_name: RoomName,
        username: Username,
        event: RoomEvent,
    },
    #[strum(to_string = "Room Created({0})")]
    RoomCreated(RoomName),
    #[strum(to_string = "Room Deleted({0})")]
    RoomDeleted(RoomName),
    #[strum(to_string = "Error({0})")]
    Error(String),
    #[strum(to_string = "Rooms({0:?})")]
    Rooms(Vec<(RoomName, usize)>),
    #[strum(to_string = "Users({0:?})")]
    Users(Vec<Username>),
    #[strum(to_string = "Disconnected")]
    Disconnect,
}

impl ServerEvent {
    pub fn help(username: &Username, commands: &str) -> Self {
        Self::CommandHelp(username.clone(), commands.to_string())
    }

    pub fn error(message: &str) -> Self {
        Self::Error(message.to_string())
    }

    pub fn rooms(rooms: Vec<(RoomName, usize)>) -> Self {
        Self::Rooms(rooms)
    }

    pub fn users(users: Vec<Username>) -> Self {
        Self::Users(users)
    }

    pub fn room_event(room_name: &RoomName, username: &Username, event: RoomEvent) -> Self {
        Self::RoomEvent {
            room_name: room_name.clone(),
            username: username.clone(),
            event,
        }
    }

    pub fn room_created(room_name: &RoomName) -> Self {
        Self::RoomCreated(room_name.clone())
    }

    pub fn room_deleted(room_name: &RoomName) -> Self {
        Self::RoomDeleted(room_name.clone())
    }

    pub fn as_json_str(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Display)]
pub enum RoomEvent {
    #[strum(to_string = "created room {0}")]
    Message(String),
    #[strum(to_string = "sent file: {filename}")]
    File { filename: String, contents: String },
    #[strum(to_string = "joined room {0}")]
    Joined(RoomName),
    #[strum(to_string = "left room {0}")]
    Left(RoomName),
    #[strum(to_string = "changed name to {0}")]
    NameChange(Username),
    #[strum(to_string = "nudged {0}")]
    Nudge(Username),
}

impl RoomEvent {
    pub fn message(message: &str) -> Self {
        Self::Message(message.to_string())
    }

    pub fn file(filename: &str, contents: &str) -> Self {
        Self::File {
            filename: filename.to_string(),
            contents: contents.to_string(),
        }
    }

    pub fn left(room_name: &RoomName) -> Self {
        Self::Left(room_name.clone())
    }

    pub fn joined(room_name: &RoomName) -> Self {
        Self::Joined(room_name.clone())
    }

    pub fn name_change(username: &Username) -> Self {
        Self::NameChange(username.clone())
    }

    pub fn nudge(username: &Username) -> Self {
        Self::Nudge(username.clone())
    }
}
