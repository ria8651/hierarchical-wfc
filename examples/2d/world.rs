use crate::{
    graph_grid::{self, Direction, GridGraphSettings},
    ui::RenderUpdateEvent,
};
use bevy::{prelude::*, utils::HashMap};
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ChunkState {
    Scheduled,
    Done,
}

#[derive(Resource)]
pub struct World {
    pub world: Vec<Vec<WaveFunction>>,
    generated_chunks: HashMap<IVec2, ChunkState>,
    chunk_size: usize,
    seed: u64,
    current_constraints: Arc<Vec<Vec<WaveFunction>>>,
    current_weights: Arc<Vec<u32>>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            world: Vec::new(),
            generated_chunks: HashMap::new(),
            chunk_size: 0,
            seed: 0,
            current_constraints: Arc::new(Vec::new()),
            current_weights: Arc::new(Vec::new()),
        }
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
                let constraints = Arc::new(tileset.get_constraints());
                let mut rng = SmallRng::seed_from_u64(seed);
                let chunks = IVec2::new(
                    settings.width as i32 / chunk_size as i32,
                    settings.height as i32 / chunk_size as i32,
                );
                let start_chunk =
                    IVec2::new(rng.gen_range(0..chunks.x), rng.gen_range(0..chunks.y));

                let filled = WaveFunction::filled(tileset.tile_count());
                let new_world = World {
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

                let graph = new_world.extract_chunk(start_chunk);
                let peasant = Peasant {
                    graph,
                    constraints: constraints.clone(),
                    weights: weights.clone(),
                    seed,
                    user_data: Some(Box::new(PeasantData::Chunked { chunk: start_chunk })),
                };
                guild.cpu_executer.queue_peasant(peasant).unwrap();

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

                guild.cpu_executer.queue_peasant(peasant).unwrap();
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
        match *peasant
            .user_data
            .unwrap()
            .downcast::<PeasantData>()
            .unwrap()
        {
            PeasantData::Chunked { chunk } => {
                // println!("Chunk done: {:?}", chunk);

                let (bottom_left, top_right) = world.chunk_bounds(chunk);
                let size = top_right - bottom_left;

                // Note: Assumes that the graph is a grid graph with a standard ordering
                let graph = peasant.graph;
                for x in 0..size.x {
                    for y in 0..size.y {
                        let tile = graph.tiles[x as usize * size.y as usize + y as usize].clone();
                        world.world[(bottom_left.x + x) as usize][(bottom_left.y + y) as usize] =
                            tile;
                    }
                }
                world.generated_chunks.insert(chunk, ChunkState::Done);
                render_world_event.send(RenderUpdateEvent);

                // queue neighbors
                'outer: for direction in 0..4 {
                    let neighbor = chunk + Direction::from(direction).to_ivec2();
                    let chunks = IVec2::new(
                        world.world.len() as i32 / world.chunk_size as i32,
                        world.world[0].len() as i32 / world.chunk_size as i32,
                    );
                    if !world.generated_chunks.contains_key(&neighbor)
                        && neighbor.cmpge(IVec2::ZERO).all()
                        && neighbor.cmplt(chunks).all()
                    {
                        // check if neighbor's neighbors are done
                        for direction in 0..4 {
                            let neighbor = neighbor + Direction::from(direction).to_ivec2();
                            if let Some(state) = world.generated_chunks.get(&neighbor) {
                                if *state == ChunkState::Scheduled {
                                    continue 'outer;
                                }
                            }
                        }

                        world
                            .generated_chunks
                            .insert(neighbor, ChunkState::Scheduled);
                        let graph = world.extract_chunk(neighbor);
                        let peasant = Peasant {
                            graph,
                            constraints: world.current_constraints.clone(),
                            weights: world.current_weights.clone(),
                            seed: world.seed + neighbor.x as u64 * chunks.y as u64 + neighbor.y as u64,
                            user_data: Some(Box::new(PeasantData::Chunked { chunk: neighbor })),
                        };
                        guild.cpu_executer.queue_peasant(peasant).unwrap();
                    }
                }
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

impl World {
    fn extract_chunk(&self, pos: IVec2) -> Graph<WaveFunction> {
        let (bottom_left, top_right) = self.chunk_bounds(pos);
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

    fn chunk_bounds(&self, pos: IVec2) -> (IVec2, IVec2) {
        let world_size = IVec2::new(self.world.len() as i32, self.world[0].len() as i32);
        let bottom_left = (pos * self.chunk_size as i32 - IVec2::ONE).max(IVec2::ZERO);
        let top_right = ((pos + IVec2::ONE) * self.chunk_size as i32 + IVec2::ONE).min(world_size);
        (bottom_left, top_right)
    }
}
