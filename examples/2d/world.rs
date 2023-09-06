use crate::ui::RenderUpdateEvent;
use bevy::{prelude::*, utils::HashMap};
use crossbeam::queue::SegQueue;
use hierarchical_wfc::{
    CpuExecutor, Executor, MultiThreadedExecutor, Peasant, TileSet, UserData, WaveFunction,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::sync::Arc;
use utilities::{
    graph_grid::GridGraphSettings,
    world::{ChunkState, World},
};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GenerateEvent>()
            .init_resource::<Guild>()
            .init_resource::<World>()
            .add_systems(Update, (handle_events, handle_output));
    }
}

#[derive(Event, Clone)]
pub enum GenerateEvent {
    Single {
        tileset: Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
        settings: GridGraphSettings,
        weights: Arc<Vec<u32>>,
        seed: u64,
    },
    Chunked {
        tileset: Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
        settings: GridGraphSettings,
        weights: Arc<Vec<u32>>,
        seed: u64,
        chunk_size: usize,
    },
    MultiThreaded {
        tileset: Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
        settings: GridGraphSettings,
        weights: Arc<Vec<u32>>,
        seed: u64,
        chunk_size: usize,
    },
}

#[derive(Resource)]
struct Guild {
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
            cpu_executor,
            multithreaded_executor,
            output,
        }
    }
}

enum PeasantData {
    Single { size: IVec2 },
    Chunked { chunk: IVec2 },
    MultiThreaded { chunk: IVec2 },
}

fn handle_events(
    mut generate_event: EventReader<GenerateEvent>,
    mut guild: ResMut<Guild>,
    mut world: ResMut<World>,
) {
    for generate_event in generate_event.iter() {
        let generate_event = generate_event.clone();

        let multithreaded = matches!(generate_event, GenerateEvent::MultiThreaded { .. });
        let executor: &mut dyn Executor = match generate_event {
            GenerateEvent::Chunked { .. } => &mut guild.multithreaded_executor,
            GenerateEvent::MultiThreaded { .. } => &mut guild.multithreaded_executor,
            GenerateEvent::Single { .. } => &mut guild.cpu_executor,
        };

        match generate_event {
            GenerateEvent::Chunked {
                tileset,
                settings,
                weights,
                seed,
                chunk_size,
            }
            | GenerateEvent::MultiThreaded {
                tileset,
                settings,
                weights,
                seed,
                chunk_size,
            } => {
                let constraints = Arc::new(tileset.get_constraints());
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
                    seed,
                    current_constraints: constraints.clone(),
                    current_weights: weights.clone(),
                };

                let user_data = if multithreaded {
                    PeasantData::MultiThreaded { chunk: start_chunk }
                } else {
                    PeasantData::Chunked { chunk: start_chunk }
                };

                new_world.start_generation(start_chunk, executor, Some(Box::new(user_data)));

                *world = new_world;
            }
            GenerateEvent::Single {
                tileset,
                settings,
                weights,
                seed,
            } => {
                let graph = tileset.create_graph(&settings);
                let constraints = Arc::new(tileset.get_constraints());
                let size = IVec2::new(settings.width as i32, settings.height as i32);
                let peasant = Peasant {
                    graph,
                    constraints,
                    weights,
                    seed,
                    user_data: Some(Box::new(PeasantData::Single { size })),
                };

                executor.queue_peasant(peasant).unwrap();
            }
        }
    }
}

fn handle_output(
    mut guild: ResMut<Guild>,
    mut world: ResMut<World>,
    mut render_world_event: EventWriter<RenderUpdateEvent>,
) {
    while let Some(peasant) = guild.output.pop() {
        let peasant_data = peasant.user_data.as_ref().unwrap().downcast_ref().unwrap();

        let executor: &mut dyn Executor = match peasant_data {
            PeasantData::Chunked { .. } => &mut guild.multithreaded_executor,
            PeasantData::MultiThreaded { .. } => &mut guild.multithreaded_executor,
            PeasantData::Single { .. } => &mut guild.cpu_executor,
        };

        match peasant_data {
            PeasantData::Chunked { chunk } | PeasantData::MultiThreaded { chunk } => {
                // println!("Chunk done: {:?}", chunk);

                let user_data: Box<dyn Fn(IVec2) -> UserData> = match peasant_data {
                    PeasantData::Chunked { .. } => {
                        Box::new(|chunk| Some(Box::new(PeasantData::Chunked { chunk })))
                    }
                    PeasantData::MultiThreaded { .. } => {
                        Box::new(|chunk| Some(Box::new(PeasantData::MultiThreaded { chunk })))
                    }
                    _ => unreachable!(),
                };

                world.process_chunk(*chunk, peasant, executor, user_data);
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
                            graph.tiles[x as usize * size.y as usize + y as usize].clone();
                    }
                }

                world.world = new_world;
                render_world_event.send(RenderUpdateEvent);
            }
        }
    }
}
