use crate::ui::RenderUpdateEvent;
use bevy::{prelude::*, utils::HashMap};
use crossbeam::queue::SegQueue;
use grid_wfc::{
    graph_grid::{self, GridGraphSettings},
    world::{ChunkState, ChunkType, GenerationMode, World},
};
use hierarchical_wfc::{
    CpuExecutor, Executor, MultiThreadedExecutor, Peasant, TileSet, UserData, WaveFunction,
};
use rand::{rngs::SmallRng, SeedableRng};
use std::sync::Arc;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GenerateEvent>()
            .init_resource::<Guild>()
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
struct Guild {
    multithreaded: bool,
    cpu_executor: CpuExecutor,
    multithreaded_executor: MultiThreadedExecutor,
    output: Arc<SegQueue<Peasant>>,
}

impl Default for Guild {
    fn default() -> Self {
        let output = Arc::new(SegQueue::new());
        let cpu_executor = CpuExecutor::new(output.clone());
        let multithreaded_executor = MultiThreadedExecutor::new(output.clone(), 8);

        Self {
            multithreaded: false,
            cpu_executor,
            multithreaded_executor,
            output,
        }
    }
}

enum PeasantData {
    Single { size: IVec2 },
    Chunked { chunk: IVec2, chunk_type: ChunkType },
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct MaybeWorld(Option<World>);

fn handle_events(
    mut generate_event: EventReader<GenerateEvent>,
    mut guild: ResMut<Guild>,
    mut world: ResMut<MaybeWorld>,
) {
    for generate_event in generate_event.iter() {
        let generate_event = generate_event.clone();

        guild.multithreaded = matches!(generate_event, GenerateEvent::MultiThreaded { .. });
        let executor: &mut dyn Executor = if guild.multithreaded {
            &mut guild.multithreaded_executor
        } else {
            &mut guild.cpu_executor
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
                    tileset,
                    rng,
                };

                let start_chunks = new_world.start_generation(GenerationMode::Deterministic);
                for (chunk, chunk_type) in start_chunks {
                    new_world
                        .generated_chunks
                        .insert(chunk, ChunkState::Scheduled);
                    let graph = new_world.extract_chunk(chunk);
                    let seed = seed + chunk.x as u64 * 1000 as u64 + chunk.y as u64;
                    let user_data: UserData =
                        Some(Arc::new(PeasantData::Chunked { chunk, chunk_type }));

                    let peasant = Peasant {
                        graph,
                        tileset: new_world.tileset.clone(),
                        seed,
                        user_data,
                    };

                    executor.queue_peasant(peasant).unwrap();
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
                let peasant = Peasant {
                    graph,
                    tileset: tileset.clone(),
                    seed,
                    user_data: Some(Arc::new(PeasantData::Single { size })),
                };

                executor.queue_peasant(peasant).unwrap();

                let rng = SmallRng::seed_from_u64(seed);
                let new_world = World {
                    world: vec![vec![WaveFunction::empty(); size.y as usize]; size.x as usize],
                    generated_chunks: HashMap::from_iter(vec![(IVec2::ZERO, ChunkState::Done)]),
                    chunk_size: 0,
                    overlap: 0,
                    tileset: tileset.clone(),
                    rng: rng.clone(),
                };
                *world = MaybeWorld(Some(new_world));
            }
        }
    }
}

fn handle_output(
    mut guild: ResMut<Guild>,
    mut world: ResMut<MaybeWorld>,
    mut render_world_event: EventWriter<RenderUpdateEvent>,
) {
    while let Some(peasant) = guild.output.pop() {
        let peasant_data = peasant.user_data.as_ref().unwrap().downcast_ref().unwrap();

        let executor: &mut dyn Executor = if guild.multithreaded {
            &mut guild.multithreaded_executor
        } else {
            &mut guild.cpu_executor
        };

        let world = world.as_mut().as_mut().unwrap();

        match peasant_data {
            PeasantData::Chunked { chunk, chunk_type } => {
                world.merge_chunk(*chunk, peasant.graph);
                world.generated_chunks.insert(*chunk, ChunkState::Done);

                let ready = world.process_chunk(*chunk, *chunk_type);

                for (chunk, chunk_type) in ready {
                    world.generated_chunks.insert(chunk, ChunkState::Scheduled);
                    let graph = world.extract_chunk(chunk);
                    let seed = chunk.x as u64 * 1000 as u64 + chunk.y as u64;
                    let user_data: UserData =
                        Some(Arc::new(PeasantData::Chunked { chunk, chunk_type }));

                    let peasant = Peasant {
                        graph,
                        tileset: world.tileset.clone(),
                        seed,
                        user_data,
                    };

                    executor.queue_peasant(peasant).unwrap();
                }

                render_world_event.send(RenderUpdateEvent);
            }
            PeasantData::Single { size } => {
                // println!("Single done");

                // Note: Assumes that the graph is a grid graph with a standard ordering
                let graph = peasant.graph;
                let mut new_world =
                    vec![vec![WaveFunction::empty(); size.y as usize]; size.x as usize];
                for x in 0..size.x {
                    for y in 0..size.y {
                        new_world[x as usize][y as usize] =
                            graph.tiles[x as usize * size.y as usize + y as usize];
                    }
                }

                world.world = new_world;
                render_world_event.send(RenderUpdateEvent);
            }
        }
    }
}
