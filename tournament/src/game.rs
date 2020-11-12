use std::collections::HashMap;
use std::time::Duration;

use model::{Action, Bucket, GameData, GameEvent, Map, Model};

use crate::competitor::IncomingEvent as CompetitorIncomingEvent;
use crate::competitor::ManagerEvent as CompetitorManagerEvent;
use crate::score::ScoreUpdate;
use crate::spectator::SpectatorEvent;

use tokio::stream::StreamExt;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{interval, Interval};

const MAX_MOBS: usize = 8;
const MAX_PLAYERS: usize = 8;

pub struct Player {
    username: String,
    high_score: u32,
    game_id: usize,
    in_game_player_id: usize,
}

/// Manages all the games on the server
pub struct GlobalManager {
    /// The id here is the **global** user id, it will not be the same id as the one generated when
    /// a user joins a game.
    ingame_players: HashMap<usize, Player>,
    spawning_players: HashMap<usize, Player>,
    games: Bucket<LocalManager>,
    tick_interval: Interval,
    rx: Receiver<CompetitorManagerEvent>,
    competitor_tx: Sender<CompetitorIncomingEvent>,
    score_tx: Sender<ScoreUpdate>,
    spectator_tx: Sender<SpectatorEvent>,
    map: Map,
}

/// Manages a single game
struct LocalManager {
    model: Model,
    /// Maps from the in game player id to the user id
    /// It is only populated once a player has been spawned
    id_map: HashMap<usize, usize>,
    game_id: usize,
}

impl LocalManager {
    fn new(map: Map, game_id: usize) -> LocalManager {
        let model = Model::new(map, MAX_MOBS);

        LocalManager {
            model,
            id_map: Default::default(),
            game_id,
        }
    }

    /// Checks to see if there is a slot available / if there is a free spawnlocation in the game,
    /// if so it will add the player to the spawn queue and return true.
    /// The in game id will be generated once the player has been spawned.
    fn try_spawn_player(&mut self, temporary_id: usize, username: String) -> bool {
        if self.total_player_count() < MAX_PLAYERS {
            self.model.add_client(temporary_id, username);
            true
        } else {
            false
        }
    }

    fn total_player_count(&self) -> usize {
        self.model.players().len() + self.model.spawning_players().len()
    }

    fn remove_client(&mut self, player_id: usize) -> model::Player {
        self.id_map.remove(&player_id);
        self.model.remove_client(player_id)
    }

    fn play_action(&mut self, player_id: usize, action: Action, tick: u32) {
        self.model.player_action(player_id, action, tick);
    }

    fn should_close(&self) -> bool {
        self.total_player_count() == 0
    }

    fn tick<F: FnMut(GameEvent)>(&mut self, mut cb: F) {
        let id_map = &mut self.id_map;
        self.model.simulate_tick(|event| {
            // This is the only way we find out about dead players, we have to update the id_map
            // when they die
            match event {
                // Pass along the event to the callback with the user id instead
                GameEvent::PlayerDied {
                    player_id,
                    final_score,
                } => {
                    let user_id = id_map.remove(&player_id).unwrap();
                    cb(GameEvent::PlayerDied {
                        player_id: user_id,
                        final_score,
                    })
                }
                event => cb(event),
            }
        });
    }

    fn players(&self) -> &model::PlayerBucket {
        self.model.players()
    }
}

impl GlobalManager {
    pub fn start(
        rx: Receiver<CompetitorManagerEvent>,
        competitor_tx: Sender<CompetitorIncomingEvent>,
        score_tx: Sender<ScoreUpdate>,
        spectator_tx: Sender<SpectatorEvent>,
        map: Map,
    ) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn(async move {
            let mut manager = GlobalManager {
                ingame_players: Default::default(),
                spawning_players: Default::default(),
                games: Default::default(),
                tick_interval: interval(Duration::from_millis(500)),
                rx,
                competitor_tx,
                score_tx,
                spectator_tx,
                map,
            };

            loop {
                tokio::select! {
                    Some(event) = manager.rx.next() => {
                        manager.handle_incoming_event(event).await;
                    }
                    _ = manager.tick_interval.tick() => {
                        manager.tick_games().await;
                    }
                }
            }
        })
    }

    async fn tick_games(&mut self) {
        // TODO: consider changing interval to instant and then advance by a second only once all the
        // games have been processed.
        // Having an instant like this will greatly simplify automatically advancing the tick when
        // all actions are in.
        let mut game_events = Vec::new();
        let mut closing_games = Vec::new();
        for (game_id, game) in self.games.iter_mut() {
            game.tick(|event| game_events.push((*game_id, event)));

            if game.should_close() {
                closing_games.push(*game_id);
            }
        }

        for (game_id, event) in game_events {
            self.handle_game_event(game_id, event).await;
        }

        for game_id in closing_games {
            self.close_game(game_id).await;
        }
    }

    async fn close_game(&mut self, game_id: usize) {
        println!("Game {} closed", game_id);
        self.games.remove(game_id);
        self.spectator_tx
            .send(SpectatorEvent::GameClosed { game_id })
            .await
            .unwrap();
    }

    async fn handle_game_event(&mut self, game_id: usize, event: GameEvent) {
        match event {
            GameEvent::PlayerDied {
                // This event is mapped such that the player id is the user id
                player_id: user_id,
                final_score,
            } => {
                self.competitor_tx
                    .send(CompetitorIncomingEvent::Game(ManagerEvent::PlayerDied {
                        user_id,
                        final_score,
                    }))
                    .await
                    .unwrap();

                let player = if let Some(player) = self.ingame_players.remove(&user_id) {
                    player
                } else {
                    println!("Player with user id {} doesn't exist but they died (they may have very recently disconnected)", user_id);
                    return;
                };
                let in_game_player_id = player.in_game_player_id;

                self.spectator_tx
                    .send(SpectatorEvent::PlayerLeft {
                        game_id,
                        user_id,
                        in_game_player_id,
                    })
                    .await
                    .unwrap();
            }
            GameEvent::ProcessTick { game_data, tick } => {
                // it's possible for the game to be deleted if there are no players but the tick
                // was still in the backlog (unlikely but possible race condition).
                // This will be mitigated by avoiding using channels for game ticks and having a
                // callback directly on the tick function so that way events can be processed in
                // sync
                if let Some(game) = self.games.get_mut(game_id) {
                    let player_scores = game
                        .players()
                        .iter()
                        .map(|(player_id, player)| {
                            (
                                *game.id_map.get(player_id).unwrap(),
                                player.borrow().score(),
                            )
                        })
                        .collect();
                    self.score_tx
                        .send(ScoreUpdate {
                            game_id,
                            player_scores,
                        })
                        .await
                        .unwrap();
                    let game_data_clone = game_data.clone();
                    self.competitor_tx
                        .send(CompetitorIncomingEvent::Game(ManagerEvent::ProcessTick {
                            game_id,
                            game_data: game_data_clone,
                            tick,
                            id_map: game.id_map.clone(),
                        }))
                        .await
                        .unwrap();

                    self.spectator_tx
                        .send(SpectatorEvent::Tick { game_id, game_data })
                        .await
                        .unwrap();
                } else {
                    println!(
                        "Got tick for game that didn't exist tick={} game_id={}",
                        tick, game_id
                    );
                }
            }
            GameEvent::PlayerSpawned {
                temporary_id: user_id,
                id: in_game_player_id,
            } => {
                let game = self.games.get_mut(game_id).unwrap();
                // temporary_id is the global user id in this case
                game.id_map.insert(in_game_player_id, user_id);

                let mut player = self.spawning_players.remove(&user_id).unwrap();
                player.in_game_player_id = in_game_player_id;
                let high_score = player.high_score;
                let username = player.username.clone();
                self.ingame_players.insert(user_id, player);

                self.competitor_tx
                    .send(CompetitorIncomingEvent::Game(ManagerEvent::PlayerSpawned {
                        game_id,
                        in_game_player_id,
                        user_id,
                    }))
                    .await
                    .unwrap();

                self.spectator_tx
                    .send(SpectatorEvent::PlayerSpawned {
                        user_id,
                        in_game_player_id,
                        game_id,
                        prev_high_score: high_score,
                        username,
                    })
                    .await
                    .unwrap();
            }
        }
    }

    async fn add_player_to_game(&mut self, user_id: usize, username: String) -> usize {
        for (game_id, game) in self.games.iter_mut() {
            if game.try_spawn_player(user_id, username.clone()) {
                return *game_id;
            }
        }

        let i = self.games.minimum_available_id();

        let mut game = LocalManager::new(self.map.clone(), i);
        let game_data = game.model.data().clone();
        self.spectator_tx
            .send(SpectatorEvent::GameOpened {
                game_id: i,
                game_data,
            })
            .await
            .unwrap();
        assert!(game.try_spawn_player(user_id, username));
        assert!(self.games.insert(i, game).is_none());

        println!(
            "Created a new instance for user {}, there are now {} games",
            user_id,
            self.games.len()
        );

        i
    }

    async fn handle_incoming_event(&mut self, event: CompetitorManagerEvent) {
        use CompetitorManagerEvent as E;
        match event {
            E::Authenticated {
                username,
                temporary_id,
                user_id,
                high_score,
            } => {
                if self.ingame_players.contains_key(&user_id)
                    || self.spawning_players.contains_key(&user_id)
                {
                    self.competitor_tx
                        .send(CompetitorIncomingEvent::Authentication(
                            crate::authentication::AuthenticationEvent::BadAuthentication {
                                temporary_id,
                                reason:
                                    crate::authentication::AuthenticationFailedReason::PlayerInGame,
                            },
                        ))
                        .await
                        .unwrap();
                    return;
                }

                // This message comes from the authentication manager, right now the competitor
                // manager does not know
                self.competitor_tx
                    .send(CompetitorIncomingEvent::Authentication(
                        crate::authentication::AuthenticationEvent::Authenticated {
                            temporary_id,
                            id: user_id,
                        },
                    ))
                    .await
                    .unwrap();

                let game_id = self.add_player_to_game(user_id, username.clone()).await;

                self.spawning_players.insert(
                    user_id,
                    Player {
                        username,
                        high_score,
                        game_id,
                        // Default to 0 and then change once we have the actual value
                        in_game_player_id: 0,
                    },
                );
            }
            E::Action {
                user_id,
                action,
                tick,
            } => {
                let player = match self.ingame_players.get(&user_id) {
                    Some(player) => player,
                    None => {
                        // This is a race condition but we handle is gracefully so it's not an
                        // issue but we might as well log it just in case
                        println!("User {} played action when they weren't in a game", user_id);
                        return;
                    }
                };

                let game = match self.games.get_mut(player.game_id) {
                    Some(game) => game,
                    None => {
                        println!("User {} played action but the game didn't exist", user_id);
                        return;
                    }
                };

                game.play_action(player.in_game_player_id, action, tick);
            }
            E::PlayerDisconnected {
                user_id,
                game_id,
                in_game_player_id,
            } => {
                if let Some(game) = self.games.get_mut(game_id) {
                    game.remove_client(in_game_player_id);
                    self.ingame_players.remove(&user_id);
                } else {
                    println!("Player disconnected from game which they were not a part of");
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ManagerEvent {
    PlayerSpawned {
        user_id: usize,
        in_game_player_id: usize,
        game_id: usize,
    },
    ProcessTick {
        game_id: usize,
        game_data: GameData,
        id_map: HashMap<usize, usize>,
        tick: u32,
    },
    PlayerDied {
        user_id: usize,
        final_score: u32,
    },
}
