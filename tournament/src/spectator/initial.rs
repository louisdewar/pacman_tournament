use model::{GameData, Grid};

use super::message::*;

pub fn create_initial_message(game_id: usize, game_data: &GameData) -> InitialMessage {
    let entities = Grid::from_column_major(
        game_data
            .entities
            .iter_column_major()
            .map(|entity_index| {
                if let Some(entity_index) = entity_index {
                    let entity = entity_index
                        .as_entity(&game_data.mobs, &game_data.players)
                        .borrow();
                    Some(CompleteEntityMetadata {
                        entity_type: entity_index.entity_type(),
                        variant: entity_index.index() as u8,
                        dynamic: DynamicEntityMetadata {
                            direction: entity.direction(),
                            invulnerable: entity.is_invulnerable(),
                        },
                    })
                } else {
                    None
                }
            })
            .collect(),
        game_data.map.height() as usize,
        game_data.map.width() as usize,
    );

    InitialMessage {
        game_id,
        entities,
        width: game_data.map.width(),
        height: game_data.map.height(),
        base_tiles: game_data.map.base_tiles().clone(),
        food: game_data.food.clone(),
    }
}
