use serde::{Deserialize, Serialize};

use crate::{RoomName, Username};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerEvent {
    Help(Username, String),
    RoomEvent(Username, RoomEvent),
    Error(String),
    Rooms(Vec<RoomName>),
    Users(Vec<Username>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomEvent {
    Message(String),
    File(String, String),
    Joined(RoomName),
    Left(RoomName),
    NameChange(Username),
}

impl ServerEvent {
    pub fn as_json_str(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}
