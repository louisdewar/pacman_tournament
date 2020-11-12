use model::{
    network::{create_tick_message, ActionMessage, PlayerDiedMessage, TickMessage},
    Bucket,
};

use std::collections::{HashMap, HashSet};
use std::io::Result as IOResult;
use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;

use crate::authentication::{
    AuthenticationEvent, AuthenticationFailedReason, AuthenticationRequest,
};
use crate::game::ManagerEvent as GameManagerEvent;
use crate::StreamMapBucket;

use serde::Deserialize;

use crate::connection::MessageStream;

struct InGame {
    game_id: usize,
    in_game_player_id: usize,
}

#[derive(serde::Serialize)]
enum CompetitorMessage {
    #[serde(rename(serialize = "error"))]
    Error(String),
    #[serde(rename(serialize = "died"))]
    Died(PlayerDiedMessage),
    #[serde(rename(serialize = "spawned"))]
    Spawned(PlayerSpawnedMessage),
    #[serde(rename(serialize = "tick"))]
    Tick(TickMessage),
}

/// Manages connections to ai clients over TCP.
/// Handles all parts of the lifecycle and communicates with the game manager via channels.
pub struct Manager {
    /// Connected but not authenticated, the index is a random temporary id
    /// The boolean indicates whether the authentication request is pending
    unauthenticated: StreamMapBucket<MessageStream, bool>,
    /// Connected and authenticated but not in a game (in spawn queue), the index is the actual id of the user
    /// (according to the game manager).
    /// Note: before giving this ID the game manger will check to see if the user is currently
    /// alive if so it will return an error.
    ///
    /// The game manager keeps track of it's own waiting list and it will tell us when we should
    /// move users from here to in_game.
    spawning: StreamMapBucket<MessageStream, ()>,
    /// Currently in a specific game, the key is the user's actual id
    in_game: StreamMapBucket<MessageStream, InGame>,
    listener: TcpListener,
    authentication_request_sender: Sender<AuthenticationRequest>,
    tx: Sender<ManagerEvent>,
    rx: Receiver<IncomingEvent>,
}

impl Manager {
    pub async fn start<A: ToSocketAddrs>(
        addr: A,
        authentication_request_sender: Sender<AuthenticationRequest>,
        competitor_tx: Sender<ManagerEvent>,
        competitor_rx: Receiver<IncomingEvent>,
    ) -> IOResult<JoinHandle<()>> {
        let mut manager = Manager {
            unauthenticated: Default::default(),
            spawning: Default::default(),
            in_game: Default::default(),
            listener: TcpListener::bind(addr).await?,
            authentication_request_sender,
            tx: competitor_tx,
            rx: competitor_rx,
        };

        let handle = tokio::task::spawn(async move {
            use crate::stream_map_bucket::StreamMapEvent;
            loop {
                tokio::select! {
                    Ok((stream, address)) = manager.listener.accept() => {
                        stream.set_nodelay(true).expect("Couldn't set nodelay on TCP stream");
                        manager.handle_incoming_connection(stream, address).await;
                    }
                    Some(event) = manager.unauthenticated.next() => {
                        match event {
                            StreamMapEvent::Message(temp_id, msg) => manager.handle_unauthenticated_message(temp_id, msg).await,
                            // In process streams are marked as not disconnectable so only non
                            // inprogress streams will be safely disconnected here
                            StreamMapEvent::Disconnection(temp_id, in_progress) => {
                                assert!(!in_progress, "invarient broken, disconnected stream when authentication was in progress");
                                println!("Temp id {} disconnected", temp_id);
                            }
                        }
                    }
                    // If a player is spawning then their disconnectable is set to false
                    Some(StreamMapEvent::Message(id, _msg)) = manager.spawning.next() => {
                        println!("User {} sent message even though they were in the waiting state", id);

                    }
                    Some(event) = manager.in_game.next() => {
                        match event {
                            StreamMapEvent::Message(id, msg) => manager.handle_ingame_message(id, msg).await,
                            StreamMapEvent::Disconnection(user_id, meta) => {
                                println!("User with id {} disconnected, there were in game {}", user_id, meta.game_id);
                                manager.tx.send(ManagerEvent::PlayerDisconnected {
                                    user_id,
                                    game_id: meta.game_id,
                                    in_game_player_id: meta.in_game_player_id,
                                }).await.unwrap();
                            }
                        }
                    }
                    Some(event) = manager.rx.recv() => {
                        match event {
                            IncomingEvent::Game(event) => manager.handle_game_event(event).await,
                            IncomingEvent::Authentication(event) => manager.handle_authentication_event(event).await,
                        }
                    }
                }
            }
        });

        Ok(handle)
    }

    async fn handle_incoming_connection(&mut self, stream: TcpStream, address: SocketAddr) {
        let temp_id = self
            .unauthenticated
            .add_stream(MessageStream::new(stream, "".to_string()), false);
        println!("Connection from {} given temp id {}", address, temp_id);
        self.unauthenticated
            .get_stream_mut(temp_id)
            .unwrap()
            .set_id(format!("[temp id] {}", temp_id));
    }

    async fn handle_unauthenticated_message(&mut self, temporary_id: usize, msg: String) {
        if *self.unauthenticated.get_metadata(temporary_id).unwrap() {
            println!("[temp id] already has their authentication pending");
            return;
        }

        let msg: AuthenticationMessage = match serde_json::from_str(&msg) {
            Ok(msg) => msg,
            Err(_) => {
                println!(
                    "User [temp id] {} sent invalid authentication message",
                    temporary_id
                );
                return;
            }
        };

        *self.unauthenticated.get_metadata_mut(temporary_id).unwrap() = true;
        // Race condition if we don't ensure that it can't be disconnected
        self.unauthenticated.set_disconnectable(temporary_id, false);

        self.authentication_request_sender
            .send(AuthenticationRequest {
                username: msg.username,
                code: msg.code,
                temporary_id,
            })
            .await
            .unwrap();
    }

    async fn handle_ingame_message(&mut self, user_id: usize, msg: String) {
        let ActionMessage { action, tick } = match serde_json::from_str(&msg) {
            Ok(msg) => msg,
            Err(_) => {
                println!("User {} sent invalid action message", user_id);
                return;
            }
        };

        self.tx
            .send(ManagerEvent::Action {
                user_id,
                action,
                tick,
            })
            .await
            .unwrap();
    }

    async fn handle_game_event(&mut self, event: GameManagerEvent) {
        match event {
            GameManagerEvent::PlayerDied {
                user_id,
                final_score,
            } => {
                let (_, mut stream) = self.in_game.remove_stream(user_id).unwrap();
                let _ = stream
                    .send(
                        serde_json::to_string(&CompetitorMessage::Died(PlayerDiedMessage {
                            final_score,
                        }))
                        .unwrap(),
                    )
                    .await;
            }
            GameManagerEvent::PlayerSpawned {
                user_id,
                in_game_player_id,
                game_id,
            } => {
                println!("Player with id {} spawned in game {}", user_id, game_id);
                let (_, mut stream) = self.spawning.remove_stream(user_id).unwrap();
                let _ = stream
                    .send(
                        serde_json::to_string(&CompetitorMessage::Spawned(PlayerSpawnedMessage {
                            game_id,
                        }))
                        .unwrap(),
                    )
                    .await;
                self.in_game.insert_stream(
                    user_id,
                    stream,
                    InGame {
                        game_id,
                        in_game_player_id,
                    },
                    true,
                );
            }
            GameManagerEvent::ProcessTick {
                game_data,
                tick,
                id_map,
                ..
            } => {
                for (player_id, user_id) in id_map.into_iter() {
                    // The user may have disconnected between sending the action and the tick ending
                    if let Some(stream) = self.in_game.get_stream_mut(user_id) {
                        let tick_msg = create_tick_message(&game_data, player_id, tick);
                        let _ = stream
                            .send(
                                serde_json::to_string(&CompetitorMessage::Tick(tick_msg)).unwrap(),
                            )
                            .await;
                    }
                }
            }
        }
    }

    async fn handle_authentication_event(&mut self, event: AuthenticationEvent) {
        match event {
            AuthenticationEvent::Authenticated { temporary_id, id } => {
                println!(
                    "Player with temporary id {} was authenticated and given id {}",
                    temporary_id, id
                );
                let (_, stream) = self.unauthenticated.remove_stream(temporary_id).unwrap();
                self.spawning.insert_stream(id, stream, (), false);
            }
            AuthenticationEvent::BadAuthentication {
                temporary_id,
                reason,
            } => {
                let (_, mut stream) = self.unauthenticated.remove_stream(temporary_id).unwrap();
                stream
                    .send(
                        serde_json::to_string(&CompetitorMessage::Error(reason.to_message()))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
            }
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct PlayerSpawnedMessage {
    game_id: usize,
}

/// Events that the competitor manager produces
#[derive(Clone, Debug)]
pub enum ManagerEvent {
    /// Represents a user who has provided the correct code, it contains the user's globally
    /// unique id from the database.
    Authenticated {
        username: String,
        user_id: usize,
        temporary_id: usize,
        high_score: u32,
    },
    Action {
        user_id: usize,
        action: model::Action,
        tick: u32,
    },
    PlayerDisconnected {
        user_id: usize,
        game_id: usize,
        in_game_player_id: usize,
    },
}

#[derive(Clone, Debug)]
pub enum IncomingEvent {
    Game(GameManagerEvent),
    Authentication(AuthenticationEvent),
}

#[derive(Deserialize)]
struct AuthenticationMessage {
    username: String,
    code: String,
}
