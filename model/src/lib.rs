use std::cell::RefCell;

mod map;
pub use map::{BaseTile, Map, SpawnLocation};

mod entity;
pub use entity::{Action, Direction, Entity, EntityIndex, EntityType, Mob, Player};

mod food;
pub use food::Food;

mod bucket;
pub use bucket::Bucket;

mod network;
pub use network::{GameMessage, NetworkManager, NetworkMessage};

mod animation;
pub use animation::Animation;

mod grid;
pub use grid::Grid;

pub use std::sync::mpsc::{Receiver, Sender};
pub use std::time::Instant;

pub type MobBucket = Bucket<RefCell<Mob>>;
pub type PlayerBucket = Bucket<RefCell<Player>>;

struct GameData {
    pub map: Map,
    pub entities: Grid<Option<EntityIndex>>,
    pub players: PlayerBucket,
    pub mobs: MobBucket,
}

pub struct Model {
    data: GameData,
    rx: Receiver<NetworkMessage>,
    tx: Sender<GameMessage>,
    spawning_players: Vec<(String, usize)>,
    tick: u32,
    tick_start: Instant,
}

impl Model {
    pub fn new(map: Map, rx: Receiver<NetworkMessage>, tx: Sender<GameMessage>) -> Model {
        Model {
            data: GameData {
                entities: Grid::fill_with_clone(None, map.width(), map.height()),
                map,
                players: Bucket::new(),
                mobs: Bucket::new(),
            },
            tick: 0,
            rx,
            tx,
            spawning_players: Vec::new(),
            tick_start: Instant::now(),
        }
    }

    pub fn add_entity(&mut self, (x, y): (u16, u16), index: EntityIndex) {
        self.data.entities[x as usize][y as usize] = Some(index);
    }

    pub fn spawn_player(&mut self, username: String) -> Option<usize> {
        let (x, y) = match self.spawn_location(&self.data.map.player_spawn) {
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
        let (x, y) = match self.spawn_location(&self.data.map.mob_spawn) {
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
                for x in 0..self.data.map.width {
                    for y in 0..self.data.map.height {
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

    // TODO: maybe delete base_tile + entity
    fn base_tile(&self, x: usize, y: usize) -> &BaseTile {
        &self.data.map.base_tile[x + y * self.data.map.width as usize]
    }

    fn entity(&self, x: usize, y: usize) -> &Option<EntityIndex> {
        &self.data.entities[x][y]
    }

    /// Performs a single tick applying all the stored actions for each player.
    ///
    /// Simulation order:
    /// 1. Mobs/Players (top left to bottom right row by row) - when entites die they are removed
    ///    from the grid immediately but their object still exists in the bucket (it is tidied
    ///    up in the next step)
    /// 2. Send messages about dead players (also clean up dead mobs)
    /// 3. Spawn new players (if there is space)
    pub fn simulate_tick(&mut self) {
        self.tick += 1;
        self.tick_start = Instant::now();

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
        let mobs = &self.data.mobs;
        let players = &self.data.players;
        let map = &self.data.map;

        for entity_index in entity_queue {
            match entity_index.entity_type() {
                EntityType::Mob => mobs
                    .get(entity_index.index())
                    .map(|m| m.borrow_mut().process_turn(entities, mobs, players, map)),
                EntityType::Player => players
                    .get(entity_index.index())
                    .map(|p| p.borrow_mut().process_turn(entities, mobs, players, map)),
            };
        }

        for player_id in 0..self.data.players.max_id() {
            if let Some(player) = self.data.players.get(player_id) {
                if player.borrow().died() {
                    let player = self.data.players.remove(player_id).unwrap();
                    self.tx
                        .send(GameMessage::PlayerDied {
                            player_id,
                            final_score: player.borrow().score(),
                        })
                        .unwrap();
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

        let mut current_spawning_players = Vec::new();

        std::mem::swap(&mut current_spawning_players, &mut self.spawning_players);

        for (username, temporary_id) in current_spawning_players {
            if let Some(id) = self.spawn_player(username.clone()) {
                self.tx
                    .send(GameMessage::PlayerSpawned { temporary_id, id })
                    .unwrap();
            } else {
                self.spawning_players.push((username, temporary_id));
            }
        }

        self.network_process_tick();
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

    pub fn mobs(&self) -> &MobBucket {
        &self.data.mobs
    }

    fn network_process_tick(&mut self) {
        self.tx
            .send(GameMessage::ProcessTick {
                map: self.data.map.clone(),
                entities: self.data.entities.clone(),
                players: self.data.players.clone(),
                mobs: self.data.mobs.clone(),
                tick: self.tick,
            })
            .expect("Couldn't communicate with network manager");
    }

    pub fn handle_network_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                NetworkMessage::ClientConnect {
                    temporary_id,
                    username,
                } => {
                    self.spawning_players.push((username, temporary_id));
                }
                NetworkMessage::PlayerAction { id, action, tick } => {
                    if self.tick != tick {
                        println!(
                            "Player sent action for tick {} when it was tick {}",
                            tick, self.tick
                        );
                        continue;
                    }
                    if let Some(player) = self.data.players.get(id) {
                        let next_action = &mut player.borrow_mut().next_action;

                        if next_action.is_some() {
                            println!("Player {} sent two actions in one tick", id);
                        }

                        *next_action = Some(action);
                    } else {
                        println!(
                            "Ignored receieved action for player that didn't exist: {}",
                            id
                        );
                    }
                }
                NetworkMessage::ClientDisconnect { id } => {
                    let player = self.data.players.remove(id).unwrap();
                    let (x, y) = player.into_inner().position();
                    self.data.entities[x as usize][y as usize] = None;
                }
            }
        }
    }
}