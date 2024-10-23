use std::fmt;

use crate::{RoomName, Username};

pub enum ServerCommand {
    Help,
    /// Set the client's username
    Name(Username),
    Rooms,
    Join(RoomName),
    Users,
    File(String, String),
    Quit,
}

impl fmt::Display for ServerCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServerCommand::Help => write!(f, "/help"),
            ServerCommand::Name(name) => write!(f, "/name {}", name),
            ServerCommand::Rooms => write!(f, "/rooms"),
            ServerCommand::Join(room) => write!(f, "/join {}", room),
            ServerCommand::Users => write!(f, "/users"),
            ServerCommand::File(name, encoded) => write!(f, "/file {} {}", name, encoded),
            ServerCommand::Quit => write!(f, "/quit"),
        }
    }
}

impl TryFrom<String> for ServerCommand {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts = value.split_whitespace();
        match parts.next() {
            Some("/help") => Ok(ServerCommand::Help),
            Some("/name") => {
                let name = parts.next().ok_or("Name is required")?.into();
                Ok(ServerCommand::Name(name))
            }
            Some("/rooms") => Ok(ServerCommand::Rooms),
            Some("/join") => {
                let room = parts.next().ok_or("Room name is required")?.into();
                Ok(ServerCommand::Join(room))
            }
            Some("/users") => Ok(ServerCommand::Users),
            Some("/file") => {
                let name = parts.next().ok_or("File name is required")?.to_string();
                let encoded = parts.next().ok_or("File content is required")?.to_string();
                Ok(ServerCommand::File(name, encoded))
            }
            Some("/quit") => Ok(ServerCommand::Quit),
            _ => Err(format!("Invalid command: {}", value)),
        }
    }
}
