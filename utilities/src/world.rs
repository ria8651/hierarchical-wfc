use crate::graph_grid::{self, Direction, GridGraphSettings};
use bevy::{prelude::*, utils::HashMap};
use hierarchical_wfc::{Executor, Graph, Peasant, TileSet, UserData, WaveFunction};
use std::sync::Arc;

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
    pub tileset: Arc<dyn TileSet>,
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

    pub fn start_generation(
        &mut self,
        start_chunk: IVec2,
        executor: &mut dyn Executor,
        user_data: UserData,
    ) {
        let graph = self.extract_chunk(start_chunk);
        let peasant = Peasant {
            graph,
            tileset: self.tileset.clone(),
            seed: self.seed,
            user_data,
        };

        executor.queue_peasant(peasant).unwrap();
    }

    pub fn process_chunk(
        &mut self,
        chunk: IVec2,
        peasant: Peasant,
        executor: &mut dyn Executor,
        user_data: Box<dyn Fn(IVec2) -> UserData>,
    ) {
        self.merge_chunk(chunk, peasant.graph);
        self.generated_chunks.insert(chunk, ChunkState::Done);

        // queue neighbors
        'outer: for direction in 0..4 {
            let neighbor = chunk + Direction::from(direction).to_ivec2();
            let chunks = IVec2::new(
                self.world.len() as i32 / self.chunk_size as i32,
                self.world[0].len() as i32 / self.chunk_size as i32,
            );
            if !self.generated_chunks.contains_key(&neighbor)
                && neighbor.cmpge(IVec2::ZERO).all()
                && neighbor.cmplt(chunks).all()
            {
                // check if neighbor's neighbors are done
                for direction in 0..4 {
                    let neighbor = neighbor + Direction::from(direction).to_ivec2();
                    if let Some(state) = self.generated_chunks.get(&neighbor) {
                        if *state == ChunkState::Scheduled {
                            continue 'outer;
                        }
                    }
                }

                self.generated_chunks
                    .insert(neighbor, ChunkState::Scheduled);
                let graph = self.extract_chunk(neighbor);
                let seed = self.seed + neighbor.x as u64 * chunks.y as u64 + neighbor.y as u64;

                let peasant = Peasant {
                    graph,
                    tileset: self.tileset.clone(),
                    seed,
                    user_data: user_data(neighbor),
                };

                executor.queue_peasant(peasant).unwrap();
            }
        }
    }
}
