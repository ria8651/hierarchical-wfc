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
use chunked::{generate_chunked, ChunkedSettings};
use single::{dispatch_single, SingleSettings};

mod tile_util;

use crate::{single::await_single, stats_builder::SparseDistribution};

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
    let threaded = {
        let mut threaded_stats = {
            let tileset = tileset.clone();
            let backend = threaded_backend.clone();
            RunStatisticsBuilder::new(
                SAMPLES,
                Box::new(|_| {}),
                Box::new(move |seed| {
                    generate_chunked(seed, tileset.clone(), backend.clone(), CHUNKED_SETTINGS)
                }),
            )
        };
        threaded_stats.run();
        threaded_stats.build()
    };

    let single = {
        let mut single_stats = {
            let tileset = tileset.clone();

            let backend = threaded_backend.clone();
            let queue_fn = Box::new(move |seed| {
                dispatch_single(seed, tileset.clone(), backend.clone(), SINGLE_SETTINGS)
            });

            let backend = threaded_backend.clone();
            let await_fn = Box::new(move |seed| await_single(backend.clone(), seed));

            RunStatisticsBuilder::new(SAMPLES, queue_fn, await_fn)
        };
        single_stats.run();
        single_stats.build()
    };

    let single_2 = {
        let mut single_stats = {
            let tileset = tileset.clone();

            let backend = threaded_backend.clone();
            let queue_fn = Box::new(move |seed| {
                dispatch_single(seed, tileset.clone(), backend.clone(), SINGLE_SETTINGS)
            });

            let backend = threaded_backend.clone();
            let await_fn = Box::new(move |seed| await_single(backend.clone(), seed));

            RunStatisticsBuilder::new(SAMPLES, queue_fn, await_fn)
        };
        single_stats.set_seed(172341234);
        single_stats.run();
        single_stats.build()
    };

    print!("[single_1 vs single_2] ");
    single.single.compare(&single_2.single);
    print!("[single_1 vs threaded] ");
    single.single.compare(&threaded.single);
    print!("[single_2 vs threaded] ");
    single_2.single.compare(&threaded.single);
}
