use std::{borrow::Cow, convert::Infallible, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct RoomName(String);

impl RoomName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RoomName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for RoomName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RoomName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl FromStr for RoomName {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RoomName(s.to_string()))
    }
}

impl From<RoomName> for String {
    fn from(value: RoomName) -> Self {
        value.0
    }
}

impl From<RoomName> for Cow<'_, str> {
    fn from(value: RoomName) -> Self {
        Cow::Owned(value.0)
    }
}

impl<'a> From<&'a RoomName> for Cow<'a, str> {
    fn from(value: &'a RoomName) -> Self {
        Cow::Borrowed(&value.0)
    }
}
