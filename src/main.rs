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

mod bucket;
use bucket::Bucket;

mod network;
use network::{GameMessage, NetworkManager, NetworkMessage};

mod animation;
use animation::Animation;

use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

struct Game {
    map: Map,
    entities: Vec<Option<EntityIndex>>,
    players: Bucket<RefCell<Player>>,
    mobs: Bucket<RefCell<Mob>>,
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
            entities: vec![None; map.base_tile.len()],
            map,
            players: Bucket::new(),
            mobs: Bucket::new(),
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
        self.entities[x as usize + y as usize * self.map.width as usize] = Some(index);
    }

    fn spawn_player(&mut self, username: String) -> Option<usize> {
        let (x, y) = match self.spawn_location(&self.map.player_spawn) {
            Some(location) => location,
            None => return None,
        };

        let id = self.players.add(RefCell::new(Player::new(
            (x, y),
            Direction::North,
            3,
            2,
            0,
            username,
            None,
        )));

        self.add_entity((x, y), EntityIndex::new_player(id));

        Some(id)
    }

    fn spawn_mob(&mut self) -> bool {
        let (x, y) = match self.spawn_location(&self.map.mob_spawn) {
            Some(location) => location,
            None => return false,
        };

        let id = self
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
                for x in 0..self.map.width {
                    for y in 0..self.map.height {
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

        for x in 0..(self.map.width as usize) {
            for y in 0..(self.map.height as usize) {
                let tex = self.map.base_tile[x + y * self.map.width as usize].texture();
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
        &self.map.base_tile[x + y * self.map.width as usize]
    }

    fn entity(&self, x: usize, y: usize) -> &Option<EntityIndex> {
        &self.entities[self.map.flatten_coordinate(x, y)]
    }

    fn entity_from_index(&self, entity_index: &EntityIndex) -> &RefCell<dyn Entity> {
        match entity_index.entity_type() {
            EntityType::Player => self
                .players
                .get(entity_index.index)
                .expect("Player didn't exist"),
            EntityType::Mob => self.mobs.get(entity_index.index).expect("Mob didn't exist"),
        }
    }

    fn move_entity(
        entities: &mut [Option<EntityIndex>],
        map: &Map,
        origin: (u16, u16),
        destination: (u16, u16),
    ) {
        let flattened_origin = map.flatten_coordinate(origin.0 as usize, origin.1 as usize);
        let flattened_destination =
            map.flatten_coordinate(destination.0 as usize, destination.1 as usize);

        entities.swap(flattened_origin, flattened_destination);
    }

    fn attack(&mut self, attacker: (u16, u16), mut victim: (u16, u16), direction: Direction) {
        let old_victim = victim;
        let blocked = if let Some(bump_pos) = self.map.calc_foward(victim.0, victim.1, &direction) {
            if self
                .entity(bump_pos.0 as usize, bump_pos.1 as usize)
                .is_some()
            {
                true
            } else {
                use BaseTile::*;
                match self.base_tile(bump_pos.0 as usize, bump_pos.1 as usize) {
                    Land => {
                        victim = bump_pos;
                        false
                    }
                    Water => {
                        let victim_index = self
                            .entity(victim.0 as usize, victim.1 as usize)
                            .as_ref()
                            .expect("victim didn't exist");

                        self.entity_from_index(victim_index).borrow_mut().kill();
                        victim = bump_pos;
                        false
                    }
                    Wall => true,
                }
            }
        } else {
            true
        };

        let attacker_index = self
            .entity(attacker.0 as usize, attacker.1 as usize)
            .as_ref()
            .expect("attacker didn't exist");

        let victim_index = self
            .entity(old_victim.0 as usize, old_victim.1 as usize)
            .as_ref()
            .expect("victim didn't exist");

        let mut victim_entity = self.entity_from_index(victim_index).borrow_mut();
        let mut attacker_entity = self.entity_from_index(attacker_index).borrow_mut();

        if blocked {
            if !victim_entity.is_invulnerable() {
                victim_entity.deal_damage(2);
            }
        } else {
            victim_entity.set_pos(victim, true);
            if !victim_entity.is_invulnerable() {
                victim_entity.deal_damage(1);
            }
            attacker_entity.set_pos(old_victim, true);
            drop(victim_entity);
            drop(attacker_entity);

            Self::move_entity(&mut self.entities, &self.map, old_victim, victim);
            Self::move_entity(&mut self.entities, &self.map, attacker, old_victim);
        }
    }

    /// Performs a single tick applying all the stored actions for each player.
    ///
    /// Simulation order:
    /// 1. Mobs
    /// 2. Players
    ///     1. North facing
    ///     2. East facing
    ///     3. South facing
    ///     4. West facing
    fn tick(&mut self) {
        self.tick += 1;
        self.tick_start = Instant::now();

        for i in 0..self.mobs.max_id() {
            if let Some(mob) = self.mobs.get(i) {
                let old_pos = mob.borrow().position();
                let direction = mob.borrow().direction();

                if mob.borrow().died() {
                    self.entities[self
                        .map
                        .flatten_coordinate(old_pos.0 as usize, old_pos.1 as usize)] = None;
                    self.mobs.remove(i);
                    continue;
                }

                // Compute mob strategy (should be more advanced than just go foward):
                if let Some((new_x, new_y)) = self.map.calc_foward(old_pos.0, old_pos.1, &direction)
                {
                    if self.map.base_tile(new_x as usize, new_y as usize) == &BaseTile::Land {
                        if let Some(entity_index) = &self.entities
                            [self.map.flatten_coordinate(new_x as usize, new_y as usize)]
                        {
                            if entity_index.entity_type().is_mob() {
                                mob.borrow_mut().turn(direction.clockwise(), true);
                            } else {
                                drop(mob);
                                self.attack(old_pos, (new_x, new_y), direction);
                            }
                        } else {
                            Self::move_entity(
                                &mut self.entities,
                                &self.map,
                                old_pos,
                                (new_x, new_y),
                            );
                            mob.borrow_mut().set_pos((new_x, new_y), true);
                        }
                    } else {
                        mob.borrow_mut().turn(direction.clockwise(), true);
                    }
                } else {
                    mob.borrow_mut().turn(direction.clockwise(), true);
                }
            }
        }

        let mut north_movements = Vec::new();
        let mut east_movements = Vec::new();
        let mut south_movements = Vec::new();
        let mut west_movements = Vec::new();

        for i in 0..self.players.max_id() {
            if let Some(player) = self.players.get(i) {
                player.borrow_mut().tick();
                let old_pos = player.borrow().position();
                let direction = player.borrow().direction();

                if player.borrow().died() {
                    self.entities[self
                        .map
                        .flatten_coordinate(old_pos.0 as usize, old_pos.1 as usize)] = None;
                    let player = self.players.remove(i).unwrap().into_inner();
                    self.tx
                        .send(GameMessage::PlayerDied {
                            player_id: i,
                            final_score: player.score(),
                        })
                        .unwrap();
                    continue;
                }

                let action = match player.borrow_mut().handle_action() {
                    Some(action) => action,
                    None => {
                        println!("Player {} didn't specify an action", i);
                        Action::Stay
                    }
                };

                println!("Player {} chose action {:?}", i, action);

                match action {
                    Action::Stay => {}
                    Action::TurnLeft => player.borrow_mut().turn(direction.anti_clockwise(), true),
                    Action::TurnRight => player.borrow_mut().turn(direction.clockwise(), true),
                    Action::Eat => unimplemented!(),
                    Action::Forward => {
                        use Direction::*;
                        match direction {
                            North => north_movements.push(i),
                            East => east_movements.push(i),
                            South => south_movements.push(i),
                            West => west_movements.push(i),
                        }
                    }
                }
            }
        }

        for group in &[
            north_movements,
            east_movements,
            south_movements,
            west_movements,
        ] {
            for i in group {
                let player = self.players.get(*i).expect("Player didn't exist");
                let old_pos = player.borrow().position();
                let direction = player.borrow().direction();

                // sanity check
                assert!(self.entities[self
                    .map
                    .flatten_coordinate(old_pos.0 as usize, old_pos.1 as usize)]
                .as_ref()
                .unwrap()
                .entity_type()
                .is_player());

                if let Some((new_x, new_y)) = self.map.calc_foward(old_pos.0, old_pos.1, &direction)
                {
                    if self.map.base_tile(new_x as usize, new_y as usize) == &BaseTile::Land {
                        if self.entity(new_x as usize, new_y as usize).is_some() {
                            drop(player);
                            self.attack(old_pos, (new_x, new_y), direction);
                        } else {
                            Self::move_entity(
                                &mut self.entities,
                                &self.map,
                                old_pos,
                                (new_x, new_y),
                            );
                            player.borrow_mut().set_pos((new_x, new_y), true);
                        }
                    }
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
                map: self.map.clone(),
                entities: self.entities.clone(),
                players: self.players.clone(),
                mobs: self.mobs.clone(),
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
                    if let Some(player) = self.players.get(id) {
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
                    let player = self.players.remove(id).unwrap();
                    let (x, y) = player.into_inner().position();
                    self.entities[self.map.flatten_coordinate(x as usize, y as usize)] = None;
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

        for (_index, player) in self.players.iter() {
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

        for (_index, mob) in self.mobs.iter() {
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
                -(self.map.width as f32) * scale * square_width / 2.0,
                -(self.map.height as f32) * scale * square_width / 2.0,
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
                    println!("Spawned mob, there are now: {} mobs", self.mobs.len());
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let map = Map::new(16, 16);

    let mut cb = ggez::ContextBuilder::new("Ai Game", "author");

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
