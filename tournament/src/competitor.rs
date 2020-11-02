use model::network::Connection;
use model::Bucket;

use std::collections::HashMap;
use std::io::Result as IOResult;
use std::net::{TcpListener, ToSocketAddrs};
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::game::ManagerEvents as GameManagerEvents;

use serde::Deserialize;

struct InGame {
    game_id: usize,
    connetion: Connection,
}

/// Manages connections to ai clients over TCP.
/// Handles all parts of the lifecycle and communicates with the game manager via channels.
pub struct Manager {
    /// Connected but not authenticated, the index is a random temporary id
    /// The boolean indicates whether the authentication request is pending
    unauthenticated: Bucket<(bool, Connection)>,
    /// Connected and authenticated but not in a game, the index is the actual id of the user
    /// (according to the game manager).
    /// Note: before giving this ID the game manger will check to see if the user is currently
    /// alive if so it will return an error.
    ///
    /// The game manager keeps track of it's own waiting list and it will tell us when we should
    /// move users from here to in_game.
    waiting: HashMap<usize, Connection>,
    /// Currently in a specific game, the key is the user's actual id
    in_game: HashMap<usize, InGame>,
    listener: TcpListener,
    tx: Sender<ManagerEvents>,
    rx: Receiver<GameManagerEvents>,
}

impl Manager {
    pub fn start<A: ToSocketAddrs>(
        addr: A,
    ) -> IOResult<(Sender<GameManagerEvents>, Receiver<ManagerEvents>)> {
        let (competitor_tx, game_rx) = channel();
        let (game_tx, competitor_rx) = channel();

        let mut manager = Manager {
            unauthenticated: Default::default(),
            waiting: Default::default(),
            in_game: Default::default(),
            listener: TcpListener::bind(addr)?,
            tx: competitor_tx,
            rx: competitor_rx,
        };

        manager.listener.set_nonblocking(true)?;

        std::thread::spawn(move || loop {
            manager.process_accept_requests();
            manager.check_unauthenticated();

            // TODO: in future use async IO and poll on all connections for events
            std::thread::yield_now();
        });

        Ok((game_tx, game_rx))
    }

    fn process_accept_requests(&mut self) {
        while let Ok((stream, addr)) = self.listener.accept() {
            println!("Incoming connection on {}", addr);
            stream
                .set_nonblocking(true)
                .expect("Couldn't set non-blocking on TCP connection");
            self.unauthenticated.add((false, stream.into()));
        }
    }

    fn check_unauthenticated(&mut self) {
        for temp_id in 0..self.unauthenticated.max_id() {
            if let Some((authentication_pending, connection)) =
                self.unauthenticated.get_mut(temp_id)
            {
                if !*authentication_pending {
                    match connection
                        .next_message::<AuthenticationMessage>(format!("[temp id] {}", temp_id))
                    {
                        Ok(Some(msg)) => self
                            .tx
                            .send(ManagerEvents::AuthenticationRequest {
                                username: msg.username,
                                code: msg.code,
                            })
                            .unwrap(),
                        Ok(None) => {}
                        Err(_) => {
                            println!("Disconnecting temp id {}", temp_id);
                            self.unauthenticated.remove(temp_id);
                        }
                    }
                }
            }
        }
    }

    fn check_game_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                GameManagerEvents::Authenticated { temporary_id, id } => {
                    if let Some((_, connection)) = self.unauthenticated.remove(temporary_id) {
                        println!(
                            "Authenticated temp id {}, they now have id {}",
                            temporary_id, id
                        );
                        // TODO: tell user they're authenticated
                        self.waiting.insert(id, connection);
                    } else {
                        unreachable!(
                            "Once the authentication request is sent to the game there \
                            should be no more polling on the connection hence we won't \
                            know if it disconnected so it must still be in self.unauthenticated (successful)"
                        );
                    }
                }
                GameManagerEvents::BadAuthentication {
                    temporary_id,
                    reason,
                } => {
                    // TODO: Send error back to client
                    if let Some((_, _connection)) = self.unauthenticated.remove(temporary_id) {
                        println!(
                            "Received bad authentcation from {} ({:?})",
                            temporary_id, reason
                        );

                        self.unauthenticated.remove(temporary_id);
                    } else {
                        unreachable!(
                            "Once the authentication request is sent to the game there \
                            should be no more polling on the connection hence we won't \
                            know if it disconnected so it must still be in self.unauthenticated"
                        );
                    }
                }
            }
        }
    }
}

/// Events that the competitor manager produces
pub enum ManagerEvents {
    AuthenticationRequest { username: String, code: String },
}

#[derive(Deserialize)]
struct AuthenticationMessage {
    username: String,
    code: String,
}
