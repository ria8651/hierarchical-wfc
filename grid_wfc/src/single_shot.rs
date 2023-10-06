use crate::{
    graph_grid::GridGraphSettings,
    world::{ChunkState, ChunkType, GenerationMode, World},
};
use bevy::{prelude::*, utils::HashMap};
use hierarchical_wfc::{
    wfc_backend::Backend,
    wfc_task::{BacktrackingSettings, Metadata},
    TileSet, WaveFunction, WfcTask,
};
use rand::{rngs::SmallRng, SeedableRng};
use std::sync::Arc;

struct TaskData {
    chunk: IVec2,
    chunk_type: ChunkType,
}

#[allow(dead_code)]
fn generate_world(
    tileset: Arc<dyn TileSet>,
    backend: &mut dyn Backend,
    settings: GridGraphSettings,
    seed: u64,
    generation_mode: GenerationMode,
    chunk_size: usize,
    overlap: usize,
) -> World {
    let filled = WaveFunction::filled(tileset.tile_count());
    let rng = SmallRng::seed_from_u64(seed);
    let mut world = World {
        world: vec![vec![filled; settings.height]; settings.width],
        generated_chunks: HashMap::new(),
        chunk_size,
        overlap,
        tileset,
        rng,
        outstanding: 0,
    };

    let start_chunks = world.start_generation(generation_mode);
    for (chunk, chunk_type) in start_chunks {
        world.generated_chunks.insert(chunk, ChunkState::Scheduled);
        let graph = world.extract_chunk(chunk);
        let seed = seed + chunk.x as u64 * 1000 as u64 + chunk.y as u64;
        let metadata: Metadata = Some(Arc::new(TaskData { chunk, chunk_type }));
        let tileset = world.tileset.clone();

        let task = WfcTask {
            graph,
            tileset,
            seed,
            metadata,
            backtracking: BacktrackingSettings::default(),
        };

        world.outstanding += 1;
        backend.queue_task(task).unwrap();
    }

    while world.outstanding > 0 {
        let task = backend.wait_for_output();
        world.outstanding -= 1;

        let task = match task {
            Ok(task) => task,
            Err(e) => {
                error!("Error: {:?}", e);
                continue;
            }
        };

        let task_metadata = task.metadata.as_ref().unwrap().downcast_ref().unwrap();
        match task_metadata {
            TaskData { chunk, chunk_type } => {
                world.merge_chunk(*chunk, task.graph);
                world.generated_chunks.insert(*chunk, ChunkState::Done);

                let ready = world.process_chunk(*chunk, *chunk_type);

                for (chunk, chunk_type) in ready {
                    world.generated_chunks.insert(chunk, ChunkState::Scheduled);
                    let graph = world.extract_chunk(chunk);
                    let seed = chunk.x as u64 * 1000 as u64 + chunk.y as u64;
                    let metadata: Metadata = Some(Arc::new(TaskData { chunk, chunk_type }));

                    let task = WfcTask {
                        graph,
                        tileset: world.tileset.clone(),
                        seed,
                        metadata,
                        backtracking: BacktrackingSettings::default(),
                    };

                    world.outstanding += 1;
                    backend.queue_task(task).unwrap();
                }
            }
        }
    }

    world
}
