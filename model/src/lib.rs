use std::cell::RefCell;

mod map;
pub use map::{BaseTile, Map, SpawnLocation};

mod entity;
pub use entity::{Action, Direction, Entity, EntityIndex, EntityType, Mob, Player};

mod food;
pub use food::Food;

mod bucket;
pub use bucket::Bucket;

pub mod network;
pub use network::{NetworkManager, NetworkMessage};

mod animation;
pub use animation::Animation;

mod grid;
pub use grid::Grid;

use std::collections::VecDeque;
pub use std::sync::mpsc::{Receiver, Sender};
pub use std::time::Instant;

pub type MobBucket = Bucket<RefCell<Mob>>;
pub type PlayerBucket = Bucket<RefCell<Player>>;

#[derive(Debug, Clone)]
pub struct GameData {
    pub map: Map,
    pub entities: Grid<Option<EntityIndex>>,
    pub food: Grid<Option<Food>>,
    pub players: PlayerBucket,
    pub mobs: MobBucket,
}

/// An event produced by the game model
#[derive(Debug, Clone)]
pub enum GameEvent {
    PlayerSpawned { temporary_id: usize, id: usize },
    ProcessTick { game_data: GameData, tick: u32 },
    PlayerDied { player_id: usize, final_score: u32 },
}

pub struct Model {
    data: GameData,
    spawning_players: VecDeque<(usize, String)>,
    desired_mob_count: usize,
    tick: u32,
    tick_start: Instant,
    /// The number of players we're waiting so submit an action
    waiting_players: usize,
}

impl Model {
    pub fn new(map: Map, desired_mob_count: usize) -> Model {
        Model {
            data: GameData {
                entities: Grid::fill_with_clone(None, map.width() as usize, map.height() as usize),
                food: map.default_food_locations().clone(),
                map,
                players: Bucket::new(),
                mobs: Bucket::new(),
            },
            tick: 0,
            spawning_players: VecDeque::new(),
            tick_start: Instant::now(),
            desired_mob_count,
            waiting_players: 0,
        }
    }

    pub fn add_entity(&mut self, (x, y): (u16, u16), index: EntityIndex) {
        self.data.entities[x as usize][y as usize] = Some(index);
    }

    fn spawn_player(&mut self, username: String) -> Option<usize> {
        let (x, y) = match self.spawn_location(&self.data.map.player_spawn()) {
            Some(location) => location,
            None => return None,
        };

        let id = self.data.players.add(RefCell::new(Player::new(
            (x, y),
            Direction::North,
            1,
            2,
            username,
            None,
        )));

        self.add_entity((x, y), EntityIndex::new_player(id));

        Some(id)
    }

    pub fn spawn_mob(&mut self) -> bool {
        let (x, y) = match self.spawn_location(&self.data.map.mob_spawn()) {
            Some(location) => location,
            None => return false,
        };

        let id = self
            .data
            .mobs
            .add(RefCell::new(Mob::new((x, y), Direction::North, false)));

        self.add_entity((x, y), EntityIndex::new_mob(id));

        true
    }

    /// Takes in a spawn location and either selects one of the specified points or if the spawn
    /// location is Random then searches through the map for available spawn locations (is land and
    /// there are no entities on it). If there is no suitable location then this will return None.
    fn spawn_location(&self, spawn_location: &SpawnLocation) -> Option<(u16, u16)> {
        use rand::prelude::*;

        let candidates: Vec<_> = match spawn_location {
            SpawnLocation::Defined(points) => points
                .iter()
                .filter(|(x, y)| self.base_tile(*x as usize, *y as usize) == &BaseTile::Land)
                .filter(|(x, y)| self.entity(*x as usize, *y as usize).is_none())
                .cloned()
                .collect(),
            SpawnLocation::Random => {
                let mut points = Vec::new();
                for x in 0..self.data.map.width() {
                    for y in 0..self.data.map.height() {
                        if self.base_tile(x as usize, y as usize) != &BaseTile::Land {
                            continue;
                        }

                        if self.entity(x as usize, y as usize).is_some() {
                            continue;
                        }

                        points.push((x, y));
                    }
                }

                points
            }
        };

        let mut rng = rand::thread_rng();
        candidates.choose(&mut rng).map(|point| *point)
    }

    pub fn map(&self) -> &Map {
        &self.data.map
    }

    fn base_tile(&self, x: usize, y: usize) -> &BaseTile {
        &self.data.map.base_tile(x, y)
    }

    fn entity(&self, x: usize, y: usize) -> &Option<EntityIndex> {
        &self.data.entities[x][y]
    }

    fn respawn_food(&mut self) {
        self.data.food = self.data.map.default_food_locations().clone();
    }

    /// Performs a single tick applying all the stored actions for each player.
    ///
    /// Simulation order:
    /// 1. Mobs/Players (top left to bottom right row by row) - when entites die they are removed
    ///    from the grid immediately but their object still exists in the bucket (it is tidied
    ///    up in the next step)
    /// 2. Send messages about dead players (also clean up dead mobs)
    /// 3. Spawn new players (if there is space)
    ///
    /// TODO: make tick take in a closure to remove the global callback
    pub fn simulate_tick<F: FnMut(GameEvent)>(&mut self, mut callback: F) {
        self.tick += 1;
        self.tick_start = Instant::now();

        // Every 50 ticks we respawn the food
        if self.tick % 50 == 0 {
            self.respawn_food();
        }

        // Note: this clone only clones the EntityIndex's not the entities themselves.
        // It is required since if we are to loop fairly in an order based on position,
        // during the tick positions of entites will change so we must store an older version
        // to use as the order.
        let entity_queue: Vec<EntityIndex> = self
            .data
            .entities
            .as_ref()
            .iter()
            .filter_map(|e| e.clone())
            .collect();

        let entities = &mut self.data.entities;
        let food = &mut self.data.food;
        let mobs = &self.data.mobs;
        let players = &self.data.players;
        let map = &self.data.map;

        for entity_index in entity_queue {
            match entity_index.entity_type() {
                EntityType::Mob => mobs.get(entity_index.index()).map(|m| {
                    // We shouldn't process the turn for a dead entity
                    if !m.borrow().died() {
                        m.borrow_mut()
                            .process_turn(entities, food, mobs, players, map)
                    }
                }),
                EntityType::Player => players.get(entity_index.index()).map(|p| {
                    if !p.borrow().died() {
                        p.borrow_mut()
                            .process_turn(entities, food, mobs, players, map)
                    }
                }),
            };
        }

        for player_id in 0..self.data.players.max_id() {
            if let Some(player) = self.data.players.get(player_id) {
                if player.borrow().died() {
                    let player = self.data.players.remove(player_id).unwrap();

                    callback(GameEvent::PlayerDied {
                        player_id,
                        final_score: player.borrow().score(),
                    });
                }
            }
        }

        for mob_id in 0..self.data.mobs.max_id() {
            if let Some(mob) = self.data.mobs.get(mob_id) {
                if mob.borrow().died() {
                    self.data.mobs.remove(mob_id);
                }
            }
        }

        while let Some((temporary_id, username)) = self.spawning_players.pop_front() {
            if let Some(id) = self.spawn_player(username.clone()) {
                callback(GameEvent::PlayerSpawned { temporary_id, id })
            } else {
                // We couldn't spawn the player so put them back where they were in the queue
                self.spawning_players.push_front((temporary_id, username));
                break;
            }
        }

        // Spawn as many as possible up to the entity count
        while self.mobs().len() < self.desired_mob_count {
            if !self.spawn_mob() {
                break;
            }
        }
        // Now that we've spawned all the players we can reset the waiting players and then
        // send the network tick message. (it's important that between the tick increment at the
        // start of this function no network messages we're able to be processed which is the case
        // because this is single threaded)
        self.waiting_players = self.data.players.len();

        callback(GameEvent::ProcessTick {
            game_data: self.data.clone(),
            tick: self.tick,
        })
    }

    pub fn tick_count(&self) -> u32 {
        self.tick
    }

    pub fn tick_start(&self) -> Instant {
        self.tick_start
    }

    pub fn players(&self) -> &PlayerBucket {
        &self.data.players
    }

    pub fn spawning_players(&self) -> &VecDeque<(usize, String)> {
        &self.spawning_players
    }

    pub fn mobs(&self) -> &MobBucket {
        &self.data.mobs
    }

    pub fn food(&self) -> &Grid<Option<Food>> {
        &self.data.food
    }

    /// The number of players that we're waiting for their action
    pub fn waiting_players(&self) -> usize {
        self.waiting_players
    }

    /// Adds the given username and temporary id to the list of spawning players.
    /// The system will try to spawn them in at the earliest convenience.
    /// Once they are spawned it will call the event handler with GameEvent::PlayerSpawned.
    pub fn add_client(&mut self, temporary_id: usize, username: String) {
        self.spawning_players.push_back((temporary_id, username));
    }

    pub fn player_action(&mut self, id: usize, action: Action, tick: u32) {
        if self.tick != tick {
            println!(
                "Player sent action for tick {} when it was tick {}",
                tick, self.tick
            );
            return;
        }

        if let Some(player) = self.data.players.get(id) {
            let next_action = &mut player.borrow_mut().next_action;

            if next_action.is_some() {
                println!("Player {} sent two actions in one tick", id);
            } else {
                // A way to check potential bugs
                assert_ne!(self.waiting_players, 0);
                // The player went from no action to an action:
                self.waiting_players -= 1;
            }

            *next_action = Some(action);
        } else {
            println!(
                "Ignored receieved action for player that didn't exist: {}",
                id
            );
        }
    }

    /// Removes the player from the game returning the player object
    /// Panics if the id isn't a player
    pub fn remove_client(&mut self, id: usize) -> Player {
        let player = self.data.players.remove(id).unwrap().into_inner();
        let (x, y) = player.position();
        self.data.entities[x as usize][y as usize] = None;
        player
    }

    pub fn data(&self) -> &GameData {
        &self.data
    }

    //    pub fn handle_network_messages(&mut self) {
    //        while let Ok(msg) = self.rx.try_recv() {
    //            match msg {
    //                NetworkMessage::ClientConnect {
    //                    temporary_id,
    //                    username,
    //                } => {
    //                    self.spawning_players.push((username, temporary_id));
    //                }
    //                NetworkMessage::PlayerAction { id, action, tick } => {
    //                    if self.tick != tick {
    //                        println!(
    //                            "Player sent action for tick {} when it was tick {}",
    //                            tick, self.tick
    //                        );
    //                        continue;
    //                    }
    //                    if let Some(player) = self.data.players.get(id) {
    //                        let next_action = &mut player.borrow_mut().next_action;
    //
    //                        if next_action.is_some() {
    //                            println!("Player {} sent two actions in one tick", id);
    //                        } else {
    //                            // A way to check potential bugs
    //                            assert_ne!(self.waiting_players, 0);
    //                            // The player went from no action to an action:
    //                            self.waiting_players -= 1;
    //                        }
    //
    //                        *next_action = Some(action);
    //                    } else {
    //                        println!(
    //                            "Ignored receieved action for player that didn't exist: {}",
    //                            id
    //                        );
    //                    }
    //                }
    //                NetworkMessage::ClientDisconnect { id } => {
    //                    let player = self.data.players.remove(id).unwrap();
    //                    let (x, y) = player.into_inner().position();
    //                    self.data.entities[x as usize][y as usize] = None;
    //                }
    //            }
    //        }
    //    }
}
