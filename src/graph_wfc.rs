use crate::tileset::TileSet;
use anyhow::Result;
use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

pub const TILE_U32S: usize = 4;

#[derive(Deref, DerefMut, Clone, Copy, PartialEq, Eq)]
pub struct Cell(pub [u32; TILE_U32S]);

#[derive(Clone, Copy)]
pub struct Neighbor {
    pub direction: usize,
    pub index: usize,
}

pub struct GraphWfc<T: TileSet>
where
    [(); T::DIRECTIONS]:,
{
    pub tiles: Vec<Cell>,
    pub neighbors: Vec<Vec<Neighbor>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: TileSet> GraphWfc<T>
where
    [(); T::DIRECTIONS]:,
    [(); T::TILE_COUNT]:,
{
    pub fn new(size: UVec2) -> Self {
        let mut nodes_pos = Vec::new();
        for x in 0..size.x {
            for y in 0..size.y {
                nodes_pos.push(IVec2::new(x as i32, y as i32));
            }
        }

        let directions = [
            IVec2::new(0, 1),
            IVec2::new(0, -1),
            IVec2::new(-1, 0),
            IVec2::new(1, 0),
        ];

        let mut neighbors = Vec::new();
        for pos in nodes_pos.iter() {
            let mut node_neighbors = Vec::new();
            for (i, dir_vec) in directions.iter().enumerate() {
                let neighbor_pos = *pos + *dir_vec;
                if neighbor_pos.cmpge(size.as_ivec2()).any() {
                    continue;
                }
                if neighbor_pos.cmplt(IVec2::ZERO).any() {
                    continue;
                }

                let neighbor_index = (neighbor_pos.x * size.y as i32 + neighbor_pos.y) as usize;
                node_neighbors.push(Neighbor {
                    direction: i,
                    index: neighbor_index,
                });
            }
            neighbors.push(node_neighbors);
        }

        let filled = Cell::filled(T::TILE_COUNT);
        let tiles = vec![filled; nodes_pos.len()];

        Self {
            tiles,
            neighbors,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn collapse(&mut self, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let allowed_neighbors = T::allowed_neighbors();

        let start_node = rng.gen_range(0..self.tiles.len());

        // update cell
        self.tiles[start_node].select_random(&mut rng);

        let mut stack = vec![start_node];
        while let Some(index) = stack.pop() {
            for i in 0..self.neighbors[index].len() {
                // propagate changes
                let neighbor = self.neighbors[index][i];
                if self.propagate(index, neighbor, &allowed_neighbors) {
                    stack.push(neighbor.index);
                }
            }

            if stack.len() == 0 {
                // find next cell to update
                let mut min_entropy = usize::MAX;
                let mut min_pos = None;
                for (index, node) in self.tiles.iter().enumerate() {
                    let entropy = node.count_bits();
                    if entropy > 1 && entropy < min_entropy {
                        min_entropy = entropy;
                        min_pos = Some(index);
                    }
                }

                if let Some(pos) = min_pos {
                    // update cell
                    self.tiles[pos].select_random(&mut rng);
                    stack.push(pos);
                }
            }
        }

        // for y in (0..grid_wfc.grid[0].len()).rev() {
        //     for x in 0..grid_wfc.grid.len() {
        //         let tiles = &grid_wfc.grid[x][y];
        //         print!("{:<22}", format!("{:?}", tiles));
        //     }
        //     println!();
        // }
    }

    /// Returns true if the tile was updated
    pub fn propagate(
        &mut self,
        index: usize,
        neighbor: Neighbor,
        allowed_neighbors: &[[Cell; T::DIRECTIONS]; T::TILE_COUNT],
    ) -> bool {
        let mut updated = false;

        let mut allowed = Cell::empty();
        for tile in self.tiles[index].tile_iter() {
            allowed = Cell::join(&allowed, &allowed_neighbors[tile][neighbor.direction]);
        }

        let neighbor_tiles = self.tiles[neighbor.index].clone();
        let new_tiles = Cell::intersect(&neighbor_tiles, &allowed);
        if new_tiles != neighbor_tiles {
            updated = true;
            self.tiles[neighbor.index] = new_tiles;
        }

        updated
    }

    /// Consumes the grid and returns the collapsed tiles
    pub fn validate(self) -> Result<Vec<usize>> {
        let mut result = Vec::new();
        for node in 0..self.tiles.len() {
            if let Some(tile) = self.tiles[node].collapse() {
                result.push(tile);
            } else {
                return Err(anyhow::anyhow!("Invalid grid"));
            }
        }
        Ok(result)
    }
}

impl Cell {
    /// Cell fill with ones up to size
    fn filled(size: usize) -> Self {
        let mut result = [0; TILE_U32S];
        for i in 0..size {
            result[i / 32] |= 1 << (i % 32);
        }
        Self(result)
    }

    pub fn empty() -> Self {
        Self([0; TILE_U32S])
    }

    pub fn add_tile(&mut self, tile: usize) {
        self[tile / 32] |= 1 << (tile % 32);
    }

    /// Leaves a random bit set to 1 and the rest to 0
    fn select_random<R: Rng>(&mut self, rng: &mut R) {
        let selected = rng.gen_range(0..self.count_bits());
        let mut count = 0;
        for i in 0..TILE_U32S {
            for j in 0..32 {
                if self[i] & (1 << j) != 0 {
                    if count != selected {
                        self[i] &= !(1 << j);
                    }
                    count += 1;
                }
            }
        }
    }

    /// Returns the one and only tile if there is only one
    fn collapse(&self) -> Option<usize> {
        if self.count_bits() == 1 {
            Some(self.tile_iter().next().unwrap())
        } else {
            None
        }
    }

    fn join(a: &Self, b: &Self) -> Self {
        let mut result = [0; TILE_U32S];
        for i in 0..TILE_U32S {
            result[i] = a[i] | b[i];
        }
        Self(result)
    }

    fn intersect(a: &Self, b: &Self) -> Self {
        let mut result = [0; TILE_U32S];
        for i in 0..TILE_U32S {
            result[i] = a[i] & b[i];
        }
        Self(result)
    }

    /// Counts the number of bits set to 1
    fn count_bits(&self) -> usize {
        let mut result = 0;
        for i in 0..TILE_U32S {
            result += self.0[i].count_ones() as usize;
        }
        result
    }

    /// Returns an iterator over all the set bits
    pub fn tile_iter(&self) -> impl Iterator<Item = usize> + '_ {
        (0..TILE_U32S).flat_map(move |i| {
            (0..32).filter_map(move |j| {
                if self[i] & (1 << j) != 0 {
                    Some(i * 32 + j)
                } else {
                    None
                }
            })
        })
    }
}

impl std::fmt::Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // print all the bits
        for i in 0..TILE_U32S {
            for j in 0..32 {
                if self[i] & (1 << j) != 0 {
                    write!(f, "1")?;
                } else {
                    write!(f, "0")?;
                }
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // print the number of bits
        write!(f, "{}", self.count_bits())
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum Direction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

impl Direction {
    pub fn other(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn rotate(&self, rotation: usize) -> Self {
        match rotation {
            0 => *self,
            1 => match self {
                Self::Up => Self::Right,
                Self::Down => Self::Left,
                Self::Left => Self::Up,
                Self::Right => Self::Down,
            },
            2 => match self {
                Self::Up => Self::Down,
                Self::Down => Self::Up,
                Self::Left => Self::Right,
                Self::Right => Self::Left,
            },
            3 => match self {
                Self::Up => Self::Left,
                Self::Down => Self::Right,
                Self::Left => Self::Down,
                Self::Right => Self::Up,
            },
            _ => panic!("Invalid rotation: {}", rotation),
        }
    }
}

impl From<usize> for Direction {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            _ => panic!("Invalid direction: {}", value),
        }
    }
}
