use bevy_inspector_egui::egui::Grid;
use grid_wfc::{
    carcassonne_tileset::CarcassonneTileset,
    grid_graph::GridGraphSettings,
    mxgmn_tileset::MxgmnTileset,
    world::{ChunkMerging, ChunkSettings, GenerationMode},
};
use hierarchical_wfc::{
    wfc_backend,
    wfc_task::{BacktrackingSettings, Entropy, WfcSettings},
    TileSet,
};
use std::{cell::RefCell, path::Path, rc::Rc, sync::Arc};

mod std_err;

mod stats_builder;
use stats_builder::RunStatisticsBuilder;

mod chunked;
mod single;
use chunked::ChunkedSettings;
use single::SingleSettings;

mod tile_util;

use crate::{chunked::ChunkedRunner, single::SingleRunner, stats_builder::SparseDistribution};

const THREADS: usize = 10;
const SIZE: usize = 64;
const RESTARTS: usize = 25;

const GRID_GRAPH_SETTINGS: GridGraphSettings = GridGraphSettings {
    height: SIZE,
    width: SIZE,
    periodic: false,
};
const CHUNK_SETTINGS: ChunkSettings = ChunkSettings {
    chunk_size: 32,
    overlap: 4,
    discard: 2,
    chunk_merging: ChunkMerging::Mixed,
};
const WFC_SETTINGS: WfcSettings = WfcSettings {
    backtracking: BacktrackingSettings::Enabled {
        restarts_left: RESTARTS,
    },
    entropy: Entropy::Shannon,
};

const SINGLE_SETTINGS: SingleSettings = SingleSettings {
    size: SIZE,
    wfc_settings: WFC_SETTINGS,
    grid_graph_settings: GridGraphSettings {
        height: SIZE,
        width: SIZE,
        periodic: false,
    },
};

const CHUNKED_SETTINGS: ChunkedSettings = ChunkedSettings {
    generation_mode: GenerationMode::NonDeterministic,
    grid_graph_settings: GRID_GRAPH_SETTINGS,
    chunk_settings: CHUNK_SETTINGS,
    wfc_settings: WFC_SETTINGS,
};

pub fn main() {
    // let tileset = Arc::new(CarcassonneTileset::default());
    let threaded_backend = Rc::new(RefCell::new(wfc_backend::MultiThreaded::new(THREADS)));

    let mut csv_writer = csv::Writer::from_path(format!("benches/data/quality.csv")).unwrap();
    csv_writer
        .write_record(["chunk_size", "average_t"])
        .unwrap();

    for (tileset, tileset_name, scale, samples) in [
        (
            Arc::new(CarcassonneTileset::default()) as Arc<dyn TileSet>,
            "Carcassonne",
            1,
            8,
        ),
        (
            Arc::new(
                MxgmnTileset::new(Path::new("assets/mxgmn/Circuit.xml"), None)
                    .ok()
                    .unwrap(),
            ) as Arc<dyn TileSet>,
            "Circuit",
            1,
            8,
        ),
    ] {
        let single = {
            let single_settings = SingleSettings {
                grid_graph_settings: GridGraphSettings {
                    width: SIZE / scale,
                    height: SIZE / scale,
                    ..GRID_GRAPH_SETTINGS
                },

                ..SINGLE_SETTINGS
            };

            let mut single_stats = {
                let tileset = tileset.clone();

                let backend = threaded_backend.clone();

                RunStatisticsBuilder::new(
                    samples,
                    Box::new(SingleRunner {
                        tileset,
                        backend,
                        settings: single_settings,
                    }),
                )
            };
            single_stats.run();
            single_stats.build()
        };
        for chunk_size in [8, 16, 32] {
            println!("\nSettings:");
            println!("   Chunk size: {}", chunk_size / scale);
            println!("      Tileset: {}", tileset_name);

            let threaded = {
                let chunked_settings = ChunkedSettings {
                    chunk_settings: ChunkSettings {
                        chunk_size,
                        overlap: chunk_size.div_euclid(4),
                        ..CHUNKED_SETTINGS.chunk_settings
                    },
                    grid_graph_settings: GridGraphSettings {
                        width: SIZE / scale,
                        height: SIZE / scale,
                        ..GRID_GRAPH_SETTINGS
                    },
                    ..CHUNKED_SETTINGS
                };

                let mut threaded_stats = {
                    let tileset = tileset.clone();
                    let backend = threaded_backend.clone();
                    RunStatisticsBuilder::new(
                        samples,
                        Box::new(ChunkedRunner {
                            backend,
                            tileset,
                            seeds: vec![],
                            setings: chunked_settings,
                        }),
                    )
                };
                threaded_stats.run();
                threaded_stats.build()
            };

            println!("Results:");
            print!("   single ");
            single.single.compare(&threaded.single);
            print!("     pair ");
            single.pair.compare(&threaded.pair);
            print!("     quad ");
            single.quad.compare(&threaded.quad);
        }
    }
}
