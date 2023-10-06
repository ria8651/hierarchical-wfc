use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use grid_wfc::{
    basic_tileset::BasicTileset,
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
};
use hierarchical_wfc::{
    wfc_backend::SingleThreaded, wfc_task::BacktrackingSettings, TileSet, WaveFunction, WfcTask,
};
use rand::Rng;
use std::sync::Arc;

pub fn criterion_benchmark(c: &mut Criterion) {
    let tilesets = load_tilesets();

    let mut rng = rand::thread_rng();
    let mut seed: u64 = rng.gen();

    let mut group = c.benchmark_group("Map Size");
    for (tileset, tileset_name) in tilesets {
        for size in [8, 16, 32, 64].into_iter() {
            let mut iterations = 0;
            let mut failures = 0;

            group.bench_with_input(BenchmarkId::new(&tileset_name, size), &size, |b, &size| {
                b.iter(|| {
                    let settings = GridGraphSettings {
                        height: size,
                        width: size,
                        periodic: false,
                    };
                    let filled = WaveFunction::filled(tileset.tile_count());
                    let graph = graph_grid::create(&settings, filled);

                    let mut task = WfcTask {
                        graph,
                        tileset: tileset.clone(),
                        seed,
                        metadata: None,
                        backtracking: BacktrackingSettings::Enabled { restarts_left: 100 },
                    };

                    let result = SingleThreaded::execute(&mut task);
                    iterations += 1;
                    failures += result.is_err() as usize;

                    seed += 1;
                })
            });

            println!(
                "{}: {}x{} {} iterations, {} failures",
                tileset_name, size, size, iterations, failures
            );
        }
    }
}

pub fn load_tilesets() -> Vec<(Arc<dyn TileSet>, String)> {
    // load tilesets
    let mut tile_sets: Vec<(Arc<dyn TileSet>, String)> = vec![
        (
            Arc::new(CarcassonneTileset::default()),
            "CarcassonneTileset".to_string(),
        ),
        (
            Arc::new(BasicTileset::default()),
            "BasicTileset".to_string(),
        ),
    ];

    let paths = std::fs::read_dir("assets/mxgmn").unwrap();
    for path in paths {
        let path = path.unwrap().path();
        if let Some(ext) = path.extension() {
            if ext == "xml" {
                let name = path.file_stem().unwrap();
                if name == "Castle" || name == "FloorPlan" || name == "Summer" || name == "Circuit"
                {
                    tile_sets.push((
                        Arc::new(MxgmnTileset::new(&path, None).unwrap()),
                        path.file_stem().unwrap().to_str().unwrap().to_string(),
                    ));
                }
            }
        }
    }

    tile_sets
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
