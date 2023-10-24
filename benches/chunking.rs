use anyhow::Result;
use bevy::utils::Instant;
use core_wfc::{
    wfc_backend::{MultiThreaded, SingleThreaded},
    wfc_task::WfcSettings,
    TileSet, WaveFunction, WfcTask,
};
use csv::Writer;
use grid_wfc::{
    basic_tileset::BasicTileset,
    carcassonne_tileset::CarcassonneTileset,
    grid_graph::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
    single_shot,
    world::{ChunkSettings, GenerationMode},
};
use rand::Rng;
use stats::std_err::{RollingStdErr, StdErr};
use std::sync::Arc;

mod stats;

const THREADS: usize = 8;
const ITTERATIONS: usize = 20;

fn time_process<F: FnMut() -> bool>(mut f: F) -> Result<StdErr<f64>> {
    let mut failures = 0;
    let mut total_time = RollingStdErr::default();
    for _ in 0..ITTERATIONS {
        let now = Instant::now();
        let result = f();
        let time = now.elapsed().as_secs_f64();
        if time > 20.0 {
            return Err(anyhow::anyhow!("Too long: {}s", time));
        }

        if result {
            total_time.insert(time);
        } else {
            failures += 1;
        }

        if failures as f32 / ITTERATIONS as f32 > 0.5 {
            return Err(anyhow::anyhow!(
                "Too many failures: {} out of {}",
                failures,
                ITTERATIONS
            ));
        }
    }

    let average_time = total_time.avg();
    Ok(average_time)
}

fn main() {
    let tilesets = load_tilesets();
    let mut backend = MultiThreaded::new(THREADS);

    let mut rng = rand::thread_rng();
    let mut seed: u64 = rng.gen();

    for generation_type in ["standard", "non_deterministic", "deterministic"].iter() {
        let mut csv =
            Writer::from_path(format!("benches/data/map_size_{}.csv", generation_type)).unwrap();
        csv.write_record(["tileset", "size", "time", "std_err"])
            .unwrap();

        for size in (32..=256).step_by(32) {
            for (tileset, tileset_name) in tilesets.iter() {
                match *generation_type {
                    "standard" => {
                        let time: (f64, f64) = match time_process(|| -> bool {
                            let settings = GridGraphSettings {
                                height: size,
                                width: size,
                                periodic: false,
                            };
                            let filled = WaveFunction::filled(tileset.tile_count());
                            let graph = grid_graph::create(&settings, filled);

                            let mut task = WfcTask {
                                graph,
                                tileset: tileset.clone(),
                                seed,
                                metadata: None,
                                settings: WfcSettings::default(),
                                update_channel: None,
                            };

                            let result = SingleThreaded::execute(&mut task);
                            seed += 1;

                            result.is_ok()
                        }) {
                            Ok(time) => (time.n, time.s),
                            Err(e) => {
                                println!("{}: {}x{} {}", tileset_name, size, size, e);
                                (f64::NAN, f64::NAN)
                            }
                        };

                        println!(
                            "{}: {}x{} {} pm {}s",
                            tileset_name, size, size, time.0, time.1
                        );
                        csv.write_record([
                            tileset_name,
                            &size.to_string(),
                            &time.0.to_string(),
                            &time.1.to_string(),
                        ])
                        .unwrap();
                    }
                    "non_deterministic" | "deterministic" => {
                        let time = match time_process(|| -> bool {
                            let settings = GridGraphSettings {
                                height: size,
                                width: size,
                                periodic: false,
                            };

                            let generation_mode = match *generation_type {
                                "non_deterministic" => GenerationMode::NonDeterministic,
                                "deterministic" => GenerationMode::Deterministic,
                                _ => unreachable!(),
                            };
                            let (_, error) = single_shot::generate_world(
                                tileset.clone(),
                                &mut backend,
                                settings,
                                seed,
                                generation_mode,
                                ChunkSettings::default(),
                                WfcSettings::default(),
                            );

                            seed += 1;

                            error.is_ok()
                        }) {
                            Ok(time) => (time.n, time.s),
                            Err(e) => {
                                println!("{}: {}x{} {}", tileset_name, size, size, e);
                                (f64::NAN, f64::NAN)
                            }
                        };

                        println!(
                            "{}: {}x{} {} pm {}s",
                            tileset_name, size, size, time.0, time.1
                        );
                        csv.write_record([
                            tileset_name,
                            &size.to_string(),
                            &time.0.to_string(),
                            &time.1.to_string(),
                        ])
                        .unwrap();
                    }
                    _ => unreachable!(),
                }
            }
        }

        csv.flush().unwrap();
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
