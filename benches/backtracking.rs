use bevy::utils::Instant;
use core_wfc::{
    wfc_backend::SingleThreaded,
    wfc_task::{BacktrackingHeuristic, BacktrackingSettings, WfcSettings},
    TileSet, WaveFunction, WfcTask,
};
use csv::Writer;
use grid_wfc::{
    grid_graph::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
};
use rand::Rng;
use std::{path::Path, sync::Arc, time::Duration};

mod stats;
mod utils;

fn main() {
    let tileset: Arc<dyn TileSet> =
        Arc::new(MxgmnTileset::new(Path::new("assets/mxgmn/Summer.xml"), None).unwrap());

    let mut rng = rand::thread_rng();
    let mut seed: u64 = rng.gen();

    let mut csv = Writer::from_path("benches/data/backtracking.csv").unwrap();
    csv.write_record(["tileset", "size", "heuristic", "time"])
        .unwrap();

    for (size, itterations, timeout) in [(32, 20, 1.0), (64, 10, 20.0), (96, 5, 30.0)] {
        for heuristic in [
            BacktrackingHeuristic::Restart,
            BacktrackingHeuristic::Standard,
            BacktrackingHeuristic::Degree { degree: 3 },
            BacktrackingHeuristic::Proportional { proportion: 0.2 },
            BacktrackingHeuristic::Fixed { distance: 500 },
        ] {
            for _ in 0..itterations {
                let time = Instant::now();
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
                    settings: WfcSettings {
                        backtracking: BacktrackingSettings::Enabled {
                            restarts_left: 100000,
                            heuristic: heuristic.clone(),
                        },
                        timeout: Some(Duration::from_secs_f32(timeout)),
                        ..Default::default()
                    },
                    update_channel: None,
                };

                let result = SingleThreaded::execute(&mut task);
                seed += 1;

                if let Err(e) = result {
                    println!("Error during test: {:?}", e);
                }
                let time = time.elapsed().as_secs_f64();

                println!("{:?}: {}", heuristic, time);

                csv.write_record(&[
                    "Summer",
                    &format!("{}", size),
                    &format!("{:?}", heuristic).split(" ").next().unwrap(),
                    &format!("{}", time),
                ])
                .unwrap();
                csv.flush().unwrap();
            }
        }
    }
}
