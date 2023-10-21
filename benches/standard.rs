use criterion::{criterion_group, criterion_main, Criterion};
use grid_wfc::{
    carcassonne_tileset::CarcassonneTileset,
    grid_graph::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
};
use core_wfc::{
    wfc_backend::SingleThreaded, wfc_task::WfcSettings, TileSet, WaveFunction, WfcTask,
};
use rand::Rng;
use std::{path::Path, sync::Arc};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    let tileset = Arc::new(MxgmnTileset::new(Path::new("assets/mxgmn/Summer.xml"), None).unwrap());

    c.bench_function("Summer", |b| {
        b.iter(|| {
            let settings = GridGraphSettings {
                height: 32,
                width: 32,
                periodic: false,
            };
            let filled = WaveFunction::filled(tileset.tile_count());
            let graph = grid_graph::create(&settings, filled);
            let seed = rng.gen();

            let mut task = WfcTask {
                graph,
                tileset: tileset.clone(),
                seed,
                metadata: None,
                settings: WfcSettings::default(),
            };

            let result = SingleThreaded::execute(&mut task);
            match result {
                Ok(_) => {}
                Err(e) => {
                    println!("{}x{} {}", settings.width, settings.height, e);
                }
            }
        });
    });
    let tileset = Arc::new(CarcassonneTileset::default());

    c.bench_function("Carcassonne", |b| {
        b.iter(|| {
            let settings = GridGraphSettings {
                height: 32,
                width: 32,
                periodic: false,
            };
            let filled = WaveFunction::filled(tileset.tile_count());
            let graph = grid_graph::create(&settings, filled);
            let seed = rng.gen();

            let mut task = WfcTask {
                graph,
                tileset: tileset.clone(),
                seed,
                metadata: None,
                settings: WfcSettings::default(),
            };

            let result = SingleThreaded::execute(&mut task);
            match result {
                Ok(_) => {}
                Err(e) => {
                    println!("{}x{} {}", settings.width, settings.height, e);
                }
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
