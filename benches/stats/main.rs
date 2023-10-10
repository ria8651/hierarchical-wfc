use grid_wfc::{
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::GridGraphSettings,
    mxgmn_tileset::MxgmnTileset,
    world::{ChunkMerging, ChunkSettings, GenerationMode},
};
use hierarchical_wfc::{
    wfc_backend,
    wfc_task::{BacktrackingSettings, Entropy, WfcSettings},
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

const THREADS: usize = 8;
const SAMPLES: usize = 16;
const SIZE: usize = 64;
const RESTARTS: usize = 100;

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
    let tileset = Arc::new(
        MxgmnTileset::new(Path::new("assets/mxgmn/Circuit.xml"), None)
            .ok()
            .unwrap(),
    );

    // let tileset = Arc::new(CarcassonneTileset::default());
    let threaded_backend = Rc::new(RefCell::new(wfc_backend::MultiThreaded::new(THREADS)));

    let mut csv_writer = csv::Writer::from_path(format!("benches/data/quality.csv")).unwrap();
    csv_writer
        .write_record(["chunk_size", "average_t"])
        .unwrap();

    for chunk_size in [8, 16, 32] {
        let threaded = {
            let chunked_settings = ChunkedSettings {
                chunk_settings: ChunkSettings {
                    chunk_size,
                    overlap: chunk_size.div_euclid(4),
                    ..CHUNKED_SETTINGS.chunk_settings
                },
                ..CHUNKED_SETTINGS
            };

            let mut threaded_stats = {
                let tileset = tileset.clone();
                let backend = threaded_backend.clone();
                RunStatisticsBuilder::new(
                    SAMPLES,
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

        let single_1 = {
            let single_settings = SingleSettings { ..SINGLE_SETTINGS };

            let mut single_stats = {
                let tileset = tileset.clone();

                let backend = threaded_backend.clone();

                RunStatisticsBuilder::new(
                    SAMPLES,
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

        println!("\n[single vs threaded] Chunk size: {}", chunk_size);
        print!("single ");
        single_1.single.compare(&threaded.single);
        print!("pair   ");
        single_1.pair.compare(&threaded.pair);
        print!("quad   ");
        single_1.quad.compare(&threaded.quad);
    }

    // println!("\n[single vs single] ");
    // single_1.single.compare(&single_2.single);
    // single_1.pair.compare(&single_2.pair);
    // single_1.quad.compare(&single_2.quad);

    // println!("\n[single_2 vs threaded] ");
    // single_2.single.compare(&threaded.single);
    // single_2.pair.compare(&threaded.pair);
    // single_2.quad.compare(&threaded.quad);
}
