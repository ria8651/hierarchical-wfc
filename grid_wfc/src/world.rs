use crate::graph_grid::{self, Direction, GridGraphSettings};
use bevy::{prelude::*, utils::HashMap};
use hierarchical_wfc::{wfc_backend, wfc_task, Graph, TileSet, WaveFunction, WfcTask};
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
    pub overlap: usize,
    pub seed: u64,
    pub tileset: Arc<dyn TileSet>,
    pub outstanding: usize,
}

impl World {
    pub fn extract_chunk(&self, chunk: IVec2) -> Graph<WaveFunction> {
        let (bottom_left, top_right) = self.chunk_bounds(chunk);
        let size = top_right - bottom_left;

        let settings = GridGraphSettings {
            width: size.x as usize,
            height: size.y as usize,
            periodic: false,
        };
        let filled = WaveFunction::filled(self.tileset.tile_count());
        let mut graph = graph_grid::create(&settings, filled);

        let chunk_bottom_left = chunk * self.chunk_size as i32;
        let chunk_top_right = (chunk + IVec2::ONE) * self.chunk_size as i32;
        for x in 0..size.x {
            for y in 0..size.y {
                let pos = IVec2::new(bottom_left.x + x, bottom_left.y + y);
                if pos.cmplt(chunk_bottom_left).any() || pos.cmpge(chunk_top_right).any() {
                    let tile = &self.world[pos.x as usize][pos.y as usize];
                    graph.tiles[x as usize * size.y as usize + y as usize] = tile.clone();
                }
            }
        }

        // for y in (0..size.y).rev() {
        //     for x in 0..size.x {
        //         print!(
        //             "{:>5}",
        //             graph.tiles[x as usize * size.y as usize + y as usize].count_bits()
        //         );
        //     }
        //     println!();
        // }
        // println!();

        graph
    }

    pub fn merge_chunk(&mut self, chunk: IVec2, graph: Graph<WaveFunction>) {
        let (bottom_left, top_right) = self.chunk_bounds(chunk);
        let size = top_right - bottom_left;

        let chunk_bottom_left = chunk * self.chunk_size as i32;
        let chunk_top_right = (chunk + IVec2::ONE) * self.chunk_size as i32;

        // Note: Assumes that the graph is a grid graph with a standard ordering
        for x in 0..size.x {
            for y in 0..size.y {
                let pos = IVec2::new(bottom_left.x + x, bottom_left.y + y);

                // overwrite tiles inside the chunk while preserving tiles on the border
                if (pos.cmpge(chunk_bottom_left).all() && pos.cmplt(chunk_top_right).all())
                    || self.world[pos.x as usize][pos.y as usize].count_bits() > 1
                {
                    let tile = graph.tiles[x as usize * size.y as usize + y as usize].clone();
                    self.world[pos.x as usize][pos.y as usize] = tile;
                }
            }
        }
    }

    pub fn chunk_bounds(&self, pos: IVec2) -> (IVec2, IVec2) {
        let world_size = IVec2::new(self.world.len() as i32, self.world[0].len() as i32);
        let bottom_left =
            (pos * self.chunk_size as i32 - IVec2::splat(self.overlap as i32)).max(IVec2::ZERO);
        let top_right = ((pos + IVec2::ONE) * self.chunk_size as i32
            + IVec2::splat(self.overlap as i32))
        .min(world_size);
        (bottom_left, top_right)
    }

    pub fn start_generation(
        &mut self,
        start_chunk: IVec2,
        backend: &mut dyn wfc_backend::Backend,
        user_data: wfc_task::Metadata,
    ) {
        let graph = self.extract_chunk(start_chunk);
        let task = WfcTask {
            graph,
            tileset: self.tileset.clone(),
            seed: self.seed,
            metadata: user_data,
            backtracking: wfc_task::BacktrackingSettings::default(),
        };

        self.outstanding += 1;
        backend.queue_task(task).unwrap();
    }

    pub fn process_chunk(
        &mut self,
        chunk: IVec2,
        task: WfcTask,
        backend: &mut dyn wfc_backend::Backend,
        user_data: Box<dyn Fn(IVec2) -> wfc_task::Metadata>,
    ) {
        self.outstanding -= 1;
        self.merge_chunk(chunk, task.graph);
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

                let task = WfcTask {
                    graph,
                    tileset: self.tileset.clone(),
                    seed,
                    metadata: user_data(neighbor),
                    backtracking: wfc_task::BacktrackingSettings::default(),
                };
                self.outstanding += 1;
                backend.queue_task(task).unwrap();
            }
        }
    }
}
