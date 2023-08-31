use crate::{
    graph_grid::{self, GridGraphSettings},
    ui::RenderUpdateEvent,
};
use bevy::prelude::*;
use crossbeam::queue::SegQueue;
use hierarchical_wfc::{CpuExecuter, Executer, Graph, Peasant, TileSet, WaveFunction};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::sync::Arc;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GenerateEvent>()
            .init_resource::<Guild>()
            .init_resource::<World>()
            .add_systems(Update, (handle_events, handle_output));
    }
}

#[derive(Resource)]
pub struct World {
    pub world: Vec<Vec<WaveFunction>>,
    chunk_size: usize,
    seed: u64,
}

impl Default for World {
    fn default() -> Self {
        Self {
            world: Vec::new(),
            chunk_size: 0,
            seed: 0,
        }
    }
}

#[derive(Event, Clone)]
pub enum GenerateEvent {
    Single {
        tileset: Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
        settings: GridGraphSettings,
        weights: Vec<u32>,
        seed: u64,
    },
    Chunked {
        tileset: Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
        settings: GridGraphSettings,
        weights: Vec<u32>,
        seed: u64,
        chunk_size: usize,
    },
}

#[derive(Resource)]
struct Guild {
    cpu_executer: CpuExecuter,
    output: Arc<SegQueue<Peasant>>,
}

impl Default for Guild {
    fn default() -> Self {
        let output = Arc::new(SegQueue::new());
        let cpu_executer = CpuExecuter::new(output.clone());

        Self {
            cpu_executer,
            output,
        }
    }
}

enum PeasantData {
    Single { size: IVec2 },
    Chunked { chunk: IVec2 },
}

fn handle_events(
    mut generate_event: EventReader<GenerateEvent>,
    mut guild: ResMut<Guild>,
    mut world: ResMut<World>,
) {
    for generate_event in generate_event.iter() {
        let generate_event = generate_event.clone();
        match generate_event {
            GenerateEvent::Chunked {
                tileset,
                settings,
                weights,
                seed,
                chunk_size,
            } => {
                world.world = vec![vec![WaveFunction::empty(); settings.height]; settings.width];

                let mut rng = SmallRng::seed_from_u64(seed);
                let chunks = IVec2::new(
                    settings.width as i32 / chunk_size as i32,
                    settings.height as i32 / chunk_size as i32,
                );
                let start_chunk =
                    IVec2::new(rng.gen_range(0..chunks.x), rng.gen_range(0..chunks.y));

                let graph = world.extract_chunk(start_chunk);
                let constraints = Arc::new(tileset.get_constraints());
                let peasant = Peasant {
                    graph,
                    constraints,
                    weights,
                    seed,
                    user_data: Some(Box::new(PeasantData::Chunked { chunk: start_chunk })),
                };

                guild.cpu_executer.queue_peasant(peasant).unwrap();
                world.seed = seed;
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

                guild.cpu_executer.queue_peasant(peasant).unwrap();
            }
        }
    }
}

fn handle_output(
    guild: Res<Guild>,
    mut world: ResMut<World>,
    mut render_world_event: EventWriter<RenderUpdateEvent>,
) {
    while let Some(peasant) = guild.output.pop() {
        match *peasant
            .user_data
            .unwrap()
            .downcast::<PeasantData>()
            .unwrap()
        {
            PeasantData::Chunked { chunk } => {
                println!("Chunk done: {:?}", chunk);
            }
            PeasantData::Single { size } => {
                println!("Single done");

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

impl World {
    fn extract_chunk(&self, pos: IVec2) -> Graph<WaveFunction> {
        let bottom_left = (pos * self.chunk_size as i32 - IVec2::ONE).max(IVec2::ZERO);
        let top_right = (pos * self.chunk_size as i32 + IVec2::ONE).min(IVec2::new(
            self.world.len() as i32,
            self.world[0].len() as i32,
        ));
        let size = top_right - bottom_left;

        let settings = GridGraphSettings {
            width: size.x as usize,
            height: size.y as usize,
            periodic: false,
        };
        let mut graph = graph_grid::create(&settings, WaveFunction::empty());

        for x in 0..size.x {
            for y in 0..size.y {
                let tile = &self.world[(bottom_left.x + x) as usize][(bottom_left.y + y) as usize];
                graph.tiles[x as usize * size.y as usize + y as usize] = tile.clone();
            }
        }

        graph
    }
}
