use crate::ui::RenderUpdateEvent;
use anyhow::Result;
use bevy::{prelude::*, utils::HashMap};
use crossbeam::queue::SegQueue;
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
        seed: u64,
    },
    Chunked {
        tileset: Arc<dyn TileSet>,
        settings: GridGraphSettings,
        seed: u64,
        chunk_size: usize,
        overlap: usize,
    },
    MultiThreaded {
        tileset: Arc<dyn TileSet>,
        settings: GridGraphSettings,
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
    output: Arc<SegQueue<Result<WfcTask>>>,
}

impl Default for Backends {
    fn default() -> Self {
        let output = Arc::new(SegQueue::new());
        let single_threaded = SingleThreaded::new(output.clone());
        let multi_threaded = MultiThreaded::new(output.clone(), 8);

        Self {
            multithreaded: false,
            single_threaded,
            multi_threaded,
            output,
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

        backends.multithreaded = matches!(generate_event, GenerateEvent::MultiThreaded { .. });
        let backend: &mut dyn Backend = if backends.multithreaded {
            &mut backends.multi_threaded
        } else {
            &mut backends.single_threaded
        };

        match generate_event {
            GenerateEvent::Chunked {
                tileset,
                settings,
                seed,
                chunk_size,
                overlap,
            }
            | GenerateEvent::MultiThreaded {
                tileset,
                settings,
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
                };

                let start_chunks = new_world.start_generation(GenerationMode::Deterministic);
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
                        backtracking: BacktrackingSettings::default(),
                    };

                    backend.queue_task(task).unwrap();
                }

                *world = MaybeWorld(Some(new_world));
            }
            GenerateEvent::Single {
                tileset,
                settings,
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
                    backtracking: BacktrackingSettings::default(),
                };

                backend.queue_task(task).unwrap();

                let rng = SmallRng::seed_from_u64(seed);
                let new_world = World {
                    world: vec![vec![WaveFunction::empty(); size.y as usize]; size.x as usize],
                    generated_chunks: HashMap::from_iter(vec![(IVec2::ZERO, ChunkState::Done)]),
                    chunk_size: 0,
                    overlap: 0,
                    tileset: tileset.clone(),
                    rng: rng.clone(),
                    outstanding: 0,
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
    while let Some(Ok(task)) = backends.output.pop() {
        let task_metadata = task.metadata.as_ref().unwrap().downcast_ref().unwrap();

        let backend: &mut dyn Backend = if backends.multithreaded {
            &mut backends.multi_threaded
        } else {
            &mut backends.single_threaded
        };

        let world = world.as_mut().as_mut().unwrap();

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
                        backtracking: BacktrackingSettings::default(),
                    };

                    backend.queue_task(task).unwrap();
                }

                render_world_event.send(RenderUpdateEvent);
            }
            TaskData::Single { size } => {
                // println!("Single done");

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
