use serde::{Deserialize, Serialize};

use crate::{Grid, Map, MobBucket, PlayerBucket};

mod player;
pub use player::Player;

mod mob;
pub use mob::Mob;

#[derive(Clone, Debug, Copy, Serialize)]
pub enum Direction {
    #[serde(rename(serialize = "N"))]
    North,
    #[serde(rename(serialize = "E"))]
    East,
    #[serde(rename(serialize = "S"))]
    South,
    #[serde(rename(serialize = "W"))]
    West,
}

impl Direction {
    pub fn clockwise(self) -> Self {
        use Direction::*;

        match self {
            North => East,
            East => South,
            South => West,
            West => North,
        }
    }

    pub fn anti_clockwise(self) -> Self {
        use Direction::*;

        match self {
            North => West,
            East => North,
            South => East,
            West => South,
        }
    }

    pub fn to_rad(&self) -> f32 {
        use Direction::*;
        let pi = std::f32::consts::PI;
        let pi_2 = std::f32::consts::FRAC_PI_2;

        match &self {
            North => pi + pi_2,
            East => 0.0,
            South => pi_2,
            West => pi,
        }
    }

    pub fn to_num(&self) -> u8 {
        use Direction::*;

        match &self {
            North => 0,
            East => 1,
            South => 2,
            West => 3,
        }
    }

    pub fn from_num(num: u8) -> Self {
        use Direction::*;

        match num {
            0 => North,
            1 => East,
            2 => South,
            3 => West,
            _ => panic!("Invalid num"),
        }
    }
}

impl std::ops::Add<Direction> for Direction {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::from_num((self.to_num() + other.to_num()) % 3)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum Action {
    #[serde(rename(deserialize = "F"))]
    Forward,
    #[serde(rename(deserialize = "S"))]
    Stay,
    #[serde(rename(deserialize = "L"))]
    TurnLeft,
    #[serde(rename(deserialize = "R"))]
    TurnRight,
    #[serde(rename(deserialize = "E"))]
    Eat,
}

pub trait Entity {
    fn position(&self) -> (u16, u16);
    fn set_pos(&mut self, new_pos: (u16, u16), animated: bool);
    fn position_animated(&mut self, cur_time: f32) -> (f32, f32);

    fn direction(&self) -> Direction;
    fn turn(&mut self, direction: Direction, animated: bool);
    fn direction_animated(&mut self, cur_time: f32) -> f32;

    fn deal_damage(&mut self, damage: u8);
    fn kill(&mut self);
    fn health(&self) -> u8;

    fn process_turn(
        &mut self,
        entities: &mut Grid<Option<EntityIndex>>,
        mobs: &MobBucket,
        players: &PlayerBucket,
        map: &Map,
    );

    fn is_invulnerable(&self) -> bool {
        false
    }

    fn died(&self) -> bool {
        self.health() == 0
    }
}

#[derive(Clone, Debug, Copy)]
pub enum EntityType {
    Player,
    Mob,
}

impl EntityType {
    pub fn is_mob(&self) -> bool {
        match &self {
            EntityType::Mob => true,
            _ => false,
        }
    }

    pub fn is_player(&self) -> bool {
        match &self {
            EntityType::Player => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EntityIndex {
    entity_type: EntityType,
    pub index: usize,
}

impl EntityIndex {
    pub fn new_player(index: usize) -> Self {
        EntityIndex {
            index,
            entity_type: EntityType::Player,
        }
    }

    pub fn new_mob(index: usize) -> Self {
        EntityIndex {
            index,
            entity_type: EntityType::Mob,
        }
    }

    pub fn entity_type(&self) -> EntityType {
        self.entity_type
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn as_entity_mut<'a>(
        &self,
        mobs: &'a MobBucket,
        players: &'a PlayerBucket,
    ) -> Option<std::cell::RefMut<'a, dyn Entity>> {
        match self.entity_type {
            EntityType::Mob => mobs.get(self.index).map(|e| {
                // This short cast is unforunately required for some reason I'm not too sure about,
                // I think it has something to do with the map function.
                let x: std::cell::RefMut<'_, dyn Entity> = e.borrow_mut();
                x
            }),
            EntityType::Player => players.get(self.index).map(|e| {
                let x: std::cell::RefMut<'_, dyn Entity> = e.borrow_mut();
                x
            }),
        }
    }
}
