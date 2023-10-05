use crate::graph_grid::{self, Direction, GridGraphSettings};
use bevy::{prelude::*, utils::HashMap};
use hierarchical_wfc::{Graph, TileSet, WaveFunction};
use rand::{rngs::SmallRng, Rng};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkState {
    Scheduled,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenerationMode {
    NonDeterministic,
    Deterministic,
}

#[derive(Resource)]
pub struct World {
    pub world: Vec<Vec<WaveFunction>>,
    pub generated_chunks: HashMap<IVec2, ChunkState>,
    pub chunk_size: usize,
    pub overlap: usize,
    pub tileset: Arc<dyn TileSet>,
    pub rng: SmallRng,
    pub outstanding: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkType {
    NonDeterministic,
    Corner,
    Edge,
    Center,
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

    pub fn start_generation(&mut self, generation_mode: GenerationMode) -> Vec<(IVec2, ChunkType)> {
        let mut start_chunks = Vec::new();
        match generation_mode {
            GenerationMode::NonDeterministic => {
                let chunks = IVec2::new(
                    self.world.len() as i32 / self.chunk_size as i32,
                    self.world[0].len() as i32 / self.chunk_size as i32,
                );
                let start_chunk = IVec2::new(
                    self.rng.gen_range(0..chunks.x),
                    self.rng.gen_range(0..chunks.y),
                );

                start_chunks.push((start_chunk, ChunkType::NonDeterministic));
            }
            GenerationMode::Deterministic => {
                let chunks = IVec2::new(
                    self.world.len() as i32 / self.chunk_size as i32,
                    self.world[0].len() as i32 / self.chunk_size as i32,
                );
                let half_chunks = chunks / 2;
                for x in 0..half_chunks.x {
                    for y in 0..half_chunks.y {
                        let chunk = IVec2::new(x, y) * 2;

                        start_chunks.push((chunk, ChunkType::Corner));
                    }
                }
            }
        }
        start_chunks
    }

    // returns chunks that are able to be processed
    pub fn process_chunk(
        &mut self,
        chunk: IVec2,
        chunk_type: ChunkType,
    ) -> Vec<(IVec2, ChunkType)> {
        let mut ready_chunks = Vec::new();

        match chunk_type {
            ChunkType::NonDeterministic => {
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

                        ready_chunks.push((neighbor, ChunkType::NonDeterministic));
                    }
                }
            }
            ChunkType::Corner => {
                let chunks = IVec2::new(
                    self.world.len() as i32 / self.chunk_size as i32,
                    self.world[0].len() as i32 / self.chunk_size as i32,
                );

                for direction in 0..4 {
                    let next_corner = chunk + 2 * Direction::from(direction).to_ivec2();
                    let edge = chunk + Direction::from(direction).to_ivec2();

                    // check if next corner is in bounds
                    if next_corner.cmplt(IVec2::ZERO).any() || next_corner.cmpge(chunks).any() {
                        ready_chunks.push((edge, ChunkType::Edge));
                        continue;
                    }

                    // check if next corner is done
                    if let Some(state) = self.generated_chunks.get(&next_corner) {
                        if *state == ChunkState::Done {
                            ready_chunks.push((edge, ChunkType::Edge));
                        }
                    }
                }
            }
            ChunkType::Edge => {
                let chunks = IVec2::new(
                    self.world.len() as i32 / self.chunk_size as i32,
                    self.world[0].len() as i32 / self.chunk_size as i32,
                );

                for direction in 0..4 {
                    let center = chunk + Direction::from(direction).to_ivec2();
                    if self.generated_chunks.contains_key(&center)
                        || center.cmplt(IVec2::ZERO).any()
                        || center.cmpge(chunks).any()
                    {
                        continue;
                    }

                    let mut good = 0;
                    for direction in 0..4 {
                        let edge = center + Direction::from(direction).to_ivec2();
                        if let Some(state) = self.generated_chunks.get(&edge) {
                            if *state == ChunkState::Done {
                                good += 1;
                                continue;
                            }
                        }

                        if edge.cmplt(IVec2::ZERO).any() || edge.cmpge(chunks).any() {
                            good += 1;
                            continue;
                        }
                    }

                    if good == 4 {
                        ready_chunks.push((center, ChunkType::Center));
                    }
                }
            }
            _ => {
                // TODO: Implement
            }
        }

        ready_chunks
    }
}
