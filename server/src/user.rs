use common::Username;
use dashmap::DashSet;
use std::sync::Arc;

const MAX_USERS: usize = 128;

#[derive(Clone)]
#[repr(transparent)]
pub struct Users(Arc<DashSet<Username>>);

impl Users {
    pub fn new() -> Self {
        Self(Arc::new(DashSet::with_capacity(MAX_USERS)))
    }

    pub fn insert(&self, username: &Username) -> bool {
        self.0.insert(username.clone())
    }

    pub fn remove(&self, username: &Username) -> bool {
        self.0.remove(username).is_some()
    }
}
