use crate::{
    grid_graph::GridGraphSettings,
    world::{ChunkState, ChunkType, GenerationMode, World, ChunkSettings},
};
use bevy::{prelude::*, utils::HashMap};
use core_wfc::{
    wfc_backend::Backend,
    wfc_task::{Metadata, WfcSettings},
    TileSet, WaveFunction, WfcTask,
};
use rand::{rngs::SmallRng, SeedableRng};
use std::sync::Arc;

struct TaskData {
    chunk: IVec2,
    chunk_type: ChunkType,
}

#[allow(dead_code)]
pub fn generate_world(
    tileset: Arc<dyn TileSet>,
    backend: &mut dyn Backend,
    settings: GridGraphSettings,
    seed: u64,
    generation_mode: GenerationMode,
    chunk_settings: ChunkSettings,
    wfc_settings: WfcSettings,
) -> (World, anyhow::Result<()>) {
    let filled = WaveFunction::filled(tileset.tile_count());
    let rng = SmallRng::seed_from_u64(seed);
    let mut world = World {
        world: vec![vec![filled; settings.height]; settings.width],
        generated_chunks: HashMap::new(),
        chunk_settings,
        tileset,
        rng,
        outstanding: 0,
        settings: wfc_settings.clone(),
    };

    let start_chunks = world.start_generation(generation_mode);
    for (chunk, chunk_type) in start_chunks {
        world.generated_chunks.insert(chunk, ChunkState::Scheduled);
        let graph = world.extract_chunk(chunk);
        let seed = seed + chunk.x as u64 * 1000_u64 + chunk.y as u64;
        let metadata: Metadata = Some(Arc::new(TaskData { chunk, chunk_type }));
        let tileset = world.tileset.clone();

        let task = WfcTask {
            graph,
            tileset,
            seed,
            metadata,
            settings: wfc_settings.clone(),
        };

        world.outstanding += 1;
        backend.queue_task(task).unwrap();
    }

    let mut failed = false;
    while world.outstanding > 0 {
        let (task, error) = backend.wait_for_output();
        world.outstanding -= 1;

        if failed {
            continue;
        }

        let task_metadata = task.metadata.as_ref().unwrap().downcast_ref().unwrap();
        let TaskData { chunk, chunk_type } = *task_metadata;

        world.merge_chunk(chunk, task.graph);
        world.generated_chunks.insert(chunk, ChunkState::Done);

        if error.is_err() {
            error!("Failed to generate chunk {:?}: {:?}", chunk, error);

            world.generated_chunks.insert(chunk, ChunkState::Failed);
            failed = true;
            continue;
        }

        let ready = world.process_chunk(chunk, chunk_type);
        for (chunk, chunk_type) in ready {
            world.generated_chunks.insert(chunk, ChunkState::Scheduled);
            let graph = world.extract_chunk(chunk);
            let seed = chunk.x as u64 * 1000_u64 + chunk.y as u64;
            let metadata: Metadata = Some(Arc::new(TaskData { chunk, chunk_type }));

            let task = WfcTask {
                graph,
                tileset: world.tileset.clone(),
                seed,
                metadata,
                settings: WfcSettings::default(),
            };

            world.outstanding += 1;
            backend.queue_task(task).unwrap();
        }
    }

    if failed {
        (world, Err(anyhow::anyhow!("Failed to generate world")))
    } else {
        (world, Ok(()))
    }
}
