use tokio_tungstenite::{tungstenite::Message as WebSocketMessage, WebSocketStream};

use model::GameData;

use std::collections::HashMap;
use std::sync::Arc;

use crate::PgPool;

use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::stream::StreamExt;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Receiver;
use tokio::time::{interval, Interval};

use futures_util::SinkExt;

mod delta;
mod initial;
mod message;
mod serialize;

const LEADERBOARD_LIMIT: i64 = 10;

enum ListenFilter {
    /// All games + top 10 leaderboard
    AllGames,
    /// A specific game (given by the game id)
    Game(usize),
}

impl ListenFilter {
    fn listening_for_game(&self, game_id: usize) -> bool {
        match &self {
            ListenFilter::AllGames => true,
            ListenFilter::Game(listen_game_id) if *listen_game_id == game_id => true,
            _ => false,
        }
    }
}

struct Spectator {
    socket: WebSocketStream<TcpStream>,
    filter: ListenFilter,
    rx: broadcast::Receiver<BroadcastEvent>,
}

impl Spectator {
    fn start(
        stream: TcpStream,
        filter: ListenFilter,
        rx: broadcast::Receiver<BroadcastEvent>,
        games: HashMap<usize, (HashMap<usize, Player>, GameData)>,
    ) {
        tokio::task::spawn(async move {
            let socket = if let Ok(socket) = tokio_tungstenite::accept_async(stream).await {
                socket
            } else {
                println!("There was an error during the spectator websocket handshake aborting...");
                return;
            };

            let mut spectator = Spectator { socket, filter, rx };

            for (game_id, (id_map, game_data)) in games {
                let serialized_initial = serialize::serialized_initial(
                    &initial::create_initial_message(game_id, &game_data, &id_map),
                );

                if spectator
                    .socket
                    .send(WebSocketMessage::Text(serialized_initial))
                    .await
                    .is_err()
                {
                    println!("Closing socket since there was an error sending inital packet");
                    let _ = spectator.socket.close(None);
                    return;
                }
            }

            'outer: loop {
                tokio::select! {
                    recv_event = spectator.rx.recv() => {
                        match recv_event {
                            Ok(event) => spectator.handle_event(event).await,
                            // Either server has closed or we've fallen behind quite a bit, so we
                            // should close the connection
                            Err(_) => {
                                println!("Closing spectator connection due to recv error");
                                break 'outer;
                            }
                        }
                    }
                    ws_event = spectator.socket.next() => {
                        match ws_event {
                            Some(Ok(msg)) => spectator.handle_message(msg),
                            Some(Err(_)) => {
                                println!("Closing spectator connection due to ws error");
                                break 'outer;
                            }
                            None => {
                                break 'outer;
                            }
                        }
                    }
                }
            }

            let _ = spectator.socket.close(None);
        });
    }

    fn handle_message(&mut self, msg: WebSocketMessage) {
        match msg {
            WebSocketMessage::Text(_txt) => todo!(),
            // We don't care about other message types
            _ => {}
        }
    }

    async fn handle_event(&mut self, event: BroadcastEvent) {
        match event {
            BroadcastEvent::TickUpdate {
                game_id,
                serialized_delta,
            } => {
                if self.filter.listening_for_game(game_id) {
                    if self
                        .socket
                        .send(WebSocketMessage::Text(String::clone(&serialized_delta)))
                        .await
                        .is_err()
                    {
                        println!("Closing socket since there was an error sending delta");
                        let _ = self.socket.close(None);
                    }
                }
            }
            BroadcastEvent::GameClosed { game_id } => {
                if self.filter.listening_for_game(game_id) {
                    if self
                        .socket
                        .send(WebSocketMessage::Text(format!("c{}", game_id)))
                        .await
                        .is_err()
                    {
                        println!("Closing socket since there was an error sending closed");
                        let _ = self.socket.close(None);
                    }
                }
            }
            BroadcastEvent::GameOpened {
                game_id,
                serialized_initial,
            } => {
                if self.filter.listening_for_game(game_id) {
                    if self
                        .socket
                        .send(WebSocketMessage::Text(String::clone(&serialized_initial)))
                        .await
                        .is_err()
                    {
                        println!("Closing socket since there was an error sending initial");
                        let _ = self.socket.close(None);
                    }
                }
            }
            BroadcastEvent::LeaderboardUpdate { serialized } => {
                if self
                    .socket
                    .send(WebSocketMessage::Text(serialized))
                    .await
                    .is_err()
                {
                    println!("Closing socket since there was an error sending leaderboard update");
                    let _ = self.socket.close(None);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    user_id: usize,
    username: String,
    prev_high_score: u32,
}

/// Manages connections to the websocket clients (spectators)
pub struct Manager {
    /// The inner hashmap if a map from the in game player id to the user id.
    games: HashMap<usize, (HashMap<usize, Player>, GameData)>,
    ws_listener: TcpListener,
    rx: Receiver<SpectatorEvent>,
    broadcaster: broadcast::Sender<BroadcastEvent>,
    score_update_interval: Interval,
    pool: PgPool,
}

impl Manager {
    pub async fn start<A: ToSocketAddrs>(
        addr: A,
        rx: Receiver<SpectatorEvent>,
        pool: PgPool,
    ) -> tokio::task::JoinHandle<()> {
        let (broadcaster, _) = broadcast::channel(5);

        let mut manager = Manager {
            games: Default::default(),
            ws_listener: TcpListener::bind(addr)
                .await
                .expect("Failed to start the spectator ws server"),
            rx,
            broadcaster,
            score_update_interval: interval(std::time::Duration::from_millis(3000)),
            pool,
        };

        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    Ok((stream, _)) = manager.ws_listener.accept() => {
                        manager.handle_incoming_websocket(stream);
                    }
                    Some(event) = manager.rx.next() => {
                        manager.handle_event(event);
                    }
                    _ = manager.score_update_interval.tick() => {
                        manager.send_score_update()
                    }
                }
            }
        })
    }

    fn send_score_update(&self) {
        let broadcaster = self.broadcaster.clone();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let conn = pool
                .get()
                .expect("Score manager lost connection to the database");

            let leaderboard = db::actions::get_leaderboard(&conn, Some(LEADERBOARD_LIMIT));
            let _ = broadcaster.send(BroadcastEvent::LeaderboardUpdate {
                serialized: serialize::serialize_leaderboard(&leaderboard),
            });
        });
    }

    fn handle_incoming_websocket(&mut self, stream: TcpStream) {
        // TODO: In future I will try to figure out a way of avoiding the clone since the data is
        // basically immutable since we receive a new owned value over the broadcast each tick.
        // The issue is that internally the gamedata contains refcells so they can't be put in an
        // Arc.
        // Maybe the model should transform the data to a format that has less to do with internal
        // implementation details.
        Spectator::start(
            stream,
            ListenFilter::AllGames,
            self.broadcaster.subscribe(),
            self.games.clone(),
        );
    }

    fn handle_event(&mut self, event: SpectatorEvent) {
        match event {
            SpectatorEvent::GameOpened { game_data, game_id } => {
                let initial = initial::create_initial_message(game_id, &game_data, &HashMap::new());
                let _ = self.broadcaster.send(BroadcastEvent::GameOpened {
                    game_id,
                    serialized_initial: Arc::new(serialize::serialized_initial(&initial)),
                });
                assert!(
                    self.games
                        .insert(game_id, (HashMap::new(), game_data))
                        .is_none(),
                    "Game {} opened but it already existed",
                    game_id
                );
            }
            SpectatorEvent::GameClosed { game_id } => {
                assert!(
                    self.games.remove(&game_id).is_some(),
                    "Game {} closed but it didn't exist",
                    game_id
                );
                let _ = self
                    .broadcaster
                    .send(BroadcastEvent::GameClosed { game_id });
            }
            SpectatorEvent::PlayerSpawned {
                user_id,
                in_game_player_id,
                game_id,
                prev_high_score,
                username,
            } => {
                self.games
                    .get_mut(&game_id)
                    .expect("Player spawned in game which didn't exist")
                    .0
                    .insert(
                        in_game_player_id,
                        Player {
                            username,
                            prev_high_score,
                            user_id,
                        },
                    );
            }
            SpectatorEvent::PlayerLeft {
                game_id,
                in_game_player_id,
                ..
            } => {
                self.games
                    .get_mut(&game_id)
                    .expect("Player left game which didn't exist")
                    .0
                    .remove(&in_game_player_id);
            }
            SpectatorEvent::Tick {
                game_data: new_game_data,
                game_id,
            } => {
                let game = self
                    .games
                    .get_mut(&game_id)
                    .expect("Tick happened in game which didn't exist");

                let delta_message =
                    delta::create_delta_message(game_id, &game.1, &new_game_data, &game.0);
                game.1 = new_game_data;

                self.send_delta(game_id, delta_message);
            }
        }
    }

    fn send_delta(&mut self, game_id: usize, delta_message: message::DeltaMessage) {
        // it's okay for this to fail e.g. if there are no spectators currently
        let _ = self.broadcaster.send(BroadcastEvent::TickUpdate {
            game_id,
            serialized_delta: Arc::new(serialize::serialized_delta(&delta_message)),
        });
    }
}

#[derive(Clone, Debug)]
pub enum BroadcastEvent {
    GameOpened {
        game_id: usize,
        serialized_initial: Arc<String>,
    },
    TickUpdate {
        game_id: usize,
        serialized_delta: Arc<String>,
    },
    GameClosed {
        game_id: usize,
    },
    LeaderboardUpdate {
        serialized: String,
    },
}

#[derive(Clone, Debug)]
pub enum SpectatorEvent {
    GameOpened {
        game_data: GameData,
        game_id: usize,
    },
    GameClosed {
        game_id: usize,
    },
    PlayerSpawned {
        user_id: usize,
        in_game_player_id: usize,
        game_id: usize,
        prev_high_score: u32,
        username: String,
    },
    PlayerLeft {
        user_id: usize,
        in_game_player_id: usize,
        game_id: usize,
    },
    Tick {
        game_id: usize,
        game_data: GameData,
    },
}
