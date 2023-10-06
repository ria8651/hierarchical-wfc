use crate::ui::RenderUpdateEvent;
use bevy::{prelude::*, utils::HashMap};
use grid_wfc::{
    graph_grid::{self, GridGraphSettings},
    world::{ChunkState, ChunkType, GenerationMode, World},
};
use hierarchical_wfc::{
    wfc_backend::{Backend, MultiThreaded, SingleThreaded},
    wfc_task::{BacktrackingSettings, Metadata},
    TileSet, WaveFunction, WfcTask,
};
use rand::{rngs::SmallRng, SeedableRng};
use std::sync::Arc;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GenerateEvent>()
            .init_resource::<Backends>()
            .init_resource::<MaybeWorld>()
            .add_systems(Update, (handle_events, handle_output).chain());
    }
}

#[derive(Event, Clone)]
pub enum GenerateEvent {
    Single {
        tileset: Arc<dyn TileSet>,
        settings: GridGraphSettings,
        backtracking: BacktrackingSettings,
        seed: u64,
    },
    Chunked {
        tileset: Arc<dyn TileSet>,
        settings: GridGraphSettings,
        backtracking: BacktrackingSettings,
        multithreaded: bool,
        deterministic: bool,
        seed: u64,
        chunk_size: usize,
        overlap: usize,
    },
}

#[derive(Resource)]
struct Backends {
    multithreaded: bool,
    single_threaded: SingleThreaded,
    multi_threaded: MultiThreaded,
}

impl Default for Backends {
    fn default() -> Self {
        let single_threaded = SingleThreaded::new();
        let multi_threaded = MultiThreaded::new(8);

        Self {
            multithreaded: false,
            single_threaded,
            multi_threaded,
        }
    }
}

enum TaskData {
    Single { size: IVec2 },
    Chunked { chunk: IVec2, chunk_type: ChunkType },
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct MaybeWorld(Option<World>);

fn handle_events(
    mut generate_event: EventReader<GenerateEvent>,
    mut backends: ResMut<Backends>,
    mut world: ResMut<MaybeWorld>,
) {
    for generate_event in generate_event.iter() {
        let generate_event = generate_event.clone();

        match generate_event {
            GenerateEvent::Chunked {
                tileset,
                settings,
                backtracking,
                multithreaded,
                deterministic,
                seed,
                chunk_size,
                overlap,
            } => {
                let filled = WaveFunction::filled(tileset.tile_count());
                let rng = SmallRng::seed_from_u64(seed);
                let mut new_world = World {
                    world: vec![vec![filled; settings.height]; settings.width],
                    generated_chunks: HashMap::new(),
                    chunk_size,
                    overlap,
                    tileset: tileset.clone(),
                    rng,
                    outstanding: 0,
                    backtracking: backtracking.clone(),
                };

                let generation_mode = match deterministic {
                    true => GenerationMode::Deterministic,
                    false => GenerationMode::NonDeterministic,
                };
                let start_chunks = new_world.start_generation(generation_mode);
                for (chunk, chunk_type) in start_chunks {
                    new_world
                        .generated_chunks
                        .insert(chunk, ChunkState::Scheduled);
                    let graph = new_world.extract_chunk(chunk);
                    let seed = seed + chunk.x as u64 * 1000 as u64 + chunk.y as u64;
                    let metadata: Metadata =
                        Some(Arc::new(TaskData::Chunked { chunk, chunk_type }));

                    let task = WfcTask {
                        graph,
                        tileset: new_world.tileset.clone(),
                        seed,
                        metadata,
                        backtracking: backtracking.clone(),
                    };

                    backends.multithreaded = multithreaded;
                    let backend: &mut dyn Backend = if multithreaded {
                        &mut backends.multi_threaded
                    } else {
                        &mut backends.single_threaded
                    };
                    new_world.outstanding += 1;
                    backend.queue_task(task).unwrap();
                }

                *world = MaybeWorld(Some(new_world));
            }
            GenerateEvent::Single {
                tileset,
                settings,
                backtracking,
                seed,
            } => {
                let graph =
                    graph_grid::create(&settings, WaveFunction::filled(tileset.tile_count()));
                let size = IVec2::new(settings.width as i32, settings.height as i32);
                let task = WfcTask {
                    graph,
                    tileset: tileset.clone(),
                    seed,
                    metadata: Some(Arc::new(TaskData::Single { size })),
                    backtracking: backtracking.clone(),
                };

                backends.multithreaded = false;
                backends.single_threaded.queue_task(task).unwrap();

                let rng = SmallRng::seed_from_u64(seed);
                let new_world = World {
                    world: vec![vec![WaveFunction::empty(); size.y as usize]; size.x as usize],
                    generated_chunks: HashMap::from_iter(vec![(IVec2::ZERO, ChunkState::Done)]),
                    chunk_size: 0,
                    overlap: 0,
                    tileset: tileset.clone(),
                    rng: rng.clone(),
                    outstanding: 0,
                    backtracking: backtracking.clone(),
                };
                *world = MaybeWorld(Some(new_world));
            }
        }
    }
}

fn handle_output(
    mut backends: ResMut<Backends>,
    mut world: ResMut<MaybeWorld>,
    mut render_world_event: EventWriter<RenderUpdateEvent>,
) {
    let backend: &mut dyn Backend = if backends.multithreaded {
        &mut backends.multi_threaded
    } else {
        &mut backends.single_threaded
    };

    while let Some((task, error)) = backend.check_output() {
        let world = world.as_mut().as_mut().unwrap();
        world.outstanding -= 1;
        
        if error.is_err() {
            error!("Error while generating world: {:?}", error);
        }

        let task_metadata = task.metadata.as_ref().unwrap().downcast_ref().unwrap();

        match task_metadata {
            TaskData::Chunked { chunk, chunk_type } => {
                world.merge_chunk(*chunk, task.graph);
                world.generated_chunks.insert(*chunk, ChunkState::Done);

                let ready = world.process_chunk(*chunk, *chunk_type);

                for (chunk, chunk_type) in ready {
                    world.generated_chunks.insert(chunk, ChunkState::Scheduled);
                    let graph = world.extract_chunk(chunk);
                    let seed = chunk.x as u64 * 1000 as u64 + chunk.y as u64;
                    let metadata: Metadata =
                        Some(Arc::new(TaskData::Chunked { chunk, chunk_type }));

                    let task = WfcTask {
                        graph,
                        tileset: world.tileset.clone(),
                        seed,
                        metadata,
                        backtracking: world.backtracking.clone(),
                    };

                    world.outstanding += 1;
                    backend.queue_task(task).unwrap();
                }

                render_world_event.send(RenderUpdateEvent);
            }
            TaskData::Single { size } => {
                // Note: Assumes that the graph is a grid graph with a standard ordering
                let graph = task.graph;
                let mut new_world =
                    vec![vec![WaveFunction::empty(); size.y as usize]; size.x as usize];
                for x in 0..size.x {
                    for y in 0..size.y {
                        new_world[x as usize][y as usize] =
                            graph.tiles[x as usize * size.y as usize + y as usize].clone();
                    }
                }

                world.world = new_world;
                render_world_event.send(RenderUpdateEvent);
            }
        }
    }
}
