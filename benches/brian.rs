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
use std::sync::Arc;

const THREADS: usize = 8;
const ITTERATIONS: usize = 20;

fn time_process<F: FnMut() -> bool>(mut f: F) -> Result<StdErr<f64>> {
    let mut failures = 0;
    let mut total_time = RollingStdErr::default();
    let mut iterations = 0;
    for _ in 0..ITTERATIONS {
        let now = Instant::now();
        let result = f();
        let time = now.elapsed().as_secs_f64();
        if time > 10.0 {
            return Err(anyhow::anyhow!("Too long: {}s", time));
        }

        if result {
            iterations += 1;
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

        for size in [64, 128, 256].into_iter() {
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

pub struct StdErr<T> {
    pub n: T,
    pub s: T,
}

impl std::fmt::Display for StdErr<f64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sf_index = self.s.log10().floor() as i32;

        let n = (self.n.abs() * (10f64).powi(1 - sf_index)).round() as i64;
        let s = (self.s.abs() * (10f64).powi(1 - sf_index)).round() as i64;

        let sign = if self.n.is_sign_negative() { "-" } else { "" };
        let n = format!("{}{}.{}", sign, n.div_euclid(10), n.rem_euclid(10));
        let s = format!("{}.{}", s.div_euclid(10), s.rem_euclid(10));

        write!(f, "({}pm{})e{}", n, s, sf_index)
    }
}

impl std::fmt::Debug for StdErr<f64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sf_index = self.s.log10().floor() as i32;

        let n = (self.n.abs() * (10f64).powi(1 - sf_index)).round() as i64;
        let s = (self.s.abs() * (10f64).powi(1 - sf_index)).round() as i64;

        let sign = if self.n.is_sign_negative() { "-" } else { "" };
        let n = format!("{}{}.{}", sign, n.div_euclid(10), n.rem_euclid(10));
        let s = format!("{}.{}", s.div_euclid(10), s.rem_euclid(10));

        write!(f, "({}pm{})e{}", n, s, sf_index)
    }
}

impl StdErr<f64> {
    /// T-Test formula for two samples:
    /// t = X_1 - X_2/S
    /// Where S is the pooled standard error:
    /// sqrt( (std_err_1)^2 + (std_err_2)^2)
    pub fn t_test(&self, other: &Self) -> f64 {
        (self.n - other.n) / (self.s * self.s + other.s * other.s).sqrt()
    }
}

// https://en.wikipedia.org/wiki/Standard_deviation#Rapid_calculation_methods
#[derive(Default)]
pub struct RollingStdErr<T> {
    pub current: T,
    pub s_1: T,
    pub s_2: T,
    pub n: usize,
}

impl RollingStdErr<f64> {
    pub fn insert(&mut self, v: f64) {
        self.increment(v);
        self.commit();
    }

    pub fn increment(&mut self, v: f64) {
        self.current += v;
    }

    pub fn commit(&mut self) {
        self.s_1 += self.current;
        self.s_2 += self.current * self.current;
        self.current = 0.0;
        self.n += 1;
    }

    pub fn avg(&self) -> StdErr<f64> {
        if self.n == 0 {
            return StdErr::<f64> { n: 0.0, s: 0.0 };
        }

        let avg = self.s_1 / self.n as f64;
        let sigma = (self.n as f64 * self.s_2 - self.s_1 * self.s_1).sqrt() / self.n as f64;
        let std_err = sigma / (self.n as f64).sqrt();
        StdErr::<f64> { n: avg, s: std_err }
    }
}
