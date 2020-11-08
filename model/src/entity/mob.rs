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
    target: (u16, u16),
    target_time: u16,
}

impl Mob {
    pub fn new(pos: (u16, u16), direction: Direction, is_dead: bool) -> Self {
        Mob {
            pos,
            direction,
            is_dead,
            pos_animation: Default::default(),
            direction_animation: Default::default(),
            target: pos,
            target_time: 0,
        }
    }

    /// Util method for working out whether a mob at the given location could advance (there was no
    /// mob blocking and there was no wall).
    ///
    /// It will return the coordinates of the location in Some
    fn can_advance(
        pos: (u16, u16),
        direction: Direction,
        map: &Map,
        entities: &Grid<Option<EntityIndex>>,
    ) -> Option<(u16, u16)> {
        if let Some((new_x, new_y)) = map.calc_foward(pos.0, pos.1, &direction) {
            if let Some(entity_index) = &entities[new_x as usize][new_y as usize] {
                if entity_index.entity_type.is_mob() {
                    return None;
                }
            }

            if map.base_tile(new_x as usize, new_y as usize) != &BaseTile::Land {
                return None;
            }

            Some((new_x, new_y))
        } else {
            None
        }
    }

    /// Process the ai for when the pacman is at a junction.
    /// Returns None if there is no direction to turn.
    /// This method could simulate the ghosts ai at any tick, it's just that it's slightly less
    /// efficient since it does some extra calculations based on the target (very cheap
    /// calculations)
    fn junction(&self, map: &Map, entities: &Grid<Option<EntityIndex>>) -> Option<Direction> {
        use Direction::*;

        let delta = (
            self.pos.0 as i16 - self.target.0 as i16,
            self.pos.1 as i16 - self.target.1 as i16,
        );

        let preferred_x = if delta.0 > 0 {
            [East, West]
        } else {
            [West, East]
        };
        // Increasing y is down (south)
        let preferred_y = if delta.1 > 0 {
            [South, North]
        } else {
            [North, South]
        };

        // x direction has a greater magnitude
        let preferences = if delta.0.abs() >= delta.1.abs() {
            [
                preferred_x[0],
                preferred_y[0],
                preferred_x[1],
                preferred_y[1],
            ]
        } else {
            [
                preferred_y[0],
                preferred_x[0],
                preferred_y[1],
                preferred_x[1],
            ]
        };

        let reverse = self.direction.clockwise().clockwise();

        for direction in preferences.iter() {
            if direction == &reverse {
                continue;
            }

            if let Some(_) = Self::can_advance(self.pos, *direction, map, entities) {
                return Some(*direction);
            }
        }

        if Self::can_advance(self.pos, reverse, map, entities).is_some() {
            Some(reverse)
        } else {
            None
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
        debug_assert!(entities[self.pos.0 as usize][self.pos.1 as usize].is_some());

        if self.target_time == 0 || self.pos == self.target {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            self.target = (
                rng.gen_range(0, map.width()),
                rng.gen_range(0, map.height()),
            );
            self.target_time = 20;
        }

        // For now just use the junction ai even though it's slightly overkill for most ticks
        // if this returns None there's nowhere we can go so just wait
        if let Some(direction) = self.junction(map, entities) {
            if direction != self.direction {
                // We can only start turning if the direction is in the reverse
                if direction == self.direction.reverse() {
                    self.turn(self.direction().clockwise(), true);
                } else {
                    // The direction is either left or right which we can do in one tick
                    self.turn(direction, true);
                }

                return;
            }

            // Our direction is forward so let's process that
            let (new_x, new_y) = Self::can_advance(self.pos, direction, map, entities)
                .expect("Junction only returns a direction if we're able to move to it");

            debug_assert_eq!(
                (new_x, new_y),
                map.calc_foward(self.pos.0, self.pos.1, &direction).unwrap()
            );

            debug_assert_eq!(
                map.base_tile(new_x as usize, new_y as usize),
                &BaseTile::Land
            );

            if let Some(entity_index) = &entities[new_x as usize][new_y as usize] {
                assert!(
                    entity_index.entity_type().is_player(),
                    "We should not be given these coordinates if there is a mob in that location"
                );

                players.get(entity_index.index()).unwrap().borrow_mut();

                let mut enemy = players.get(entity_index.index()).unwrap().borrow_mut();
                if enemy.is_invulnerable() {
                    // do nothing if we can't attack them
                    return;
                }

                enemy.deal_damage(1);

                // only if the entity died do we continue and do the normal move forward
                // otherwise we do nothing for this turn (except for the attack that didn't
                // kill)
                if !enemy.died() {
                    return;
                }

                // remove the dead enemy from the board
                entities[new_x as usize][new_y as usize] = None;
            }

            // The new_x, new_y should be empty so it should put None into the entity's original
            // position
            entities.swap(
                (self.pos.0 as usize, self.pos.1 as usize),
                (new_x as usize, new_y as usize),
            );
            self.set_pos((new_x, new_y), true);
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

    fn entity_type(&self) -> EntityType {
        EntityType::Mob
    }
}
