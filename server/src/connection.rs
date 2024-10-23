use crate::{b, room::DEFAULT_ROOM, SERVER_COMMANDS};
use std::{io::ErrorKind, net::SocketAddr};

use common::{RoomEvent, ServerCommand, ServerEvent, Username};
use futures::SinkExt;
use tokio::{net::TcpStream, sync::broadcast::error::RecvError};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError};

use crate::{room::Rooms, user::Users};

const MAX_MSG_LEN: usize = usize::MAX;

pub struct Connection {
    tcp: TcpStream,
    users: Users,
    rooms: Rooms,
    username: Username,
    addr: SocketAddr,
}

impl Connection {
    pub fn new(tcp: TcpStream, users: Users, rooms: Rooms, addr: SocketAddr) -> Self {
        let username = petname::petname(1, "")
            .expect("failed to generate username")
            .into();
        tracing::debug!("{addr} connected with the name: {username}");
        Self {
            tcp,
            users,
            rooms,
            username,
            addr,
        }
    }

    pub async fn handle(&mut self) {
        let (reader, writer) = self.tcp.split();
        let mut stream = FramedRead::new(reader, LinesCodec::new_with_max_length(MAX_MSG_LEN));
        let mut sink = FramedWrite::new(writer, LinesCodec::new_with_max_length(MAX_MSG_LEN));

        let mut exit_result = sink
            .send(
                ServerEvent::Help(self.username.clone(), SERVER_COMMANDS.to_string()).as_json_str(),
            )
            .await;
        // let mut exit_result = sink
        //     .send(format!("{SERVER_COMMANDS}\nYou are {}", self.username))
        //     .await;
        if should_exit(exit_result) {
            self.users.remove(&self.username);
            return;
        }

        let mut room_name = DEFAULT_ROOM.into();
        let mut room_tx = self.rooms.join(&room_name, &self.username);
        let mut room_rx = room_tx.subscribe();
        let _ = room_tx.send(ServerEvent::RoomEvent(
            self.username.clone(),
            RoomEvent::Joined(room_name.clone()),
        ));

        let mut discarding_long_msg = false;

        exit_result = loop {
            tokio::select! {
                user_msg = stream.next() => {
                    let user_msg = match user_msg {
                        Some(Ok(msg)) => msg,
                        Some(Err(LinesCodecError::MaxLineLengthExceeded)) => {
                            b!(sink.send(ServerEvent::Error(format!("Messages can only be {MAX_MSG_LEN} chars long")).as_json_str()).await);
                            discarding_long_msg = true;
                            continue;
                        }
                        Some(Err(LinesCodecError::Io(io_err))) => match io_err.kind() {
                            ErrorKind::InvalidData | ErrorKind::InvalidInput
                            | ErrorKind::BrokenPipe | ErrorKind::ConnectionReset => break Ok(()),
                            _ => break Err(LinesCodecError::Io(io_err)),
                        },
                        None if !discarding_long_msg => break Ok(()),
                        None => {
                            discarding_long_msg = false;
                            continue;
                        }
                    };
                    if !user_msg.starts_with("/") {
                        let event = ServerEvent::RoomEvent(self.username.clone(), RoomEvent::Message(user_msg));
                        let _ = room_tx.send(event);
                        continue;
                    }

                    match ServerCommand::try_from(user_msg.clone()) {
                        Ok(ServerCommand::Help) => {
                            b!(sink.send(ServerEvent::Help(self.username.clone(), SERVER_COMMANDS.to_string()).as_json_str()).await);
                        },
                        Ok(ServerCommand::Name(new_name)) => {
                            let changed_name = self.users.insert(&new_name);
                            if changed_name {
                                self.rooms.change_name(&room_name, &self.username, &new_name);
                                let event = ServerEvent::RoomEvent(self.username.clone(), RoomEvent::NameChange(new_name.clone()));
                                let _ = room_tx.send(event);
                                self.username = new_name;
                            } else {
                                b!(sink.send(ServerEvent::Error(format!("{new_name} is already taken")).as_json_str()).await);
                            }
                        },
                        Ok(ServerCommand::Join(new_room)) => {
                            if new_room == room_name {
                                b!(sink.send(ServerEvent::Error(format!("You are in {room_name}")).as_json_str()).await);
                                continue;
                            }
                            let _ = room_tx.send(ServerEvent::RoomEvent(self.username.clone(), RoomEvent::Left(room_name.clone())));
                            room_tx = self.rooms.change(&room_name, &new_room, &self.username);
                            room_rx = room_tx.subscribe();
                            room_name = new_room;
                            let _ = room_tx.send(ServerEvent::RoomEvent(self.username.clone(), RoomEvent::Joined(room_name.clone())));
                        },
                        Ok(ServerCommand::Rooms) => {
                            let rooms_list = self.rooms.list();
                            b!(sink.send(ServerEvent::Rooms(rooms_list.iter().map(|(v1, _)| v1.clone()).collect()).as_json_str()).await);
                        },
                        Ok(ServerCommand::Users) => {
                            let users_list = self.rooms.list_users(&room_name).unwrap();
                            b!(sink.send(ServerEvent::Users(users_list).as_json_str()).await);
                        },
                        Ok(ServerCommand::File(filename, contents)) => {
                            let _ = room_tx.send(ServerEvent::RoomEvent(self.username.clone(), RoomEvent::File(filename.clone(), contents.clone())));
                        },
                        Ok(ServerCommand::Quit) => {
                            break Ok(());
                        },
                        Err(err) => {
                            b!(sink.send(ServerEvent::Error(format!("{err}, try /help")).as_json_str()).await);
                        }
                    };

                }
                peer_msg = room_rx.recv() => {
                    let peer_msg = match peer_msg {
                        Ok(ok) => ok,
                        // we would get this error if all tx
                        // were dropped for this rx, which is not
                        // possible since we're holding a tx,
                        // but if this were to somehow ever happen
                        // we just put the user back into the main
                        // room
                        Err(RecvError::Closed) => {
                            let _ = room_tx.send(ServerEvent::RoomEvent(self.username.clone(), RoomEvent::Left(room_name.clone())));
                            room_tx = self.rooms.change(&room_name, &DEFAULT_ROOM.into(), &self.username);
                            room_rx = room_tx.subscribe();
                            room_name = DEFAULT_ROOM.into();
                            let _ = room_tx.send(ServerEvent::RoomEvent(self.username.clone(), RoomEvent::Joined(room_name.clone())));
                            continue;
                        },
                        // under high load we might not deliver all msgs
                        // to all users in a room, in which case we let
                        // them know that we dropped some msgs
                        Err(RecvError::Lagged(n)) => {
                            tracing::warn!("Server dropped {n} messages for {room_name} with {} users", room_tx.receiver_count());
                            b!(sink.send(ServerEvent::Error(format!("Server is very busy and dropped {n} messages, sorry!")).as_json_str()).await);
                            continue;
                        }
                    };
                    b!(sink.send(serde_json::to_string(&peer_msg).unwrap()).await);
                }
            }
        };

        let _ = room_tx.send(ServerEvent::RoomEvent(
            self.username.clone(),
            RoomEvent::Left(room_name.clone()),
        ));
        tracing::debug!("{} disconnected (name: {})", self.addr, self.username);
        self.rooms.leave(&room_name, &self.username);
        self.users.remove(&self.username);
        should_exit(exit_result);
    }
}

fn should_exit(result: Result<(), LinesCodecError>) -> bool {
    match result {
        Ok(_) => false,
        Err(LinesCodecError::MaxLineLengthExceeded) => true,
        Err(LinesCodecError::Io(err))
            if matches!(
                err.kind(),
                ErrorKind::BrokenPipe | ErrorKind::ConnectionReset
            ) =>
        {
            true
        }
        Err(LinesCodecError::Io(err)) => {
            tracing::error!("unexpected error: {err}");
            true
        }
    }
}
