use crate::{
    Action, Animation, BaseTile, Direction, Entity, EntityIndex, Food, Grid, Map, MobBucket,
    PlayerBucket,
};

#[derive(Clone, Debug)]
pub struct Player {
    pos: (u16, u16),
    pos_animation: (Animation, Animation),
    direction: Direction,
    direction_animation: Animation,
    health: u8,
    invulnerable_turns: u8,
    has_powerpill: bool,
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
        username: String,
        next_action: Option<Action>,
    ) -> Self {
        Player {
            pos,
            direction,
            health,
            invulnerable_turns,
            has_powerpill: false,
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

    pub fn has_powerpill(&self) -> bool {
        self.has_powerpill
    }

    pub fn eat(&mut self, food: Food) {
        match food {
            Food::Fruit => {
                // 10 points per fruit
                self.score += 10;
            }
            Food::PowerPill => {
                self.score += 50;
                self.has_powerpill = true;
            }
        }
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

    fn process_turn(
        &mut self,
        entities: &mut Grid<Option<EntityIndex>>,
        food: &mut Grid<Option<Food>>,
        mobs: &MobBucket,
        players: &PlayerBucket,
        map: &Map,
    ) {
        // TODO: decide whether to tick before process or at the end
        self.tick();

        let mut action = None;

        std::mem::swap(&mut action, &mut self.next_action);

        // The default action if none is provided is stay
        let action = action.unwrap_or(Action::Stay);

        let (cur_x, cur_y) = self.position();
        let direction = self.direction();

        match action {
            Action::Stay => return,
            Action::Forward => {
                if let Some((new_x, new_y)) = map.calc_foward(cur_x, cur_y, &direction) {
                    if map.base_tile(new_x as usize, new_y as usize) != &BaseTile::Land {
                        // You can only move onto land
                        return;
                    }

                    if let Some(entity_index) = &entities[new_x as usize][new_y as usize] {
                        let mut enemy = entity_index.as_entity(mobs, players).borrow_mut();
                        if !enemy.is_invulnerable() {
                            enemy.deal_damage(1);

                            // If they don't die stay
                            if !enemy.died() {
                                return;
                            }

                            // Remove the dead enemy from the board
                            entities[new_x as usize][new_y as usize] = None;
                        } else {
                            // We can't attack them so we must stay
                            return;
                        }
                    }

                    if let Some(food_item) = food[new_x as usize][new_y as usize].clone() {
                        self.eat(food_item);
                        food[new_x as usize][new_y as usize] = None;
                    }

                    // Apply the movement:
                    entities.swap(
                        (cur_x as usize, cur_y as usize),
                        (new_x as usize, new_y as usize),
                    );
                    self.set_pos((new_x, new_y), true);
                } else {
                    println!("Player {} tried to move forward off map", &self.username);
                }
            }
            Action::TurnRight => self.turn(self.direction().clockwise(), true),
            Action::TurnLeft => self.turn(self.direction().anti_clockwise(), true),
            Action::Eat => todo!("Maybe remove this action / change to eat power pill"),
        }
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
