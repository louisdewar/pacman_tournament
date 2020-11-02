use crate::{Food, Grid};

#[derive(Clone, Debug)]
pub struct Map {
    width: u16,
    height: u16,
    base_tile: Grid<BaseTile>,
    default_food_locations: Grid<Option<Food>>,
    player_spawn: SpawnLocation,
    mob_spawn: SpawnLocation,
}

impl Map {
    pub fn new(width: u16, height: u16) -> Map {
        let mut base_tile = Grid::fill_with_clone(BaseTile::Land, width as usize, height as usize);
        base_tile[0][0] = BaseTile::Water;
        base_tile[12][1] = BaseTile::Wall;

        let mut default_food_locations =
            Grid::fill_with_clone(None, width as usize, height as usize);
        default_food_locations[10][5] = Some(Food::Fruit);
        default_food_locations[10][3] = Some(Food::Fruit);
        default_food_locations[10][1] = Some(Food::PowerPill);

        Map {
            width,
            height,
            base_tile,
            default_food_locations,
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
        &self.base_tile[x][y]
    }

    pub fn base_tiles(&self) -> &Grid<BaseTile> {
        &self.base_tile
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn mob_spawn(&self) -> &SpawnLocation {
        &self.mob_spawn
    }

    pub fn player_spawn(&self) -> &SpawnLocation {
        &self.player_spawn
    }

    pub fn default_food_locations(&self) -> &Grid<Option<Food>> {
        &self.default_food_locations
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
