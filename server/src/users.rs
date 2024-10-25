use std::sync::Arc;

use common::Username;
use dashmap::DashSet;

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
