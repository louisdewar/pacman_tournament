use model::{Entity, EntityType, GameData, Grid};

use std::collections::HashMap;

use super::message::*;
use super::Player;

pub fn create_initial_message(
    game_id: usize,
    game_data: &GameData,
    id_map: &HashMap<usize, Player>,
) -> InitialMessage {
    let entities = Grid::from_column_major(
        game_data
            .entities
            .iter_column_major()
            .map(|entity_index| {
                if let Some(entity_index) = entity_index {
                    match entity_index.entity_type() {
                        EntityType::Mob => {
                            let mob = game_data.mobs.get(entity_index.index()).unwrap().borrow();
                            Some(CompleteEntityMetadata {
                                entity_type: entity_index.entity_type(),
                                variant: entity_index.index() as u8,
                                player_data: None,
                                dynamic: DynamicEntityMetadata {
                                    direction: mob.direction(),
                                    invulnerable: mob.is_invulnerable(),
                                    live_score: None,
                                },
                            })
                        }
                        EntityType::Player => {
                            let player = game_data
                                .players
                                .get(entity_index.index())
                                .unwrap()
                                .borrow();
                            let player_data = id_map.get(&entity_index.index()).unwrap();
                            let player_data = Some(PlayerStaticMetadata {
                                username: player_data.username.clone(),
                                high_score: player_data.prev_high_score,
                            });

                            Some(CompleteEntityMetadata {
                                entity_type: entity_index.entity_type(),
                                variant: entity_index.index() as u8,
                                player_data,
                                dynamic: DynamicEntityMetadata {
                                    direction: player.direction(),
                                    invulnerable: player.is_invulnerable(),
                                    live_score: Some(player.score()),
                                },
                            })
                        }
                    }
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
