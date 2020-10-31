use ggez::{
    event::EventHandler,
    graphics,
    graphics::spritebatch::SpriteBatch,
    input::keyboard::{KeyCode, KeyMods},
    Context, GameResult,
};

use std::cell::RefCell;

mod map;
use map::{BaseTile, Map, SpawnLocation};

mod entity;
use entity::{Action, Direction, Entity, EntityIndex, EntityType, Mob, Player};

mod food;
use food::Food;

mod bucket;
use bucket::Bucket;

mod network;
use network::{GameMessage, NetworkManager, NetworkMessage};

mod animation;
use animation::Animation;

mod grid;
use grid::Grid;

use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

pub type MobBucket = Bucket<RefCell<Mob>>;
pub type PlayerBucket = Bucket<RefCell<Player>>;

struct GameData {
    pub map: Map,
    pub entities: Grid<Option<EntityIndex>>,
    pub players: PlayerBucket,
    pub mobs: MobBucket,
}

struct Game {
    data: GameData,
    player_sprite: SpriteBatch,
    mob_sprite: SpriteBatch,
    tick: u32,
    rx: Receiver<NetworkMessage>,
    tx: Sender<GameMessage>,
    spawning_players: Vec<(String, usize)>,
    tick_start: Instant,
}

fn load_sprite<P: AsRef<std::path::Path>>(ctx: &mut Context, path: P) -> SpriteBatch {
    let image = graphics::Image::new(ctx, path).unwrap();
    SpriteBatch::new(image)
}

impl Game {
    fn new(
        map: Map,
        ctx: &mut Context,
        rx: Receiver<NetworkMessage>,
        tx: Sender<GameMessage>,
    ) -> Game {
        Game {
            data: GameData {
                entities: Grid::fill_with_clone(None, map.width(), map.height()),
                map,
                players: Bucket::new(),
                mobs: Bucket::new(),
            },
            player_sprite: load_sprite(ctx, "/player.png"),
            mob_sprite: load_sprite(ctx, "/mob.png"),
            tick: 0,
            rx,
            tx,
            spawning_players: Vec::new(),
            tick_start: Instant::now(),
        }
    }

    fn add_entity(&mut self, (x, y): (u16, u16), index: EntityIndex) {
        self.data.entities[x as usize][y as usize] = Some(index);
    }

    fn spawn_player(&mut self, username: String) -> Option<usize> {
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

    fn spawn_mob(&mut self) -> bool {
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

    fn build_terrain_mesh(&self, square_width: f32) -> graphics::MeshBuilder {
        let mut builder = graphics::MeshBuilder::new();

        for x in 0..(self.data.map.width as usize) {
            for y in 0..(self.data.map.height as usize) {
                let tex = self.data.map.base_tile[x + y * self.data.map.width as usize].texture();
                let rect = graphics::Rect::new(
                    x as f32 * square_width,
                    y as f32 * square_width,
                    square_width,
                    square_width,
                );
                builder.rectangle(graphics::DrawMode::Fill(Default::default()), rect, tex);
            }
        }

        builder
    }

    fn setup(&mut self, ctx: &mut Context) -> GameResult {
        graphics::set_resizable(ctx, true)?;
        graphics::set_window_title(ctx, "Ai game");

        Ok(())
    }

    fn base_tile(&self, x: usize, y: usize) -> &BaseTile {
        &self.data.map.base_tile[x + y * self.data.map.width as usize]
    }

    fn entity(&self, x: usize, y: usize) -> &Option<EntityIndex> {
        &self.data.entities[x][y]
    }

    /// Performs a single tick applying all the stored actions for each player.
    ///
    /// Simulation order:
    /// 1. Mobs/Players (top left to bottom right row by row)
    /// 2. Spawn new players (if there is space)
    fn tick(&mut self) {
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

    fn handle_network_messages(&mut self) {
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

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.handle_network_messages();

        if ggez::timer::check_update_time(ctx, 1) {
            self.tick();
        } else {
            std::thread::sleep(std::time::Duration::from_millis(15));
        }

        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) {
        graphics::set_screen_coordinates(
            ctx,
            graphics::Rect::new(-width / 2.0, -height / 2.0, width, height),
        )
        .unwrap();
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, [0.5, 0.5, 0.5, 1.0].into());
        let cur_time = self.tick_start.elapsed().as_secs_f32();
        let square_width = 15.0;
        let mesh = self.build_terrain_mesh(square_width).build(ctx)?;

        for (_index, player) in self.data.players.iter() {
            let (x, y) = player.borrow_mut().position_animated(cur_time);
            let param = graphics::DrawParam::new()
                .offset([0.5, 0.5])
                .dest([
                    (x as f32 + 0.5) * square_width,
                    (y as f32 + 0.5) * square_width,
                ])
                .scale([square_width / 170.0, square_width / 170.0])
                .rotation(player.borrow().direction().to_rad());

            self.player_sprite.add(param);
        }

        for (_index, mob) in self.data.mobs.iter() {
            let (x, y) = mob.borrow_mut().position_animated(cur_time);
            let param = graphics::DrawParam::new()
                .offset([0.5, 0.5])
                .dest([
                    (x as f32 + 0.5) * square_width,
                    (y as f32 + 0.5) * square_width,
                ])
                .scale([square_width / 170.0, square_width / 170.0])
                .rotation(mob.borrow().direction().to_rad());

            self.mob_sprite.add(param);
        }

        let scale = 1.5;

        let draw_params = graphics::DrawParam::new()
            .dest([
                -(self.data.map.width as f32) * scale * square_width / 2.0,
                -(self.data.map.height as f32) * scale * square_width / 2.0,
            ])
            //.offset([0.5, 0.5])
            .scale([scale; 2]);
        graphics::draw(ctx, &mesh, draw_params.clone())?;
        graphics::draw(ctx, &self.player_sprite, draw_params.clone())?;
        graphics::draw(ctx, &self.mob_sprite, draw_params.clone())?;

        self.player_sprite.clear();
        self.mob_sprite.clear();

        graphics::present(ctx)
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
        match keycode {
            KeyCode::M => {
                if !self.spawn_mob() {
                    println!("Couldn't spawn mob");
                } else {
                    println!("Spawned mob, there are now: {} mobs", self.data.mobs.len());
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let map = Map::new(16, 16);

    let mut cb = ggez::ContextBuilder::new("Ai Game", "Louis de Wardt")
        .window_mode(ggez::conf::WindowMode::default().resizable(true))
        .window_setup(ggez::conf::WindowSetup::default().vsync(true));

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = std::path::PathBuf::from(manifest_dir);
        path.push("resources");
        cb = cb.add_resource_path(path);
    } else {
        cb = cb.add_resource_path("./resources");
    }

    let (mut ctx, mut event_loop) = cb.build().unwrap();

    let (tx, rx) = NetworkManager::start("localhost:2010").expect("Couldn't start network manager");
    println!("Listening on localhost:2010");

    let mut game = Game::new(map, &mut ctx, rx, tx);
    game.setup(&mut ctx).unwrap();
    ggez::event::run(&mut ctx, &mut event_loop, &mut game).unwrap();
}
