use bevy::{prelude::*, utils::HashMap};
use hierarchical_wfc::{Graph, WaveFunction};
use std::sync::Arc;

use crate::graph_grid::{self, GridGraphSettings};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkState {
    Scheduled,
    Done,
}

#[derive(Resource)]
pub struct World {
    pub world: Vec<Vec<WaveFunction>>,
    pub generated_chunks: HashMap<IVec2, ChunkState>,
    pub chunk_size: usize,
    pub seed: u64,
    pub current_constraints: Arc<Vec<Vec<WaveFunction>>>,
    pub current_weights: Arc<Vec<u32>>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            world: Vec::new(),
            generated_chunks: HashMap::new(),
            chunk_size: 0,
            seed: 0,
            current_constraints: Arc::new(Vec::new()),
            current_weights: Arc::new(Vec::new()),
        }
    }
}

impl World {
    pub fn extract_chunk(&self, pos: IVec2) -> Graph<WaveFunction> {
        let (bottom_left, top_right) = self.chunk_bounds(pos);
        let size = top_right - bottom_left;

        let settings = GridGraphSettings {
            width: size.x as usize,
            height: size.y as usize,
            periodic: false,
        };
        let mut graph = graph_grid::create(&settings, WaveFunction::empty());

        for x in 0..size.x {
            for y in 0..size.y {
                let tile = &self.world[(bottom_left.x + x) as usize][(bottom_left.y + y) as usize];
                graph.tiles[x as usize * size.y as usize + y as usize] = tile.clone();
            }
        }

        graph
    }

    pub fn merge_chunk(&mut self, chunk: IVec2, graph: Graph<WaveFunction>) {
        let (bottom_left, top_right) = self.chunk_bounds(chunk);
        let size = top_right - bottom_left;

        // Note: Assumes that the graph is a grid graph with a standard ordering
        for x in 0..size.x {
            for y in 0..size.y {
                let tile = graph.tiles[x as usize * size.y as usize + y as usize].clone();
                self.world[(bottom_left.x + x) as usize][(bottom_left.y + y) as usize] = tile;
            }
        }
    }

    pub fn chunk_bounds(&self, pos: IVec2) -> (IVec2, IVec2) {
        let world_size = IVec2::new(self.world.len() as i32, self.world[0].len() as i32);
        let bottom_left = (pos * self.chunk_size as i32 - IVec2::ONE).max(IVec2::ZERO);
        let top_right = ((pos + IVec2::ONE) * self.chunk_size as i32 + IVec2::ONE).min(world_size);
        (bottom_left, top_right)
    }
}
