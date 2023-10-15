use grid_wfc::{
    grid_graph::GridGraphSettings,
    mxgmn_tileset::MxgmnTileset,
    single_shot,
    world::{ChunkMerging, ChunkSettings, GenerationMode},
};
use hierarchical_wfc::{wfc_backend::MultiThreaded, wfc_task::WfcSettings, TileSet};
use rand::Rng;
use std::{path::Path, sync::Arc};

const THREADS: usize = 8;
const ITTERATIONS: usize = 100;
const SIZE: usize = 64;
const CHUNK_SIZE: usize = 16;
const OVERLAP: usize = 3;
const DISCARD: usize = 1;

fn main() {
    let tileset: Arc<dyn TileSet> =
        Arc::new(MxgmnTileset::new(Path::new("assets/mxgmn/summer.xml"), None).unwrap());
    let mut backend = MultiThreaded::new(THREADS);

    let mut rng = rand::thread_rng();
    let mut seed: u64 = rng.gen();

    #[derive(Debug, Clone, Copy)]
    enum ChunkingTests {
        Naive,
        Full,
        Mixed,
        MixedDiscard,
    }

    for generation_type in [
        ChunkingTests::Naive,
        ChunkingTests::Full,
        ChunkingTests::Mixed,
        ChunkingTests::MixedDiscard,
    ]
    .into_iter()
    {
        let mut failures = 0;
        for _ in 0..ITTERATIONS {
            let settings = GridGraphSettings {
                height: SIZE,
                width: SIZE,
                periodic: false,
            };

            let chunk_merging = match generation_type {
                ChunkingTests::Naive => ChunkMerging::Interior,
                ChunkingTests::Full => ChunkMerging::Full,
                ChunkingTests::Mixed | ChunkingTests::MixedDiscard => ChunkMerging::Mixed,
            };
            let discard = match generation_type {
                ChunkingTests::MixedDiscard => DISCARD,
                _ => 0,
            };

            let (_, err) = single_shot::generate_world(
                tileset.clone(),
                &mut backend,
                settings,
                seed,
                GenerationMode::NonDeterministic,
                ChunkSettings {
                    chunk_merging: chunk_merging,
                    chunk_size: CHUNK_SIZE,
                    overlap: OVERLAP,
                    discard,
                    ..Default::default()
                },
                WfcSettings::default(),
            );

            failures += err.is_err() as usize;
            seed += 1;
        }
        println!(
            "{:?} failures: {} / {}",
            generation_type, failures, ITTERATIONS
        );
    }
}
