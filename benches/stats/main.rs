use core_wfc::{
    wfc_backend,
    wfc_task::{BacktrackingHeuristic, BacktrackingSettings, Entropy, WfcSettings},
    TileSet,
};
use grid_wfc::{
    carcassonne_tileset::CarcassonneTileset,
    grid_graph::GridGraphSettings,
    mxgmn_tileset::MxgmnTileset,
    world::{ChunkMerging, ChunkSettings, GenerationMode},
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
    size: 32,
    overlap: 4,
    discard: 2,
    merging: ChunkMerging::Mixed,
};
const WFC_SETTINGS: WfcSettings = WfcSettings {
    backtracking: BacktrackingSettings::Enabled {
        restarts_left: RESTARTS,
        heuristic: BacktrackingHeuristic::Proportional { proportion: 0.7 },
    },
    entropy: Entropy::Shannon,
    progress_updates: None,
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

    let mut csv_writer = csv::Writer::from_path("benches/data/quality.csv").unwrap();
    csv_writer
        .write_record([
            "tileset",
            "size",
            "chunk_size",
            "overlap",
            "discard",
            "single",
            "pair",
            "quad",
        ])
        .unwrap();

    for (tileset, tileset_name, chunk_sizes, size, samples) in [
        (
            Arc::new(CarcassonneTileset::default()) as Arc<dyn TileSet>,
            "Carcassonne",
            [2, 4, 8],
            64,
            16,
        ),
        (
            Arc::new(
                MxgmnTileset::new(Path::new("assets/mxgmn/Summer.xml"), None)
                    .ok()
                    .unwrap(),
            ) as Arc<dyn TileSet>,
            "Summer",
            [8, 16, 32],
            64,
            32,
        ),
        (
            Arc::new(
                MxgmnTileset::new(Path::new("assets/mxgmn/Circuit.xml"), None)
                    .ok()
                    .unwrap(),
            ) as Arc<dyn TileSet>,
            "Circuit",
            [8, 16, 32],
            64,
            32,
        ),
        (
            Arc::new(
                MxgmnTileset::new(Path::new("assets/mxgmn/FloorPlan.xml"), None)
                    .ok()
                    .unwrap(),
            ) as Arc<dyn TileSet>,
            "FloorPlan",
            [8, 16, 32],
            64,
            32,
        ),
    ] {
        let single = {
            let single_settings = SingleSettings {
                grid_graph_settings: GridGraphSettings {
                    width: size,
                    height: size,
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
            single_stats.set_seed(0);
            single_stats.run();
            single_stats.build()
        };
        for (overlap, discard) in [(1, 0), (3, 2), (7, 4)] {
            for chunk_size in chunk_sizes {
                if 2 * overlap >= chunk_size {
                    continue;
                }
                println!("\nSettings:");
                println!("   Chunk size: {}", chunk_size);
                println!("      overlap: {}", overlap);
                println!("      discard: {}", discard);
                println!("      Tileset: {}", tileset_name);

                let threaded = {
                    let chunked_settings = ChunkedSettings {
                        chunk_settings: ChunkSettings {
                            size: chunk_size,
                            discard,
                            overlap,
                            ..CHUNKED_SETTINGS.chunk_settings
                        },
                        grid_graph_settings: GridGraphSettings {
                            width: size,
                            height: size,
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
                    threaded_stats.set_seed(0);
                    threaded_stats.run();
                    threaded_stats.build()
                };

                println!("Results:");
                print!("   single ");
                let t_single = single.single.compare(&threaded.single);
                print!("     pair ");
                let t_pair = single.pair.compare(&threaded.pair);
                print!("     quad ");
                let t_quad = single.quad.compare(&threaded.quad);
                csv_writer
                    .write_record([
                        tileset_name,
                        &format!("{size}"),
                        &format!("{chunk_size}"),
                        &format!("{overlap}"),
                        &format!("{discard}"),
                        &format!("{t_single}"),
                        &format!("{t_pair}"),
                        &format!("{t_quad}"),
                    ])
                    .unwrap();
            }
            csv_writer.flush().unwrap();
        }
    }
}
