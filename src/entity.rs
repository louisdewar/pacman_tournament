use crate::Animation;
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug)]
pub struct Player {
    pos: (u16, u16),
    pos_animation: (Animation, Animation),
    direction: Direction,
    direction_animation: Animation,
    health: u8,
    invulnerable_turns: u8,
    apple_count: u8,
    score: usize,
    pub username: String,
    pub next_action: Option<Action>,
}

impl Player {
    pub fn new(
        pos: (u16, u16),
        direction: Direction,
        health: u8,
        invulnerable_turns: u8,
        apple_count: u8,
        username: String,
        next_action: Option<Action>,
    ) -> Self {
        Player {
            pos,
            direction,
            health,
            invulnerable_turns,
            apple_count,
            score: 0,
            username,
            next_action,
            pos_animation: Default::default(),
            direction_animation: Default::default(),
        }
    }

    pub fn tick(&mut self) {
        self.invulnerable_turns = self.invulnerable_turns.saturating_sub(1);
        self.score += 1;
    }

    /// Returns the action setting it back to None.
    pub fn handle_action(&mut self) -> Option<Action> {
        let mut action = None;
        std::mem::swap(&mut action, &mut self.next_action);
        action
    }

    pub fn score(&self) -> usize {
        self.score
    }
}

#[derive(Clone, Debug)]
pub struct Mob {
    pos: (u16, u16),
    pos_animation: (Animation, Animation),
    direction: Direction,
    direction_animation: Animation,
    is_dead: bool,
}

impl Mob {
    pub fn new(pos: (u16, u16), direction: Direction, is_dead: bool) -> Self {
        Mob {
            pos,
            direction,
            is_dead,
            pos_animation: Default::default(),
            direction_animation: Default::default(),
        }
    }
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

    fn is_invulnerable(&self) -> bool {
        false
    }

    fn died(&self) -> bool {
        self.health() == 0
    }
}

impl Entity for Player {
    fn position(&self) -> (u16, u16) {
        self.pos
    }

    fn position_animated(&mut self, cur_time: f32) -> (f32, f32) {
        (
            self.pos.0 as f32 + self.pos_animation.0.current_delta(cur_time),
            self.pos.1 as f32 + self.pos_animation.1.current_delta(cur_time),
        )
    }

    fn set_pos(&mut self, new_pos: (u16, u16), animated: bool) {
        if animated {
            let new_x = new_pos.0 as f32;
            let new_y = new_pos.1 as f32;

            let old_x = self.pos.0 as f32;
            let old_y = self.pos.1 as f32;

            let delta = (old_x - new_x, old_y - new_y);
            self.pos_animation = (
                Animation::new(0.4, 0.4, delta.0, 0.0),
                Animation::new(0.4, 0.4, delta.1, 0.0),
            );
        }
        self.pos = new_pos;
    }

    fn direction(&self) -> Direction {
        self.direction
    }

    fn direction_animated(&mut self, cur_time: f32) -> f32 {
        todo!();
    }

    fn turn(&mut self, direction: Direction, animated: bool) {
        // TODO: Setup animation
        self.direction = direction;
    }

    fn deal_damage(&mut self, damage: u8) {
        self.health = self.health.saturating_sub(damage);
    }

    fn kill(&mut self) {
        self.health = 0;
    }

    fn health(&self) -> u8 {
        self.health
    }

    fn is_invulnerable(&self) -> bool {
        self.invulnerable_turns > 0
    }
}

impl Entity for Mob {
    fn position(&self) -> (u16, u16) {
        self.pos
    }

    fn position_animated(&mut self, cur_time: f32) -> (f32, f32) {
        (
            self.pos.0 as f32 + self.pos_animation.0.current_delta(cur_time),
            self.pos.1 as f32 + self.pos_animation.1.current_delta(cur_time),
        )
    }

    fn set_pos(&mut self, new_pos: (u16, u16), animated: bool) {
        if animated {
            let new_x = new_pos.0 as f32;
            let new_y = new_pos.1 as f32;

            let old_x = self.pos.0 as f32;
            let old_y = self.pos.1 as f32;

            let delta = (old_x - new_x, old_y - new_y);
            self.pos_animation = (
                Animation::new(0.4, 0.4, delta.0, 0.0),
                Animation::new(0.4, 0.4, delta.1, 0.0),
            );
        }

        self.pos = new_pos;
    }

    fn direction(&self) -> Direction {
        self.direction
    }

    fn direction_animated(&mut self, cur_time: f32) -> f32 {
        todo!();
    }

    fn turn(&mut self, direction: Direction, animated: bool) {
        // TODO: Setup animation
        self.direction = direction;
    }

    fn deal_damage(&mut self, damage: u8) {
        if damage > 0 {
            self.is_dead = true;
        }
    }

    fn kill(&mut self) {
        self.is_dead = true;
    }

    fn health(&self) -> u8 {
        if self.is_dead {
            0
        } else {
            1
        }
    }
}

#[derive(Clone, Debug)]
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
    pub entity_type: EntityType,
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

    pub fn entity_type(&self) -> &EntityType {
        &self.entity_type
    }
}
