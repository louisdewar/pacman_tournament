use crate::{
    Action, Animation, BaseTile, Direction, Entity, EntityIndex, EntityType, Food, Grid, Map,
    MobBucket, PlayerBucket,
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
    score: u32,
    username: String,
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
            next_action,
            pos_animation: Default::default(),
            direction_animation: Default::default(),
            username,
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

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn username(&self) -> &String {
        &self.username
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
                self.score += 100;
                self.has_powerpill = true;
            }
        }
    }

    /// Simulates the player moving from their current location in the provided direction and
    /// attacking the entity if there is on there.
    /// It will not allow the player to move forward if it is blocked by terrain (i.e. a wall)
    /// It will move the player if the attack is successful / there was no entity (both with
    /// self.pos and with the entity grid).
    /// If the player attacks a mob and the player is not using a power pill they will die,
    /// otherwise the mob will die.
    /// This will handle killing the enemy player if there is one but it will check to make sure
    /// that the player isn't invulnerable.
    /// It will also simulate eating any food that's on the square
    fn attack_square(
        &mut self,
        direction: Direction,
        using_powerpill: bool,
        map: &Map,
        entities: &mut Grid<Option<EntityIndex>>,
        food: &mut Grid<Option<Food>>,
        mobs: &MobBucket,
        players: &PlayerBucket,
    ) {
        let src = self.position();
        if let Some(dest) = map.calc_foward(src.0, src.1, &direction) {
            if map.base_tile(dest.0 as usize, dest.1 as usize) != &BaseTile::Land {
                // You can only move onto land
                return;
            }

            if let Some(entity_index) = &entities[dest.0 as usize][dest.1 as usize] {
                match entity_index.entity_type() {
                    EntityType::Player => {
                        let mut enemy = players.get(entity_index.index()).unwrap().borrow_mut();

                        // We're both looking at each other so we both die
                        if enemy.direction().reverse() == direction {
                            enemy.kill();
                            self.kill();

                            entities[src.0 as usize][src.1 as usize] = None;
                            entities[dest.0 as usize][dest.1 as usize] = None;
                            return;
                        }

                        if !enemy.is_invulnerable() {
                            enemy.deal_damage(1);

                            // If they don't die stay
                            if !enemy.died() {
                                return;
                            }

                            // Remove the dead enemy from the board
                            entities[dest.0 as usize][dest.1 as usize] = None;
                            // Get some points for killing an enemy
                            self.score += 150;
                        } else {
                            // We can't attack them so we must stay
                            return;
                        }
                    }
                    EntityType::Mob => {
                        if using_powerpill {
                            let mut enemy = mobs.get(entity_index.index()).unwrap().borrow_mut();
                            enemy.kill();
                            entities[dest.0 as usize][dest.1 as usize] = None;
                            self.score += 150;
                            return;
                        } else {
                            self.kill();
                            entities[src.0 as usize][src.1 as usize] = None;
                            return;
                        }
                    }
                }
            }

            if let Some(food_item) = food[dest.0 as usize][dest.1 as usize].clone() {
                self.eat(food_item);
                food[dest.0 as usize][dest.1 as usize] = None;
            }

            debug_assert!(
                entities[dest.0 as usize][dest.1 as usize].is_none(),
                "Tile we're moving into wasn't None"
            );

            debug_assert_ne!(
                (dest.0, dest.1),
                (src.0, src.1),
                "Old and new positions were the same"
            );

            // Apply the movement:
            entities.swap(
                (dest.0 as usize, dest.1 as usize),
                (src.0 as usize, src.1 as usize),
            );
            self.set_pos((dest.0, dest.1), true);
        } else {
            println!("Player tried to move forward off map");
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
        let direction = self.direction();

        match action {
            Action::Stay => return,
            Action::Forward => {
                self.attack_square(direction, false, map, entities, food, mobs, players);
            }
            Action::TurnRight => self.turn(self.direction().clockwise(), true),
            Action::TurnLeft => self.turn(self.direction().anti_clockwise(), true),
            Action::Eat => {
                if self.has_powerpill {
                    self.has_powerpill = false;
                    for _ in 0..2 {
                        self.attack_square(direction, true, map, entities, food, mobs, players);
                    }
                }
            }
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

    fn entity_type(&self) -> EntityType {
        EntityType::Player
    }
}
