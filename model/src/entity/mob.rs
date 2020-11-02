use crate::{
    Animation, BaseTile, Direction, Entity, EntityIndex, EntityType, Food, Grid, Map, MobBucket,
    PlayerBucket,
};

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

    fn direction_animated(&mut self, _cur_time: f32) -> f32 {
        todo!();
    }

    fn turn(&mut self, direction: Direction, _animated: bool) {
        // TODO: Setup animation
        self.direction = direction;
    }

    fn deal_damage(&mut self, damage: u8) {
        if damage > 0 {
            self.is_dead = true;
        }
    }

    fn process_turn(
        &mut self,
        entities: &mut Grid<Option<EntityIndex>>,
        _food: &mut Grid<Option<Food>>,
        _mobs: &MobBucket,
        players: &PlayerBucket,
        map: &Map,
    ) {
        let (cur_x, cur_y) = self.position();
        if let Some((new_x, new_y)) = map.calc_foward(cur_x, cur_y, &self.direction) {
            // We can't move to that tile
            if map.base_tile(new_x as usize, new_y as usize) != &BaseTile::Land {
                self.turn(self.direction.clockwise(), true);
                return;
            }

            if let Some(entity_index) = &entities[new_x as usize][new_y as usize] {
                match entity_index.entity_type() {
                    EntityType::Mob => {
                        self.turn(self.direction.clockwise(), true);
                        return;
                    }
                    EntityType::Player => {
                        let mut enemy = players.get(entity_index.index()).unwrap().borrow_mut();
                        if enemy.is_invulnerable() {
                            // Do nothing if we can't attack them
                            return;
                        }

                        enemy.deal_damage(1);

                        // Only if the entity died do we continue and do the normal move forward
                        // otherwise we do nothing for this turn (except for the attack that didn't
                        // kill)
                        if !enemy.died() {
                            return;
                        }

                        // Remove the dead enemy from the board
                        entities[new_x as usize][new_y as usize] = None;
                    }
                }
            }

            // The next tile is land and there is now no entity on it:
            self.set_pos((new_x, new_y), true);
            // The new_x, new_y should be empty so it should put None into the entity's original
            // position
            entities.swap(
                (cur_x as usize, cur_y as usize),
                (new_x as usize, new_y as usize),
            );
        } else {
            self.turn(self.direction.clockwise(), true);
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
