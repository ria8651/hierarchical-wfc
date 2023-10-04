use bevy::{prelude::*, utils::HashMap};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration};
use crossbeam::queue::SegQueue;
use grid_wfc::{
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::{self, GridGraphSettings},
    world::{ChunkState, World},
};
use hierarchical_wfc::{
    wfc_backend::{self, Backend},
    wfc_task, TileSet, WaveFunction, WfcTask,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::sync::Arc;

pub fn criterion_benchmark(c: &mut Criterion) {
    let tileset = Arc::new(CarcassonneTileset::default());

    let mut seed = 0;
    let threads = 8;

    let output = Arc::new(SegQueue::new());
    let mut single_threaded_backend = wfc_backend::SingleThreaded::new(output.clone());
    let mut multi_threaded_backend = wfc_backend::MultiThreaded::new(output.clone(), threads);

    let chunked = |seed: u64,
                   chunk_size: usize,
                   settings: &GridGraphSettings,
                   backend: &mut dyn wfc_backend::Backend| {
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
            overlap: 1,
            seed,
            tileset: tileset.clone(),
            outstanding: 0,
        };
        world.start_generation(start_chunk, backend, Some(Box::new(start_chunk)));

        // process output
        let chunk_count = (chunks.x * chunks.y) as usize;
        'outer: loop {
            match output.pop() {
                Some(Ok(task)) => {
                    let chunk = task.metadata.as_ref().unwrap().downcast_ref().unwrap();

                    world.process_chunk(
                        *chunk,
                        task,
                        backend,
                        Box::new(|chunk| Some(Box::new(chunk))),
                    );
                }
                Some(Err(_)) => {
                    break 'outer;
                }
                None => (),
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
                        chunked(seed, chunk_size, &settings, &mut single_threaded_backend);
                        seed += 1;
                    })
                },
            );
            group.bench_with_input(
                BenchmarkId::new("Multi", chunk_size),
                chunk_size,
                |b, &chunk_size| {
                    b.iter(|| {
                        chunked(seed, chunk_size, &settings, &mut multi_threaded_backend);
                        seed += 1;
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
                    let graph =
                        graph_grid::create(&settings, WaveFunction::filled(tileset.tile_count()));
                    let task = WfcTask {
                        graph,
                        tileset: tileset.clone(),
                        seed,
                        metadata: None,
                        backtracking: wfc_task::BacktrackingSettings::default(),
                    };
                    seed += 1;

                    single_threaded_backend.queue_task(task).unwrap();

                    // wait for data in output
                    while output.pop().is_none() {}
                })
            });
            group.bench_with_input(BenchmarkId::new("Chunked", size), &size, |b, _| {
                b.iter(|| {
                    chunked(seed, chunk_size, &settings, &mut single_threaded_backend);
                })
            });
            group.bench_with_input(BenchmarkId::new("Multi", size), &size, |b, _| {
                b.iter(|| {
                    chunked(seed, chunk_size, &settings, &mut multi_threaded_backend);
                })
            });
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
