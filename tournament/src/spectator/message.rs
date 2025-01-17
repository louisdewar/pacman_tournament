use model::{BaseTile, Direction, EntityType, Food, Grid};

pub struct DeltaMessage {
    pub game_id: usize,
    pub entity_died: Vec<EntityDied>,
    pub entity_moved: Vec<EntityMoved>,
    pub entity_spawned: Vec<EntitySpawned>,
    pub food_eaten: Vec<FoodEaten>,
    pub food_spawned: Vec<FoodSpawned>,
    pub metadata_changed: Vec<MetadataChanged>,
}

pub struct EntityDied {
    pub position: u32,
}

pub struct EntityMoved {
    pub start: u32,
    pub end: u32,
}

pub struct EntitySpawned {
    pub position: u32,
    pub metadata: CompleteEntityMetadata,
}

pub struct FoodEaten {
    pub position: u32,
}

pub struct FoodSpawned {
    pub position: u32,
    pub food_type: Food,
}

pub struct MetadataChanged {
    pub position: u32,
    pub metadata: DynamicEntityMetadata,
}

pub struct DynamicEntityMetadata {
    pub direction: Direction,
    pub invulnerable: bool,
    /// Players have a live score
    pub live_score: Option<u32>,
}

pub struct PlayerStaticMetadata {
    pub high_score: u32,
    pub username: String,
}

pub struct CompleteEntityMetadata {
    pub entity_type: EntityType,
    pub variant: u8,
    pub dynamic: DynamicEntityMetadata,
    pub player_data: Option<PlayerStaticMetadata>,
}

pub struct InitialMessage {
    pub game_id: usize,
    pub width: u16,
    pub height: u16,
    pub base_tiles: Grid<BaseTile>,
    pub entities: Grid<Option<CompleteEntityMetadata>>,
    pub food: Grid<Option<Food>>,
}
