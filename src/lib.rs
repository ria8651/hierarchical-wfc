pub use cpu_executer::*;
pub use graph::*;
use rand::Rng;
pub use tileset::*;

mod cpu_executer;
mod graph;
mod tileset;

pub trait Executer {
    fn execute<R: Rng>(&mut self, rng: &mut R, graph: &mut Peasant) -> bool;
}

pub struct Peasant<'a> {
    pub graph: Graph<WaveFunction>,
    pub constraints: &'a Vec<Vec<WaveFunction>>,
    pub weights: &'a Vec<u32>,
}

impl<'a> Peasant<'a> {
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
