use bevy::{prelude::*, utils::HashMap};
use criterion::{criterion_group, criterion_main, Criterion};
use crossbeam::queue::SegQueue;
use hierarchical_wfc::{
    CpuExecutor, Executor, MultiThreadedExecutor, Peasant, TileSet, WaveFunction,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::sync::Arc;
use utilities::{
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::{Direction, GridGraphSettings},
    world::{ChunkState, World},
};

pub fn criterion_benchmark(c: &mut Criterion) {
    let tileset = Box::new(CarcassonneTileset::default());
    let constraints = Arc::new(tileset.get_constraints());
    let weights = Arc::new(tileset.get_weights());

    let seed = 0;
    let width = 64;
    let height = 64;
    let settings = GridGraphSettings {
        height,
        width,
        periodic: false,
    };

    let chunk_size = 8;
    let threads = 8;

    let output = Arc::new(SegQueue::new());
    let mut cpu_executor = CpuExecutor::new(output.clone());
    let mut multithreaded_executor = MultiThreadedExecutor::new(output.clone(), threads);

    c.bench_function("single", |b| {
        b.iter(|| {
            let graph = tileset.create_graph(&settings);
            let peasant = Peasant {
                graph,
                constraints: constraints.clone(),
                weights: weights.clone(),
                seed,
                user_data: None,
            };

            cpu_executor.queue_peasant(peasant).unwrap();

            // wait for data in output
            while output.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let _ = output.pop();
        })
    });

    c.bench_function("chunked", |b| {
        b.iter(|| {
            let mut rng = SmallRng::seed_from_u64(seed);
            let chunks = IVec2::new(
                settings.width as i32 / chunk_size as i32,
                settings.height as i32 / chunk_size as i32,
            );
            let start_chunk = IVec2::new(rng.gen_range(0..chunks.x), rng.gen_range(0..chunks.y));

            let filled = WaveFunction::filled(tileset.tile_count());
            let mut world = World {
                world: vec![vec![filled; settings.height]; settings.width],
                generated_chunks: HashMap::from_iter(vec![(start_chunk, ChunkState::Scheduled)]),
                chunk_size,
                seed,
                current_constraints: constraints.clone(),
                current_weights: weights.clone(),
            };

            let graph = world.extract_chunk(start_chunk);
            let peasant = Peasant {
                graph,
                constraints: constraints.clone(),
                weights: weights.clone(),
                seed,
                user_data: Some(Box::new(start_chunk)),
            };

            cpu_executor.queue_peasant(peasant).unwrap();

            // process output
            let chunk_count = (chunks.x * chunks.y) as usize;
            while world.generated_chunks.len() < chunk_count {
                if let Some(peasant) = output.pop() {
                    let chunk = *peasant.user_data.unwrap().downcast::<IVec2>().unwrap();
                    world.merge_chunk(chunk, peasant.graph);
                    world.generated_chunks.insert(chunk, ChunkState::Done);

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
                            let seed = world.seed
                                + neighbor.x as u64 * chunks.y as u64
                                + neighbor.y as u64;

                            let peasant = Peasant {
                                graph,
                                constraints: world.current_constraints.clone(),
                                weights: world.current_weights.clone(),
                                seed,
                                user_data: Some(Box::new(neighbor)),
                            };

                            cpu_executor.queue_peasant(peasant).unwrap();
                        }
                    }
                }
            }
        })
    });

    c.bench_function("multi", |b| {
        b.iter(|| {
            let mut rng = SmallRng::seed_from_u64(seed);
            let chunks = IVec2::new(
                settings.width as i32 / chunk_size as i32,
                settings.height as i32 / chunk_size as i32,
            );
            let start_chunk = IVec2::new(rng.gen_range(0..chunks.x), rng.gen_range(0..chunks.y));

            let filled = WaveFunction::filled(tileset.tile_count());
            let mut world = World {
                world: vec![vec![filled; settings.height]; settings.width],
                generated_chunks: HashMap::from_iter(vec![(start_chunk, ChunkState::Scheduled)]),
                chunk_size,
                seed,
                current_constraints: constraints.clone(),
                current_weights: weights.clone(),
            };

            let graph = world.extract_chunk(start_chunk);
            let peasant = Peasant {
                graph,
                constraints: constraints.clone(),
                weights: weights.clone(),
                seed,
                user_data: Some(Box::new(start_chunk)),
            };

            multithreaded_executor.queue_peasant(peasant).unwrap();

            // process output
            let chunk_count = (chunks.x * chunks.y) as usize;
            while world.generated_chunks.len() < chunk_count {
                if let Some(peasant) = output.pop() {
                    let chunk = *peasant.user_data.unwrap().downcast::<IVec2>().unwrap();
                    world.merge_chunk(chunk, peasant.graph);
                    world.generated_chunks.insert(chunk, ChunkState::Done);

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
                            let seed = world.seed
                                + neighbor.x as u64 * chunks.y as u64
                                + neighbor.y as u64;

                            let peasant = Peasant {
                                graph,
                                constraints: world.current_constraints.clone(),
                                weights: world.current_weights.clone(),
                                seed,
                                user_data: Some(Box::new(neighbor)),
                            };

                            multithreaded_executor.queue_peasant(peasant).unwrap();
                        }
                    }
                }
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
