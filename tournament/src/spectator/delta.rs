use model::{Bucket, Entity, EntityIndex, Food, GameData, Grid};

use std::cell::RefCell;

use super::message::*;

fn flatten_coordinate(height: u32, (x, y): (u16, u16)) -> u32 {
    x as u32 * height as u32 + y as u32
}

pub fn find_entity_deltas<E: Entity>(
    old_entities: &Bucket<RefCell<E>>,
    new_entities: &Bucket<RefCell<E>>,
    // The only purpose of the grid is for an extra assert to ensure that positions match the index
    // in the grid
    old_grid: &Grid<Option<EntityIndex>>,
    entity_died: &mut Vec<EntityDied>,
    entity_moved: &mut Vec<EntityMoved>,
    entity_spawned: &mut Vec<EntitySpawned>,
    metadata_changed: &mut Vec<MetadataChanged>,
    height: u32,
) {
    for (entity_id, old_entity) in old_entities.iter() {
        if let Some(new_entity) = new_entities.get(*entity_id) {
            let new_pos = new_entity.borrow().position();
            let old_pos = old_entity.borrow().position();

            let new_direction = new_entity.borrow().direction();
            let old_direction = old_entity.borrow().direction();

            let new_invulnerable = new_entity.borrow().is_invulnerable();
            let old_invulnerable = old_entity.borrow().is_invulnerable();

            if new_pos != old_pos {
                entity_moved.push(EntityMoved {
                    start: flatten_coordinate(height, old_pos),
                    end: flatten_coordinate(height, new_pos),
                });
            }

            if new_direction != old_direction || new_invulnerable != old_invulnerable {
                metadata_changed.push(MetadataChanged {
                    metadata: DynamicEntityMetadata {
                        invulnerable: new_invulnerable,
                        direction: new_direction,
                    },
                });
            }
        } else {
            entity_died.push(EntityDied {
                position: flatten_coordinate(height, old_entity.borrow().position()),
            });
        }

        let (old_x, old_y) = old_entity.borrow().position();
        assert!(old_grid[old_x as usize][old_y as usize].is_some());
        assert_eq!(
            old_grid[old_x as usize][old_y as usize]
                .as_ref()
                .unwrap()
                .index(),
            *entity_id
        );
    }

    // Find the new entities that weren't in old entities
    for (entity_id, new_entity) in new_entities
        .iter()
        .filter(|(id, _)| old_entities.get(**id).is_none())
    {
        let new_entity = new_entity.borrow();
        let position = new_entity.position();
        let invulnerable = new_entity.is_invulnerable();
        let direction = new_entity.direction();
        let entity_type = new_entity.entity_type();

        entity_spawned.push(EntitySpawned {
            position: flatten_coordinate(height, position),
            metadata: CompleteEntityMetadata {
                entity_type,
                variant: *entity_id as u8,
                dynamic: DynamicEntityMetadata {
                    invulnerable,
                    direction,
                },
            },
        });
    }
}

fn find_food_deltas(
    old_food: &Grid<Option<Food>>,
    new_food: &Grid<Option<Food>>,
    food_eaten: &mut Vec<FoodEaten>,
    food_spawned: &mut Vec<FoodSpawned>,
) {
    for (position, (old_food, new_food)) in old_food
        .iter_column_major()
        .zip(new_food.iter_column_major())
        .enumerate()
    {
        match (old_food, new_food) {
            (Some(_), None) => food_eaten.push(FoodEaten {
                position: position as u32,
            }),
            // No change
            (Some(old), Some(new)) if old == new => {}
            // This includes the case if the food changes type, spawned will overwrite either null
            // or the old food type
            // It also handles the case when the old was None
            (_, Some(new)) => food_spawned.push(FoodSpawned {
                position: position as u32,
                food_type: new.clone(),
            }),
            (None, None) => {}
        }
    }
}

pub fn create_delta_message(
    game_id: usize,
    old_state: &GameData,
    new_state: &GameData,
) -> DeltaMessage {
    let mut entity_died = Vec::new();
    let mut entity_moved = Vec::new();
    let mut entity_spawned = Vec::new();
    let mut food_eaten = Vec::new();
    let mut food_spawned = Vec::new();
    let mut metadata_changed = Vec::new();

    let old_players = &old_state.players;
    let new_players = &new_state.players;

    let old_mobs = &old_state.mobs;
    let new_mobs = &new_state.mobs;

    let old_food = &old_state.food;
    let new_food = &new_state.food;

    let height = old_state.map.height() as u32;

    let old_grid = &old_state.entities;

    find_entity_deltas(
        old_players,
        new_players,
        old_grid,
        &mut entity_died,
        &mut entity_moved,
        &mut entity_spawned,
        &mut metadata_changed,
        height,
    );

    find_entity_deltas(
        old_mobs,
        new_mobs,
        old_grid,
        &mut entity_died,
        &mut entity_moved,
        &mut entity_spawned,
        &mut metadata_changed,
        height,
    );

    find_food_deltas(old_food, new_food, &mut food_eaten, &mut food_spawned);

    DeltaMessage {
        game_id,
        entity_died,
        entity_moved,
        entity_spawned,
        food_eaten,
        food_spawned,
        metadata_changed,
    }
}
