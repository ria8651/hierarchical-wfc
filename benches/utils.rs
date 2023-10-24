use anyhow::Result;
use bevy::utils::Instant;
use core_wfc::TileSet;
use grid_wfc::{
    basic_tileset::BasicTileset, carcassonne_tileset::CarcassonneTileset,
    mxgmn_tileset::MxgmnTileset,
};
use stats::{RollingStdErr, StdErr};
use std::sync::Arc;

#[path = "./stats/std_err.rs"]
mod stats;

pub fn time_process<F: FnMut() -> bool>(itterations: usize, mut f: F) -> Result<StdErr<f64>> {
    let mut failures = 0;
    let mut total_time = RollingStdErr::default();
    for _ in 0..itterations {
        let now = Instant::now();
        let result = f();
        let time = now.elapsed().as_secs_f64();

        if result {
            total_time.insert(time);
        } else {
            failures += 1;
        }

        if failures as f32 / itterations as f32 >= 0.5 {
            return Err(anyhow::anyhow!(
                "Too many failures: {} out of {}",
                failures,
                itterations
            ));
        }
    }

    let average_time = total_time.avg();
    Ok(average_time)
}

#[allow(dead_code)]
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
