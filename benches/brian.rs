use anyhow::Result;
use bevy::utils::Instant;
use csv::Writer;
use grid_wfc::{
    basic_tileset::BasicTileset,
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
    single_shot,
    world::GenerationMode,
};
use hierarchical_wfc::{
    wfc_backend::{MultiThreaded, SingleThreaded},
    wfc_task::BacktrackingSettings,
    TileSet, WaveFunction, WfcTask,
};
use rand::Rng;
use std::sync::Arc;

const THREADS: usize = 8;
const ITTERATIONS: usize = 20;

fn time_process<F: FnMut() -> bool>(mut f: F) -> Result<f64> {
    let mut failures = 0;
    let mut total_time = 0.0;
    let mut iterations = 0;
    for _ in 0..ITTERATIONS {
        let now = Instant::now();
        let result = f();
        if result {
            iterations += 1;
            total_time += now.elapsed().as_secs_f64();
        } else {
            failures += 1;
        }

        if failures as f32 / ITTERATIONS as f32 > 0.4 {
            return Err(anyhow::anyhow!(
                "Too many failures: {} out of {}",
                failures,
                ITTERATIONS
            ));
        }
    }

    let average_time = total_time / iterations as f64;
    Ok(average_time)
}

fn main() {
    let tilesets = load_tilesets();

    let mut rng = rand::thread_rng();
    let mut seed: u64 = rng.gen();

    let mut csv = Writer::from_path("target/map_size_single.csv").unwrap();
    csv.write_record(&["tileset", "size", "time"]).unwrap();

    for (tileset, tileset_name) in tilesets.iter() {
        for size in [32, 64, 128, 256].into_iter() {
            if tileset_name == "Summer" && size >= 256 {
                continue;
            }

            let time = time_process(|| -> bool {
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
                seed += 1;

                result.is_ok()
            })
            .unwrap();

            println!("{}: {}x{} {}s", tileset_name, size, size, time);
            csv.write_record(&[tileset_name, &size.to_string(), &time.to_string()])
                .unwrap();
        }
    }
    csv.flush().unwrap();

    let mut csv = Writer::from_path("target/map_size_non_deterministic.csv").unwrap();
    csv.write_record(&["tileset", "size", "time"]).unwrap();

    let mut backend = MultiThreaded::new(THREADS);
    for (tileset, tileset_name) in tilesets.iter() {
        for size in [32, 64, 128, 256].into_iter() {
            let time = time_process(|| -> bool {
                let settings = GridGraphSettings {
                    height: size,
                    width: size,
                    periodic: false,
                };

                let chunk_size = match size {
                    32 => 16,
                    _ => 32,
                };
                let (_, error) = single_shot::generate_world(
                    tileset.clone(),
                    &mut backend,
                    settings,
                    seed,
                    GenerationMode::NonDeterministic,
                    chunk_size,
                    4,
                    BacktrackingSettings::Enabled { restarts_left: 100 },
                );

                seed += 1;

                error.is_ok()
            }).unwrap_or(f64::NAN);

            println!("{}: {}x{} {}s", tileset_name, size, size, time);
            csv.write_record(&[tileset_name, &size.to_string(), &time.to_string()])
                .unwrap();
        }
    }
    csv.flush().unwrap();

    let mut csv = Writer::from_path("target/map_size_deterministic.csv").unwrap();
    csv.write_record(&["tileset", "size", "time"]).unwrap();

    let mut backend = MultiThreaded::new(THREADS);
    for (tileset, tileset_name) in tilesets.iter() {
        for size in [32, 64, 128, 256].into_iter() {
            let time = time_process(|| -> bool {
                let settings = GridGraphSettings {
                    height: size,
                    width: size,
                    periodic: false,
                };

                let chunk_size = match size {
                    32 => 16,
                    _ => 32,
                };
                let (_, error) = single_shot::generate_world(
                    tileset.clone(),
                    &mut backend,
                    settings,
                    seed,
                    GenerationMode::Deterministic,
                    chunk_size,
                    4,
                    BacktrackingSettings::Enabled { restarts_left: 100 },
                );

                seed += 1;

                error.is_ok()
            }).unwrap_or(f64::NAN);

            println!("{}: {}x{} {}s", tileset_name, size, size, time);
            csv.write_record(&[tileset_name, &size.to_string(), &time.to_string()])
                .unwrap();
        }
    }
    csv.flush().unwrap();
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
                if name == "Castle" || name == "Summer" || name == "Circuit" {
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
