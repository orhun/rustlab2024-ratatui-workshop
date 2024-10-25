use std::net::SocketAddr;

use common::ServerEvent;
use tokio::{
    net::TcpListener,
    sync::broadcast::{self, Sender},
};

use crate::{connection::Connection, rooms::Rooms, users::Users};

pub const COMMANDS: &str =
    "/help | /name {name} | /rooms | /join {room} | /users | /nudge {name} | /quit";

pub struct Server {
    listener: TcpListener,
    users: Users,
    rooms: Rooms,
    event_tx: Sender<ServerEvent>,
}

impl Server {
    pub async fn listen(addr: SocketAddr) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!("Listening on {local_addr}");
        let (event_tx, _) = broadcast::channel(1024);

        Ok(Self {
            listener,
            users: Users::default(),
            rooms: Rooms::new(event_tx.clone()),
            event_tx,
        })
    }

    pub async fn run(&self) {
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok(ok) => ok,
                Err(err) => {
                    tracing::error!("Failed to accept connection: {err}");
                    continue;
                }
            };
            let users = self.users.clone();
            let rooms = self.rooms.clone();
            let events = self.event_tx.subscribe();
            let mut connection = Connection::new(stream, events, users, rooms, addr);
            tokio::spawn(async move {
                connection.handle().await;
            });
        }
    }
}
