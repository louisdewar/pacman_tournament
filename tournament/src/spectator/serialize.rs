use model::{BaseTile, Direction, EntityType, Food, Grid};

use super::message::*;

/// This trait is kept private since there are implementation details that mean certain elements
/// that implement this trait would produce non-deserializable output (e.g. with sparse grid)
trait Serialize {
    fn serialize(&self, out: &mut String);
}

// A grid of option T is a sparse grid
impl<T: Serialize> Serialize for Grid<Option<T>> {
    /// The algorithm will insert numbers between sparse elements, but this relies on each element not
    /// containing numerics at the start.
    fn serialize(&self, out: &mut String) {
        let mut skip = 0;
        for item in self.iter_column_major() {
            if let Some(item) = item {
                // There were missing elements before this one
                if skip > 0 {
                    out.push_str(&format!("{}", skip));
                    skip = 0;
                }

                item.serialize(out);
            } else {
                skip += 1;
            }
        }

        if skip > 0 {
            out.push_str(&format!("{}", skip));
        }
    }
}

impl<T: Serialize> Serialize for Grid<T> {
    fn serialize(&self, out: &mut String) {
        for item in self.iter_column_major() {
            // It is assumed that T has the appropriate separators.
            // This holds since the only vecs that we serialize are vecs of messages e.g.
            // EntityDied.
            // They were all written with that in mind.
            item.serialize(out);
        }
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self, out: &mut String) {
        for item in self {
            item.serialize(out);
        }
    }
}

impl Serialize for Food {
    fn serialize(&self, out: &mut String) {
        match self {
            Food::Fruit => out.push('F'),
            Food::PowerPill => out.push('P'),
        }
    }
}

impl Serialize for Direction {
    fn serialize(&self, out: &mut String) {
        match self {
            Direction::North => out.push('N'),
            Direction::East => out.push('E'),
            Direction::South => out.push('S'),
            Direction::West => out.push('W'),
        }
    }
}

impl Serialize for BaseTile {
    fn serialize(&self, out: &mut String) {
        match self {
            BaseTile::Land => out.push('L'),
            BaseTile::Wall => out.push('X'),
            BaseTile::Water => out.push('W'),
        }
    }
}
impl Serialize for EntityType {
    fn serialize(&self, out: &mut String) {
        match self {
            EntityType::Mob => out.push('M'),
            EntityType::Player => out.push('P'),
        }
    }
}

impl Serialize for DynamicEntityMetadata {
    fn serialize(&self, out: &mut String) {
        self.direction.serialize(out);
        if let Some(live_score) = self.live_score {
            out.push_str(&format!("{}", live_score));
        }
        out.push(if self.invulnerable { 'I' } else { 'V' });
    }
}

impl Serialize for CompleteEntityMetadata {
    fn serialize(&self, out: &mut String) {
        self.dynamic.serialize(out);
        self.entity_type.serialize(out);
        out.push_str(&format!("{}", self.variant));
        if let Some(player_metadata) = &self.player_data {
            debug_assert!(self.entity_type == EntityType::Player);
            player_metadata.serialize(out);
        }
    }
}

impl Serialize for PlayerStaticMetadata {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!(
            "{}-{}{},",
            self.username.len(),
            self.username,
            self.high_score
        ));
    }
}

impl Serialize for EntityDied {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!("{},", self.position));
    }
}

impl Serialize for EntityMoved {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!("{},{},", self.start, self.end));
    }
}

impl Serialize for EntitySpawned {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!("{}", self.position));
        self.metadata.serialize(out);
    }
}

impl Serialize for FoodEaten {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!("{},", self.position));
    }
}

impl Serialize for FoodSpawned {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!("{}", self.position));
        self.food_type.serialize(out);
    }
}

impl Serialize for MetadataChanged {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!("{}", self.position));
        self.metadata.serialize(out);
    }
}

impl Serialize for DeltaMessage {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!("d{}_", self.game_id));

        if self.entity_died.len() > 0 {
            out.push('a');
            self.entity_died.serialize(out);
        }

        if self.entity_moved.len() > 0 {
            out.push('b');
            self.entity_moved.serialize(out);
        }

        if self.entity_spawned.len() > 0 {
            out.push('c');
            self.entity_spawned.serialize(out);
        }

        if self.food_eaten.len() > 0 {
            out.push('d');
            self.food_eaten.serialize(out);
        }

        if self.food_spawned.len() > 0 {
            out.push('e');
            self.food_spawned.serialize(out);
        }

        if self.metadata_changed.len() > 0 {
            out.push('f');
            self.metadata_changed.serialize(out);
        }
    }
}

impl Serialize for InitialMessage {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!(
            "i{}_{}_{}_",
            self.game_id, self.width, self.height
        ));

        self.base_tiles.serialize(out);
        out.push('|');
        self.entities.serialize(out);
        out.push('|');
        self.food.serialize(out);
    }
}

impl Serialize for db::model::LeaderboardUser {
    fn serialize(&self, out: &mut String) {
        out.push_str(&format!(
            "{}_{}_{},",
            self.id, self.username, self.high_score
        ));
    }
}

// Since Serialize is private we must provide functions for external users to call

pub fn serialized_initial(initial: &InitialMessage) -> String {
    let mut out = String::new();
    initial.serialize(&mut out);

    out
}

pub fn serialized_delta(delta: &DeltaMessage) -> String {
    let mut out = String::new();
    delta.serialize(&mut out);

    out
}

pub fn serialize_leaderboard(leaderboard: &Vec<db::model::LeaderboardUser>) -> String {
    let mut out = String::from("l");
    leaderboard.serialize(&mut out);

    out
}
