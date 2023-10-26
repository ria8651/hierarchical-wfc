use core_wfc::{
    wfc_backend::{Backend, MultiThreaded},
    wfc_task::WfcSettings,
    TileSet, WaveFunction, WfcTask,
};
use grid_wfc::{
    grid_graph::GridGraphSettings,
    mxgmn_tileset::MxgmnTileset,
    single_shot,
    world::{ChunkMerging, ChunkSettings, GenerationMode},
};
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
        Arc::new(MxgmnTileset::new(Path::new("assets/mxgmn/Summer.xml"), None).unwrap());
    let mut backend = MultiThreaded::new(THREADS);

    let mut rng = rand::thread_rng();
    let mut seed: u64 = rng.gen();

    #[derive(Debug, Clone, Copy)]
    enum ChunkingTests {
        Interior,
        Exterior,
        Ours,
        OursDiscard,
        OursDeterministic,
    }

    for generation_type in [
        None,
        Some(ChunkingTests::Interior),
        Some(ChunkingTests::Exterior),
        Some(ChunkingTests::Ours),
        Some(ChunkingTests::OursDiscard),
        Some(ChunkingTests::OursDeterministic),
    ]
    .into_iter()
    {
        let mut failures = 0;

        if let Some(method) = generation_type {
            for i in 0..ITTERATIONS {
                dbg!(i);
                let settings = GridGraphSettings {
                    height: SIZE,
                    width: SIZE,
                    periodic: false,
                };

                let merging = match method {
                    ChunkingTests::Interior => ChunkMerging::Interior,
                    ChunkingTests::Exterior => ChunkMerging::Full,
                    ChunkingTests::Ours
                    | ChunkingTests::OursDiscard
                    | ChunkingTests::OursDeterministic => ChunkMerging::Mixed,
                };
                let discard = match method {
                    ChunkingTests::OursDiscard | ChunkingTests::OursDeterministic => DISCARD,
                    _ => 0,
                };
                let generation_mode = match method {
                    ChunkingTests::OursDeterministic => GenerationMode::Deterministic,
                    _ => GenerationMode::NonDeterministic,
                };

                let (_, err) = single_shot::generate_world(
                    tileset.clone(),
                    &mut backend,
                    settings,
                    seed,
                    generation_mode,
                    ChunkSettings {
                        merging,
                        size: CHUNK_SIZE,
                        overlap: OVERLAP,
                        discard,
                        ..Default::default()
                    },
                    WfcSettings {
                        ..Default::default()
                    },
                );

                failures += err.is_err() as usize;
                seed += 1;
            }
        } else {
            for _ in 0..ITTERATIONS {
                let settings = GridGraphSettings {
                    height: SIZE,
                    width: SIZE,
                    periodic: false,
                };

                let backend: &mut dyn Backend = &mut backend;

                let graph = grid_wfc::grid_graph::create(
                    &settings,
                    WaveFunction::filled(tileset.tile_count()),
                );

                backend
                    .queue_task(WfcTask {
                        update_channel: None,
                        graph,
                        seed,
                        tileset: tileset.clone(),
                        metadata: None,
                        settings: WfcSettings {
                            ..Default::default()
                        },
                    })
                    .unwrap();
            }
            for i in 0..ITTERATIONS {
                dbg!(i);
                let (_, err) = backend.wait_for_output();
                failures += err.is_err() as usize;
                seed += 1;
            }
        }

        println!(
            "{:?} failures: {} / {}",
            generation_type, failures, ITTERATIONS
        );
    }
}
