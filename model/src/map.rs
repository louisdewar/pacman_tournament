use crate::{Food, Grid};

#[derive(Clone, Debug, PartialEq)]
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

    pub fn new_from_string(input: &str) -> Map {
        let mut lines = input.lines().peekable();

        let mut x: u16 = 0;
        let mut y: u16 = 0;

        let mut rows = Vec::new();
        let mut player_spawn_locations = Vec::new();
        let mut mob_spawn_locations = Vec::new();

        let width = lines.peek().unwrap().len();

        for line in lines {
            assert_eq!(line.len(), width, "Map must be rectangular");
            let mut row = Vec::with_capacity(width);

            for c in line.chars() {
                let (food, base_tile) = match c {
                    'X' => (None, BaseTile::Wall),
                    ' ' => (None, BaseTile::Land),
                    '.' => (Some(Food::Fruit), BaseTile::Land),
                    '|' => (Some(Food::PowerPill), BaseTile::Land),
                    'P' => {
                        player_spawn_locations.push((x, y));
                        (None, BaseTile::Land)
                    }
                    'M' => {
                        mob_spawn_locations.push((x, y));
                        (None, BaseTile::Land)
                    }
                    c => panic!("Invalid map character: {}", c),
                };

                row.push((food, base_tile));
                x += 1;
            }

            x = 0;

            rows.push(row);
            y += 1;
        }

        let height = y as usize;

        let mut base_tile = Grid::fill_with_clone(BaseTile::Land, width, height);
        let mut default_food_locations = Grid::fill_with_clone(None, width, height);

        // We need to convert from row major to column major
        for x in 0..width {
            for y in 0..height {
                let (food, tile) = rows[y][x].clone();
                base_tile[x][y] = tile;
                default_food_locations[x][y] = food;
            }
        }

        assert_eq!(base_tile.len(), width * height);
        assert_eq!(default_food_locations.len(), width * height);

        let player_spawn = if player_spawn_locations.len() > 0 {
            SpawnLocation::Defined(player_spawn_locations)
        } else {
            SpawnLocation::Random
        };

        let mob_spawn = if mob_spawn_locations.len() > 0 {
            SpawnLocation::Defined(mob_spawn_locations)
        } else {
            SpawnLocation::Random
        };

        Map {
            default_food_locations,
            base_tile,
            player_spawn,
            mob_spawn,
            width: width as u16,
            height: height as u16,
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

#[derive(Clone, Debug, PartialEq)]
pub enum SpawnLocation {
    Random,
    Defined(Vec<(u16, u16)>),
}
