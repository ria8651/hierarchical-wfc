use crate::ui::RenderUpdateEvent;
use bevy::{prelude::*, utils::HashMap};
use grid_wfc::{
    graph_grid::{self, GridGraphSettings},
    world::{ChunkSettings, ChunkState, ChunkType, GenerationMode, World},
};
use hierarchical_wfc::{
    wfc_backend::{Backend, MultiThreaded, SingleThreaded},
    wfc_task::{Metadata, WfcSettings},
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
            .init_resource::<Failed>()
            .add_systems(Update, (handle_events, handle_output).chain());
    }
}

#[derive(Event, Clone)]
pub enum GenerateEvent {
    Single {
        tileset: Arc<dyn TileSet>,
        settings: GridGraphSettings,
        wfc_settings: WfcSettings,
        seed: u64,
    },
    Chunked {
        tileset: Arc<dyn TileSet>,
        settings: GridGraphSettings,
        wfc_settings: WfcSettings,
        chunk_settings: ChunkSettings,
        multithreaded: bool,
        deterministic: bool,
        seed: u64,
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

#[derive(Resource, Deref, DerefMut, Default)]
struct Failed(bool);

fn handle_events(
    mut generate_event: EventReader<GenerateEvent>,
    mut backends: ResMut<Backends>,
    mut world: ResMut<MaybeWorld>,
    mut failed: ResMut<Failed>,
) {
    for generate_event in generate_event.iter() {
        let generate_event = generate_event.clone();

        failed.0 = false;

        match generate_event {
            GenerateEvent::Chunked {
                tileset,
                settings,
                wfc_settings,
                multithreaded,
                deterministic,
                seed,
                chunk_settings,
            } => {
                let filled = WaveFunction::filled(tileset.tile_count());
                let rng = SmallRng::seed_from_u64(seed);
                let mut new_world = World {
                    world: vec![vec![filled; settings.height]; settings.width],
                    generated_chunks: HashMap::new(),
                    chunk_settings,
                    tileset: tileset.clone(),
                    rng,
                    outstanding: 0,
                    settings: wfc_settings.clone(),
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
                    let seed = seed + chunk.x as u64 * 1000 + chunk.y as u64;
                    let metadata: Metadata =
                        Some(Arc::new(TaskData::Chunked { chunk, chunk_type }));

                    let task = WfcTask {
                        graph,
                        tileset: new_world.tileset.clone(),
                        seed,
                        metadata,
                        settings: wfc_settings.clone(),
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
                wfc_settings,
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
                    settings: wfc_settings.clone(),
                };

                backends.multithreaded = false;
                backends.single_threaded.queue_task(task).unwrap();

                let rng = SmallRng::seed_from_u64(seed);
                let new_world = World {
                    world: vec![vec![WaveFunction::empty(); size.y as usize]; size.x as usize],
                    generated_chunks: HashMap::from_iter(vec![(IVec2::ZERO, ChunkState::Done)]),
                    chunk_settings: ChunkSettings::default(),
                    tileset: tileset.clone(),
                    rng: rng.clone(),
                    outstanding: 0,
                    settings: wfc_settings.clone(),
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
    mut failed: ResMut<Failed>,
) {
    let backend: &mut dyn Backend = if backends.multithreaded {
        &mut backends.multi_threaded
    } else {
        &mut backends.single_threaded
    };

    while let Some((task, error)) = backend.get_output() {
        let world = world.as_mut().as_mut().unwrap();
        world.outstanding -= 1;

        if failed.0 {
            continue;
        }

        match task.metadata.as_ref().unwrap().downcast_ref().unwrap() {
            TaskData::Chunked { chunk, chunk_type } => {
                world.merge_chunk(*chunk, task.graph);
                if error.is_err() {
                    error!("Error while generating world: {:?}", error);

                    failed.0 = true;
                    world.generated_chunks.insert(*chunk, ChunkState::Failed);

                    render_world_event.send(RenderUpdateEvent);

                    continue;
                }

                world.generated_chunks.insert(*chunk, ChunkState::Done);

                let ready = world.process_chunk(*chunk, *chunk_type);

                for (chunk, chunk_type) in ready {
                    world.generated_chunks.insert(chunk, ChunkState::Scheduled);
                    let graph = world.extract_chunk(chunk);
                    let seed = chunk.x as u64 * 1000 + chunk.y as u64;
                    let metadata: Metadata =
                        Some(Arc::new(TaskData::Chunked { chunk, chunk_type }));

                    let task = WfcTask {
                        graph,
                        tileset: world.tileset.clone(),
                        seed,
                        metadata,
                        settings: world.settings.clone(),
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
