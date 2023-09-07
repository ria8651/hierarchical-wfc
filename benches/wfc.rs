use bevy::{prelude::*, utils::HashMap};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration};
use crossbeam::queue::SegQueue;
use hierarchical_wfc::{
    CpuExecutor, Executor, MultiThreadedExecutor, Peasant, TileSet, WaveFunction,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::sync::Arc;
use utilities::{
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::GridGraphSettings,
    world::{ChunkState, World},
};

pub fn criterion_benchmark(c: &mut Criterion) {
    let tileset = Box::new(CarcassonneTileset::default());
    let constraints = Arc::new(tileset.get_constraints());
    let weights = Arc::new(tileset.get_weights());

    let seed = 0;
    let threads = 8;

    let output = Arc::new(SegQueue::new());
    let mut cpu_executor = CpuExecutor::new(output.clone());
    let mut multithreaded_executor = MultiThreadedExecutor::new(output.clone(), threads);

    let chunked = |chunk_size: usize, settings: &GridGraphSettings, executor: &mut dyn Executor| {
        let mut rng = SmallRng::seed_from_u64(seed);
        let chunks = IVec2::new(
            settings.width as i32 / chunk_size as i32,
            settings.height as i32 / chunk_size as i32,
        )
        .max(IVec2::ONE);
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
        world.start_generation(start_chunk, executor, Some(Box::new(start_chunk)));

        // process output
        let chunk_count = (chunks.x * chunks.y) as usize;
        'outer: loop {
            if let Some(peasant) = output.pop() {
                let chunk = peasant.user_data.as_ref().unwrap().downcast_ref().unwrap();

                world.process_chunk(
                    *chunk,
                    peasant,
                    executor,
                    Box::new(|chunk| Some(Box::new(chunk))),
                );
            }

            if world.generated_chunks.len() >= chunk_count {
                for (_, state) in world.generated_chunks.iter() {
                    if *state != ChunkState::Done {
                        continue 'outer;
                    }
                }

                break;
            }
        }
    };

    {
        let mut group = c.benchmark_group("Chunk Size");
        for chunk_size in [1, 2, 4, 8, 16, 32, 64].iter() {
            let size = 64;
            let settings = GridGraphSettings {
                height: size,
                width: size,
                periodic: false,
            };

            group.bench_with_input(
                BenchmarkId::new("Chunked", chunk_size),
                chunk_size,
                |b, &chunk_size| {
                    b.iter(|| {
                        chunked(chunk_size, &settings, &mut cpu_executor);
                    })
                },
            );
            group.bench_with_input(
                BenchmarkId::new("Multi", chunk_size),
                chunk_size,
                |b, &chunk_size| {
                    b.iter(|| {
                        chunked(chunk_size, &settings, &mut multithreaded_executor);
                    })
                },
            );
        }
    }

    {
        let mut group = c.benchmark_group("Map Size");
        group.plot_config(
            PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic),
        );

        for &size in [4, 8, 16, 32, 64, 128].iter() {
            let chunk_size = 8;
            let settings = GridGraphSettings {
                height: size,
                width: size,
                periodic: false,
            };

            group.bench_with_input(BenchmarkId::new("Single", size), &size, |b, _| {
                b.iter(|| {
                    let graph = tileset.create_graph(&settings);
                    let peasant = Peasant {
                        graph,
                        constraints: constraints.clone(),
                        weights: weights.clone(),
                        seed,
                        user_data: None,
                        tile_count: tileset.tile_count(),
                    };

                    cpu_executor.queue_peasant(peasant).unwrap();

                    // wait for data in output
                    while let None = output.pop() {}
                })
            });
            group.bench_with_input(BenchmarkId::new("Chunked", size), &size, |b, _| {
                b.iter(|| {
                    chunked(chunk_size, &settings, &mut cpu_executor);
                })
            });
            group.bench_with_input(BenchmarkId::new("Multi", size), &size, |b, _| {
                b.iter(|| {
                    chunked(chunk_size, &settings, &mut multithreaded_executor);
                })
            });
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
