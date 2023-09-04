use anyhow::Result;
pub use cpu_executor::*;
pub use graph::*;
pub use multithreaded_executor::*;
use rand::Rng;
use std::{any::Any, sync::Arc};
pub use tileset::*;

mod cpu_executor;
mod graph;
mod multithreaded_executor;
mod tileset;

pub trait Executor {
    // fn execute(&mut self, graph: &mut Peasant) -> bool;
    fn queue_peasant(&mut self, peasant: Peasant) -> Result<()>;
}

pub struct Peasant {
    pub graph: Graph<WaveFunction>,
    pub constraints: Arc<Vec<Vec<WaveFunction>>>,
    pub weights: Arc<Vec<u32>>,
    pub seed: u64,
    pub user_data: Option<Box<dyn Any + Send + Sync>>,
}

impl Peasant {
    /// Todo: Use the weights when calculating the entropy
    pub fn lowest_entropy<R: Rng>(&self, rng: &mut R) -> Option<usize> {
        // find next cell to update
        let mut min_entropy = usize::MAX;
        let mut min_index = None;
        let mut with_min: usize = 0; // Track how many nodes has the lowest entropy found
        for (index, node) in self.graph.tiles.iter().enumerate() {
            let entropy = node.count_bits();
            if entropy > 1 && entropy <= min_entropy {
                with_min += 1;
                if entropy < min_entropy {
                    with_min = 1;
                    min_entropy = entropy;
                    min_index = Some(index);
                } else {
                    with_min += 1;

                    // Select new node so that all nodes with min_entropy have equal chance of been chosen
                    if rng.gen_bool(1.0f64 / with_min as f64) {
                        min_entropy = entropy;
                        min_index = Some(index);
                    }
                }
            }
        }

        min_index
    }

    /// Returns true if the tile was updated
    pub fn propagate(&mut self, index: usize, neighbor: Neighbor) -> bool {
        let mut updated = false;

        let mut allowed = WaveFunction::empty();
        for tile in self.graph.tiles[index].tile_iter() {
            allowed = WaveFunction::join(&allowed, &self.constraints[tile][neighbor.direction]);
        }

        let neighbor_tiles = self.graph.tiles[neighbor.index].clone();
        let new_tiles = WaveFunction::intersect(&neighbor_tiles, &allowed);
        if new_tiles != neighbor_tiles {
            updated = true;
            self.graph.tiles[neighbor.index] = new_tiles;
        }

        updated
    }
}
