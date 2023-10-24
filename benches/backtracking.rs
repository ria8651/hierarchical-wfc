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

const ITTERATIONS: usize = 20;

fn main() {
    let tileset: Arc<dyn TileSet> =
        Arc::new(MxgmnTileset::new(Path::new("assets/mxgmn/Summer.xml"), None).unwrap());

    let mut rng = rand::thread_rng();
    let mut seed: u64 = rng.gen();

    let mut csv = Writer::from_path("benches/data/backtracking.csv").unwrap();
    csv.write_record(["tileset", "size", "heuristic", "time", "std_err"])
        .unwrap();

    for size in [32, 64, 96] {
        for heuristic in [
            BacktrackingHeuristic::Restart,
            BacktrackingHeuristic::Standard,
            BacktrackingHeuristic::Degree { degree: 3 },
            BacktrackingHeuristic::Proportional { proportion: 0.2 },
            BacktrackingHeuristic::Fixed { distance: 500 },
        ] {
            let time: (f64, f64) = match utils::time_process(ITTERATIONS, || -> bool {
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
                            restarts_left: 1000,
                            heuristic: heuristic.clone(),
                        },
                        timeout: Some(Duration::from_secs_f32(20.0)),
                        ..Default::default()
                    },
                    update_channel: None,
                };

                let result = SingleThreaded::execute(&mut task);
                seed += 1;

                let output = result.is_ok();
                if let Err(e) = result {
                    println!("Error during test: {:?}", e);
                }

                output
            }) {
                Ok(time) => (time.n, time.s),
                Err(e) => {
                    println!("Error {:?}", e);
                    (f64::NAN, f64::NAN)
                }
            };

            println!("{:?}: {} ± {}", heuristic, time.0, time.1);

            csv.write_record(&[
                "Summer",
                &format!("{}", size),
                &format!("{:?}", heuristic).split(" ").next().unwrap(),
                &format!("{}", time.0),
                &format!("{}", time.1),
            ])
            .unwrap();
            csv.flush().unwrap();
        }
    }
}
