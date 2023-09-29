use crate::ui::RenderUpdateEvent;
use bevy::{prelude::*, utils::HashMap};
use crossbeam::queue::SegQueue;
use grid_wfc::{
    graph_grid::{self, GridGraphSettings},
    world::{ChunkState, World},
};
use hierarchical_wfc::{wfc_backend, wfc_task, TileSet, WaveFunction, WfcTask};
use rand::{rngs::SmallRng, Rng, SeedableRng};
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
    single_threaded: wfc_backend::SingleThreaded,
    multi_threaded: wfc_backend::MultiThreaded,
    output: Arc<SegQueue<anyhow::Result<WfcTask>>>,
}

impl Default for Backends {
    fn default() -> Self {
        let output = Arc::new(SegQueue::new());
        let single_threaded = wfc_backend::SingleThreaded::new(output.clone());
        let multi_threaded = wfc_backend::MultiThreaded::new(output.clone(), 8);

        Self {
            single_threaded,
            multi_threaded,
            output,
        }
    }
}

enum TaskData {
    Single { size: IVec2 },
    Chunked { chunk: IVec2 },
    MultiThreaded { chunk: IVec2 },
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

        let multithreaded = matches!(generate_event, GenerateEvent::MultiThreaded { .. });
        let backend: &mut dyn wfc_backend::Backend = match generate_event {
            GenerateEvent::Chunked { .. } => &mut backends.single_threaded,
            GenerateEvent::MultiThreaded { .. } => &mut backends.multi_threaded,
            GenerateEvent::Single { .. } => &mut backends.single_threaded,
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
                let mut rng = SmallRng::seed_from_u64(seed);
                let chunks = IVec2::new(
                    settings.width as i32 / chunk_size as i32,
                    settings.height as i32 / chunk_size as i32,
                );
                let start_chunk =
                    IVec2::new(rng.gen_range(0..chunks.x), rng.gen_range(0..chunks.y));

                let filled = WaveFunction::filled(tileset.tile_count());
                let mut new_world = World {
                    world: vec![vec![filled; settings.height]; settings.width],
                    generated_chunks: HashMap::from_iter(vec![(
                        start_chunk,
                        ChunkState::Scheduled,
                    )]),
                    chunk_size,
                    overlap,
                    seed,
                    tileset: tileset.clone(),
                    outstanding: 0,
                };

                let user_data = if multithreaded {
                    TaskData::MultiThreaded { chunk: start_chunk }
                } else {
                    TaskData::Chunked { chunk: start_chunk }
                };

                new_world.start_generation(start_chunk, backend, Some(Box::new(user_data)));

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
                    metadata: Some(Box::new(TaskData::Single { size })),
                    backtracking: wfc_task::BacktrackingSettings::default(),
                };

                backend.queue_task(task).unwrap();

                let new_world = World {
                    world: vec![vec![WaveFunction::empty(); size.y as usize]; size.x as usize],
                    generated_chunks: HashMap::from_iter(vec![(IVec2::ZERO, ChunkState::Done)]),
                    chunk_size: 0,
                    overlap: 0,
                    seed,
                    tileset: tileset.clone(),
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

        let backend: &mut dyn wfc_backend::Backend = match task_metadata {
            TaskData::Chunked { .. } => &mut backends.multi_threaded,
            TaskData::MultiThreaded { .. } => &mut backends.multi_threaded,
            TaskData::Single { .. } => &mut backends.single_threaded,
        };

        match task_metadata {
            TaskData::Chunked { chunk } | TaskData::MultiThreaded { chunk } => {
                // println!("Chunk done: {:?}", chunk);

                let user_data: Box<dyn Fn(IVec2) -> wfc_task::Metadata> = match task_metadata {
                    TaskData::Chunked { .. } => {
                        Box::new(|chunk| Some(Box::new(TaskData::Chunked { chunk })))
                    }
                    TaskData::MultiThreaded { .. } => {
                        Box::new(|chunk| Some(Box::new(TaskData::MultiThreaded { chunk })))
                    }
                    _ => unreachable!(),
                };

                world
                    .as_mut()
                    .as_mut()
                    .unwrap()
                    .process_chunk(*chunk, task, backend, user_data);
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

                world.as_mut().as_mut().unwrap().world = new_world;
                render_world_event.send(RenderUpdateEvent);
            }
        }
    }
}
