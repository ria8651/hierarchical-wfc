use crate::{tileset::*, wfc_graph::*};
use bevy::prelude::*;
use crossbeam::channel::Sender;
use rand::Rng;
use std::{any::Any, sync::Arc};

pub type Metadata = Option<Arc<dyn Any + Send + Sync>>;

#[derive(Clone, Debug, PartialEq, Reflect, Default)]
#[reflect(Default)]
pub struct WfcSettings {
    pub backtracking: BacktrackingSettings,
    pub entropy: Entropy,
    pub progress_updates: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Default)]
pub enum BacktrackingHeuristic {
    Standard,
    Fixed { distance: usize },
    Degree { degree: usize },
    Proportional { proportion: f32 },
}

impl Default for BacktrackingHeuristic {
    fn default() -> Self {
        BacktrackingHeuristic::Degree { degree: 3 }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Default)]
pub enum BacktrackingSettings {
    Disabled,
    Enabled {
        restarts_left: usize,
        heuristic: BacktrackingHeuristic,
    },
}

impl Default for BacktrackingSettings {
    fn default() -> Self {
        BacktrackingSettings::Enabled {
            restarts_left: 100,
            heuristic: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Default)]
pub enum Entropy {
    #[default]
    TileCount,
    Scanline,
    Shannon,
}

pub struct WfcTask {
    pub graph: Graph<WaveFunction>,
    pub tileset: Arc<dyn TileSet>,
    pub seed: u64,
    pub metadata: Metadata,
    pub settings: WfcSettings,
    pub update_channel: Option<Sender<(Graph<WaveFunction>, Metadata)>>,
}

impl WfcTask {
    /// Todo: Use the weights when calculating the entropy
    pub fn lowest_entropy<R: Rng>(&self, rng: &mut R) -> Option<usize> {
        let weights = self.tileset.get_weights();

        if let Entropy::Scanline = self.settings.entropy {
            for (index, node) in self.graph.tiles.iter().enumerate() {
                let bits = node.count_bits();
                if bits > 1 {
                    return Some(index);
                }
            }
            return None;
        }

        // find next cell to update
        let mut min_entropy = f32::MAX;
        let mut min_index = None;
        let mut with_min: usize = 0; // Track how many nodes has the lowest entropy found
        for (index, node) in self.graph.tiles.iter().enumerate() {
            let bits = node.count_bits();
            if bits > 1 {
                let entropy = match self.settings.entropy {
                    Entropy::TileCount => bits as f32,
                    Entropy::Shannon => {
                        let log_weight: f32 = node
                            .tile_iter()
                            .map(|t| weights[t] * weights[t].log2())
                            .sum();
                        let bits = node.count_bits() as f32;
                        bits.log2() - log_weight / bits
                    }
                    Entropy::Scanline => unreachable!(),
                };
                if entropy <= min_entropy {
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
