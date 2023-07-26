use crate::tileset::{AllowedNeighbors, TileSet};
use anyhow::Result;
use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use rand::{rngs::StdRng, Rng, SeedableRng};

#[derive(Debug)]
pub struct WfcNode<T: TileSet> {
    pub tiles: HashSet<T::Tile>,
    pub neighbors: HashMap<Direction, usize>,
}

pub struct GraphWfc<T: TileSet> {
    pub nodes: Vec<WfcNode<T>>,
}

impl<T: TileSet> GraphWfc<T> {
    pub fn new(size: UVec2) -> Self {
        let tiles = T::all_tiles();

        let mut nodes_pos = Vec::new();
        for x in 0..size.x {
            for y in 0..size.y {
                nodes_pos.push(IVec2::new(x as i32, y as i32));
            }
        }

        let directions = [
            (Direction::Up, IVec2::new(0, 1)),
            (Direction::Down, IVec2::new(0, -1)),
            (Direction::Left, IVec2::new(-1, 0)),
            (Direction::Right, IVec2::new(1, 0)),
        ];

        let mut nodes = Vec::new();
        for pos in nodes_pos.iter() {
            let mut neighbors = HashMap::new();
            for (dir, dir_vec) in directions.iter() {
                let neighbor_pos = *pos + *dir_vec;
                if let Some(neighbor_index) = nodes_pos.iter().position(|p| p == &neighbor_pos) {
                    neighbors.insert(*dir, neighbor_index);
                }
            }
            nodes.push(WfcNode {
                tiles: tiles.clone(),
                neighbors,
            });
        }

        Self { nodes }
    }

    pub fn collapse(&mut self, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let allowed_neighbors = T::allowed_neighbors();

        let start_node = rng.gen_range(0..self.nodes.len());

        // update cell
        let tiles = [T::random_tile(&mut rng)].into();
        self.nodes[start_node].tiles = tiles;

        let mut stack = vec![start_node];
        while let Some(index) = stack.pop() {
            let neighbors = self.nodes[index].neighbors.clone();
            for (neighbor_direction, neighbor_index) in neighbors.into_iter() {
                // propagate changes
                if self.propagate(
                    index,
                    neighbor_index,
                    &neighbor_direction,
                    &allowed_neighbors,
                ) {
                    stack.push(neighbor_index);
                }
            }

            if stack.len() == 0 {
                // find next cell to update
                let mut min_entropy = usize::MAX;
                let mut min_pos = None;
                for (index, node) in self.nodes.iter().enumerate() {
                    let entropy = node.tiles.len();
                    if entropy > 1 && entropy < min_entropy {
                        min_entropy = entropy;
                        min_pos = Some(index);
                    }
                }

                if let Some(pos) = min_pos {
                    // update cell
                    let tiles = self.nodes[pos]
                        .tiles
                        .iter()
                        .cloned()
                        .collect::<Vec<<T as TileSet>::Tile>>();
                    let length = tiles.len();
                    self.nodes[pos].tiles = [tiles[rng.gen_range(0..length)]].into();

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
        neighbor_index: usize,
        neighbor_direction: &Direction,
        allowed_neighbors: &AllowedNeighbors<T>,
    ) -> bool {
        let mut updated = false;

        let tiles = &self.nodes[index].tiles;
        let neighbor_tiles = self.nodes[neighbor_index].tiles.clone();

        let mut allowed = HashSet::new();
        for tile in tiles {
            allowed.extend(&allowed_neighbors[tile][neighbor_direction]);
        }

        let new_tiles = neighbor_tiles.intersection(&allowed).copied().collect();
        if new_tiles != neighbor_tiles {
            updated = true;
            self.nodes[neighbor_index].tiles = new_tiles;
        }

        updated
    }

    /// Consumes the grid and returns the collapsed tiles
    pub fn validate(self) -> Result<Vec<T::Tile>> {
        let mut result = Vec::new();
        for node in 0..self.nodes.len() {
            let tiles = &self.nodes[node].tiles;
            if tiles.len() != 1 {
                return Err(anyhow::anyhow!("Invalid grid"));
            }
            result.push(*tiles.iter().next().unwrap());
        }
        Ok(result)
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
