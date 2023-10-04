use crate::tileset::*;
use crate::wfc_graph::*;

use rand::Rng;
use std::{any::Any, sync::Arc};

pub type Metadata = Option<Arc<dyn Any + Send + Sync>>;

pub struct BacktrackingSettings {
    pub max_restarts: usize,
}
impl Default for BacktrackingSettings {
    fn default() -> Self {
        Self { max_restarts: 100 }
    }
}
pub struct WfcTask {
    pub graph: Graph<WaveFunction>,
    pub tileset: Arc<dyn TileSet>,
    pub seed: u64,
    pub metadata: Metadata,
    pub backtracking: BacktrackingSettings,
}

impl WfcTask {
    /// Todo: Use the weights when calculating the entropy
    pub fn lowest_entropy<R: Rng>(&self, rng: &mut R) -> Option<usize> {
        // find next cell to update
        let mut min_entropy = usize::MAX;
        let mut min_index = None;
        let mut with_min: usize = 0; // Track how many nodes has the lowest entropy found
        for (index, node) in self.graph.tiles.iter().enumerate() {
            let entropy = node.count_bits();
            if entropy > 1 && entropy <= min_entropy {
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

        // combine all constraints of all the tiles that the neighbor can be according to what the current tile can be
        let constraints = self.tileset.get_constraints();
        let mut allowed = WaveFunction::empty();
        for tile in self.graph.tiles[index].tile_iter() {
            allowed = WaveFunction::join(&allowed, &constraints[tile][neighbor.direction]);
        }

        let neighbor_tiles = self.graph.tiles[neighbor.index].clone();
        let new_tiles = WaveFunction::intersect(&neighbor_tiles, &allowed);
        if new_tiles != neighbor_tiles {
            updated = true;
            self.graph.tiles[neighbor.index] = new_tiles;
        }

        updated
    }

    pub fn clear(&mut self) {
        for tile in self.graph.tiles.iter_mut() {
            *tile = WaveFunction::filled(self.tileset.tile_count());
        }
    }
}
