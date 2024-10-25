use std::fmt;

use crate::{RoomName, Username};

#[derive(Debug)]
pub enum ServerCommand {
    Help,
    ChangeUsername(Username),
    ListRooms,
    Join(RoomName),
    ListUsers,
    SendFile(String, String),
    Nudge(Username),
    Quit,
}

impl fmt::Display for ServerCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServerCommand::Help => write!(f, "/help"),
            ServerCommand::ChangeUsername(name) => write!(f, "/name {}", name),
            ServerCommand::ListRooms => write!(f, "/rooms"),
            ServerCommand::Join(room) => write!(f, "/join {}", room),
            ServerCommand::ListUsers => write!(f, "/users"),
            ServerCommand::SendFile(filename, encoded) => {
                write!(f, "/file {} {}", filename, encoded)
            }
            ServerCommand::Nudge(username) => write!(f, "/nudge {}", username),
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
                Ok(ServerCommand::ChangeUsername(name))
            }
            Some("/rooms") => Ok(ServerCommand::ListRooms),
            Some("/join" | "/j") => {
                let room = parts.next().ok_or("Room name is required")?.into();
                Ok(ServerCommand::Join(room))
            }
            Some("/users") => Ok(ServerCommand::ListUsers),
            Some("/file") => {
                let filename = parts.next().ok_or("File name is required")?.to_string();
                let encoded = parts.next().ok_or("File content is required")?.to_string();
                Ok(ServerCommand::SendFile(filename, encoded))
            }
            Some("/nudge") => {
                let username = parts.next().ok_or("Username is required")?.into();
                Ok(ServerCommand::Nudge(username))
            }
            Some("/quit") => Ok(ServerCommand::Quit),
            _ => Err(format!("Invalid command: {}", value)),
        }
    }
}
