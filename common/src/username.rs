use std::{borrow::Cow, convert::Infallible, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Username(String);

impl Username {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn random() -> Self {
        let username = petname::petname(1, "").expect("failed to generate petname");
        Self(username)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Username {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Username {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl FromStr for Username {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Username(s.to_string()))
    }
}

impl From<Username> for String {
    fn from(value: Username) -> Self {
        value.0
    }
}

impl From<Username> for Cow<'_, str> {
    fn from(value: Username) -> Self {
        Cow::Owned(value.0)
    }
}

impl<'a> From<&'a Username> for Cow<'a, str> {
    fn from(value: &'a Username) -> Self {
        Cow::Borrowed(&value.0)
    }
}
