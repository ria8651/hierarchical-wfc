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
    pub neighbors: Vec<usize>,
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

        let orth = [
            IVec2::new(0, 1),
            IVec2::new(0, -1),
            IVec2::new(-1, 0),
            IVec2::new(1, 0),
        ];

        let mut nodes = Vec::new();
        for pos in nodes_pos.iter() {
            let mut neighbors = Vec::new();
            for dir in orth.iter() {
                let neighbor_pos = *pos + *dir;
                if nodes_pos.iter().any(|p| p == &neighbor_pos) {
                    neighbors.push(nodes_pos.iter().position(|p| p == &neighbor_pos).unwrap());
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
            let node = &self.nodes[index];
            for neighbor in node.neighbors.iter() {
                // propagate changes
                if self.propagate(index, *neighbor, &allowed_neighbors) {
                    stack.push(*neighbor);
                }
            }

            if stack.len() == 0 {
                // find next cell to update
                let mut min_entropy = usize::MAX;
                let mut min_pos = None;
                for x in 0..self.grid.len() {
                    for y in 0..self.grid[0].len() {
                        let pos = IVec2::new(x as i32, y as i32);
                        let tiles = &self.grid[x][y];
                        let entropy = tiles.len();
                        if entropy > 1 && entropy < min_entropy {
                            min_entropy = entropy;
                            min_pos = Some(pos);
                        }
                    }
                }

                if let Some(pos) = min_pos {
                    // update cell
                    let tiles = self.grid[pos.x as usize][pos.y as usize].clone();
                    let tile = *tiles.iter().nth(rng.gen_range(0..tiles.len())).unwrap();
                    self.grid[pos.x as usize][pos.y as usize] = [tile].into();

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
        allowed_neighbors: &AllowedNeighbors<T>,
    ) -> bool {
        let mut updated = false;

        let tiles = &self.nodes[index].tiles;
        let neighbor_tiles = self.nodes[neighbor_index].tiles.clone();

        let mut allowed = HashSet::new();
        for tile in tiles {
            allowed.extend(&allowed_neighbors[tile][dir_index]);
        }

        let new_tiles = neighbor_tiles.intersection(&allowed).copied().collect();
        if new_tiles != neighbor_tiles {
            updated = true;
            self.grid[pos.x as usize][pos.y as usize] = new_tiles;
        }

        updated
    }

    // /// Consumes the grid and returns the collapsed tiles
    // pub fn validate(self) -> Result<Vec<Vec<T::Tile>>> {
    //     let mut result = Vec::new();
    //     for x in 0..self.grid.len() {
    //         let mut row = Vec::new();
    //         for y in 0..self.grid[0].len() {
    //             let tiles = &self.grid[x][y];
    //             if tiles.len() != 1 {
    //                 return Err(anyhow::anyhow!("Invalid grid"));
    //             }
    //             row.push(*tiles.iter().next().unwrap());
    //         }
    //         result.push(row);
    //     }
    //     Ok(result)
    // }

    // fn get_tiles(&self, pos: IVec2) -> Option<&HashSet<T::Tile>> {
    //     if self.in_bounds(pos) {
    //         Some(&self.grid[pos.x as usize][pos.y as usize])
    //     } else {
    //         None
    //     }
    // }

    // fn in_bounds(&self, pos: IVec2) -> bool {
    //     pos.x >= 0
    //         && pos.x < self.grid.len() as i32
    //         && pos.y >= 0
    //         && pos.y < self.grid[0].len() as i32
    // }
}
