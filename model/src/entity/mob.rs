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
    stuck: u8,
    path: Vec<(u16, u16)>,
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
            stuck: 0,
            path: Vec::new(),
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

    /// Finds a path that gets closer to the target ignoring entities
    /// The vector is in reverse order (by popping you get the next point to go to).
    fn find_path_to_target(
        &self,
        map: &Map,
        excluded_direction: Option<Direction>,
    ) -> Vec<(u16, u16)> {
        let energy_start = 9;
        use std::collections::{HashMap, VecDeque};
        use Direction::*;
        struct PathNode {
            parent: Option<(u16, u16)>,
            unblocked: u8,
            energy: u8,
        }

        let mut paths = HashMap::new();

        let mut unsearched_points = VecDeque::new();
        // Note: complexity is energy^2 in the worst case (completely open land)
        unsearched_points.push_front(self.pos);
        paths.insert(
            self.pos,
            PathNode {
                parent: None,
                unblocked: 4,
                energy: energy_start,
            },
        );

        let mut propogate = |pos: (u16, u16), unsearched_points: &mut VecDeque<_>| {
            let node = paths.get(&pos).unwrap();
            let parent = node.parent;
            let energy = node.energy;
            let mut unblocked = 0;
            for direction in &[North, East, South, West] {
                // We're the top node and this direction has been excluded
                if energy == energy_start && Some(direction) == excluded_direction.as_ref() {
                    continue;
                }

                if let Some(new_pos) = map.calc_foward(pos.0, pos.1, direction) {
                    if map.base_tile(new_pos.0 as usize, new_pos.1 as usize) != &BaseTile::Land {
                        continue;
                    }

                    // It counts as blocked if another path is already searching this point
                    // The reasoning is that paths are propogated in order of highest energy so if
                    // it's currently being checked there is a shorter or equal path
                    if paths.get(&new_pos).is_none() {
                        if energy > 0 {
                            unsearched_points.push_back(new_pos);
                            paths.insert(
                                new_pos,
                                PathNode {
                                    parent: Some(pos),
                                    // We'll properly fill this once we propogate it but we
                                    // have at most 3
                                    unblocked: 3,
                                    energy: energy - 1,
                                },
                            );
                        }
                        // Even if our energy is 0 we count this as unblocked since it might
                        // be
                        unblocked += 1;
                    }
                }
            }

            // We have to reborrow since need to borrow above
            let node = paths.get_mut(&pos).unwrap();
            node.unblocked = unblocked;

            if unblocked == 0 {
                let mut parent = parent;
                while let Some(parent_pos) = parent {
                    let parent_path_node = paths.get_mut(&parent_pos).unwrap();
                    parent_path_node.unblocked -= 1;

                    if parent_path_node.unblocked == 0 {
                        parent = parent_path_node.parent;
                    } else {
                        parent = None;
                    }
                }
            }
        };

        while let Some(pos) = unsearched_points.pop_front() {
            propogate(pos, &mut unsearched_points);
        }

        // Note we don't want to add the current location to these points
        let produce_path_to_point = |point: (u16, u16)| {
            let mut path = Vec::new();
            let mut parent_pos = point;

            while parent_pos != self.pos {
                path.push(parent_pos);
                let parent_node = paths.get(&parent_pos).unwrap();
                parent_pos = parent_node.parent.unwrap();
            }

            return path;
        };

        let delta = (
            self.pos.0 as i16 - self.target.0 as i16,
            self.pos.1 as i16 - self.target.1 as i16,
        );
        let mut best_pos = (delta.0.abs() + delta.1.abs(), self.pos, false, energy_start);
        // We either find a point that gets within 4 tiles delta x + delta y (best case) or we find
        // the tile that is unblocked and is the closest and follow that path, it does not make
        // sense for there to be no unblocked paths unless somehow we checked the entire map
        for (pos, path_node) in &paths {
            let delta = (
                pos.0 as i16 - self.target.0 as i16,
                pos.1 as i16 - self.target.1 as i16,
            );
            let score = delta.0.abs() + delta.1.abs();
            // We're very close it doesn't matter whether the path is blocked or not
            if score < 3 && self.pos != *pos {
                // We're within the target region
                best_pos = (score, *pos, false, path_node.energy);
                break;
            } else {
                // We're not close to target so we only accept unblocked paths
                if path_node.unblocked > 0 {
                    // We only care about leaf paths (energy = 1)
                    if path_node.energy == 1 {
                        // Lower score is better, therefore this is a better unblocked leaf
                        if score < best_pos.0 || best_pos.3 != 1 {
                            best_pos = (score, *pos, false, path_node.energy);
                        }
                    }
                }
            }
        }

        produce_path_to_point(best_pos.1)
    }

    fn try_advance(
        &mut self,
        map: &Map,
        players: &PlayerBucket,
        entities: &mut Grid<Option<EntityIndex>>,
    ) -> bool {
        if let Some((new_x, new_y)) = Self::can_advance(self.pos, self.direction, map, entities) {
            if let Some(entity_index) = &entities[new_x as usize][new_y as usize] {
                assert!(
                    entity_index.entity_type().is_player(),
                    "Can advance would not return position if there was a mob there"
                );

                let mut enemy = players.get(entity_index.index()).unwrap().borrow_mut();
                if enemy.is_invulnerable() {
                    // do nothing if we can't attack them
                    return false;
                }

                enemy.deal_damage(1);

                // only if the entity died do we continue and do the normal move forward
                // otherwise we do nothing for this turn (except for the attack that didn't
                // kill)
                if !enemy.died() {
                    return false;
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

            return true;
        }

        false
    }

    fn process_path(
        &mut self,
        map: &Map,
        players: &PlayerBucket,
        entities: &mut Grid<Option<EntityIndex>>,
    ) {
        if let Some(next_point) = self.path.last() {
            assert_ne!(*next_point, self.pos);
            let direction = map.calc_direction(self.pos, *next_point);
            if direction == self.direction {
                if self.try_advance(map, players, entities) {
                    self.stuck = 0;
                    self.path.pop();
                } else {
                    self.stuck += 1;
                }
            } else if direction == self.direction.reverse() {
                self.turn(self.direction.clockwise(), true);
            } else {
                // The direction is either left or right which we can do in one tick
                self.turn(direction, true);
            }
            return;
        } else {
            let excluded_direction =
                if let Some(new_pos) = map.calc_foward(self.pos.0, self.pos.1, &self.direction) {
                    if let Some(entity_index) = &entities[new_pos.0 as usize][new_pos.1 as usize] {
                        if entity_index.entity_type().is_mob() {
                            // Avoid getting stuck with mobs
                            Some(self.direction)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
            self.path = self.find_path_to_target(map, excluded_direction);
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

        if self.target_time == 0 || self.pos == self.target || self.stuck > 5 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            self.target = (
                rng.gen_range(0, map.width()),
                rng.gen_range(0, map.height()),
            );
            self.target_time = 50;
            self.path.truncate(0);
            self.stuck = 0;
        }

        self.target_time -= 1;
        self.process_path(map, players, entities);
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
