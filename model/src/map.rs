#[derive(Clone, Debug)]
pub struct Map {
    pub width: u16,
    pub height: u16,
    pub base_tile: Vec<BaseTile>,
    pub player_spawn: SpawnLocation,
    pub mob_spawn: SpawnLocation,
}

impl Map {
    pub fn new(width: u16, height: u16) -> Map {
        let mut base_tile = vec![BaseTile::Land; width as usize * height as usize];
        base_tile[0] = BaseTile::Water;
        base_tile[30] = BaseTile::Wall;

        Map {
            width,
            height,
            base_tile,
            player_spawn: SpawnLocation::Defined(vec![(10, 10)]),
            mob_spawn: SpawnLocation::Defined(vec![(10, 13)]),
        }
    }

    /// Applies the given direction to the coordinates, returns None if the coordinates would
    /// be off of the map
    pub fn calc_foward(&self, x: u16, y: u16, direction: &crate::Direction) -> Option<(u16, u16)> {
        use crate::Direction::*;
        let new = match direction {
            North => {
                if y == 0 {
                    return None;
                }
                (x, y - 1)
            }
            East => {
                if x + 1 == self.width {
                    return None;
                } else {
                    (x + 1, y)
                }
            }
            South => {
                if y + 1 == self.height {
                    return None;
                } else {
                    (x, y + 1)
                }
            }
            West => {
                if x == 0 {
                    return None;
                } else {
                    (x - 1, y)
                }
            }
        };

        Some(new)
    }

    pub fn flatten_coordinate(&self, x: usize, y: usize) -> usize {
        x + y * self.width as usize
    }

    pub fn base_tile(&self, x: usize, y: usize) -> &BaseTile {
        &self.base_tile[self.flatten_coordinate(x, y)]
    }

    pub fn width(&self) -> usize {
        self.width as usize
    }

    pub fn height(&self) -> usize {
        self.height as usize
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BaseTile {
    #[serde(rename(serialize = "W"))]
    Water,
    #[serde(rename(serialize = "L"))]
    Land,
    #[serde(rename(serialize = "X"))]
    Wall,
}

// impl BaseTile {
//     pub fn texture(&self) -> ggez::graphics::Color {
//         match &self {
//             BaseTile::Water => (0.0, 0.0, 1.0).into(),
//             BaseTile::Land => (0.0, 1.0, 0.0).into(),
//             BaseTile::Wall => (0.2, 0.2, 0.2).into(),
//         }
//     }
// }

#[derive(Clone, Debug)]
pub enum SpawnLocation {
    Random,
    Defined(Vec<(u16, u16)>),
}
