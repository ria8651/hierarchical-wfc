use anyhow::Error;
use bevy::utils::hashbrown::HashMap;
use bevy_inspector_egui::egui::Key;
use grid_wfc::{
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
    single_shot,
    world::GenerationMode,
};
use hierarchical_wfc::{
    wfc_backend::{self, Backend, SingleThreaded},
    wfc_task::BacktrackingSettings,
    Graph, Neighbor, TileSet, WaveFunction, WfcTask,
};
use std::{cell::RefCell, path::Path, rc::Rc, sync::Arc};

// https://en.wikipedia.org/wiki/Standard_deviation#Rapid_calculation_methods
#[derive(Default)]
struct RollingStdErr<T> {
    current: T,
    s_1: T,
    s_2: T,
    n: usize,
}

struct StdErr<T> {
    n: T,
    s: T,
}

impl std::fmt::Display for StdErr<f64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sf_index = self.s.log10().floor() as i32;

        let n = (self.n.abs() * (10f64).powi(1 - sf_index)).round() as i64;
        let s = (self.s.abs() * (10f64).powi(1 - sf_index)).round() as i64;

        let sign = if self.n.is_sign_negative() { "-" } else { "" };
        let n = format!("{}{}.{}", sign, n.div_euclid(10), n.rem_euclid(10));
        let s = format!("{}.{}", s.div_euclid(10), s.rem_euclid(10));

        write!(f, "({}pm{})e{}", n, s, sf_index)
    }
}
impl std::fmt::Debug for StdErr<f64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sf_index = self.s.log10().floor() as i32;

        let n = (self.n.abs() * (10f64).powi(1 - sf_index)).round() as i64;
        let s = (self.s.abs() * (10f64).powi(1 - sf_index)).round() as i64;

        let sign = if self.n.is_sign_negative() { "-" } else { "" };
        let n = format!("{}{}.{}", sign, n.div_euclid(10), n.rem_euclid(10));
        let s = format!("{}.{}", s.div_euclid(10), s.rem_euclid(10));

        write!(f, "({}pm{})e{}", n, s, sf_index)
    }
}

impl StdErr<f64> {
    /// T-Test formula for two samples:
    /// t = X_1 - X_2/S
    /// Where S is the pooled standard error:
    /// sqrt( (std_err_1)^2 + (std_err_2)^2)
    fn t_test(&self, other: &Self) -> f64 {
        (self.n - other.n) / (self.s * self.s + other.s * other.s).sqrt()
    }
}

impl RollingStdErr<f64> {
    fn increment(&mut self, v: f64) {
        self.current += v;
    }
    fn insert(&mut self, v: f64) {
        self.increment(v);
        self.commit();
    }

    fn commit(&mut self) {
        self.s_1 += self.current;
        self.s_2 += self.current * self.current;
        self.current = 0.0;
        self.n += 1;
    }
    // fn avg_manual_sample_count(&self, n: usize) -> StdErr<f64> {
    //     assert!(n > 0);

    //     let avg = self.s_1 / n as f64;
    //     let sigma = (n as f64 * self.s_2 - self.s_1 * self.s_1).sqrt() / n as f64;
    //     let std_err = sigma / (n as f64).sqrt();
    //     StdErr::<f64> { n: avg, s: std_err }
    // }
    fn avg(&self) -> StdErr<f64> {
        if self.n == 0 {
            return StdErr::<f64> { n: 0.0, s: 0.0 };
        }

        let avg = self.s_1 / self.n as f64;
        let sigma = (self.n as f64 * self.s_2 - self.s_1 * self.s_1).sqrt() / self.n as f64;
        let std_err = sigma / (self.n as f64).sqrt();
        StdErr::<f64> { n: avg, s: std_err }
    }
}
const THREADS: usize = 8;
const SAMPLES: usize = 128;
const SIZE: usize = 32;
const CHUNK_SIZE: usize = 16;
const OVERLAP: usize = 4;
const RESTARTS: usize = 100;

struct Tile<'a, T> {
    value: T,
    neigbhours: &'a [Neighbor],
}

fn tile_in_dir<'a, T: Copy>(
    graph: &'a Graph<T>,
    tile: &Tile<T>,
    direction: usize,
) -> Option<Tile<'a, T>> {
    tile.neigbhours
        .iter()
        .flat_map(
            |Neighbor {
                 index,
                 direction: dir,
             }| {
                if *dir == direction {
                    return Some(Tile {
                        value: graph.tiles[*index],
                        neigbhours: graph.neighbors[*index].as_slice(),
                    });
                } else {
                    return None;
                }
            },
        )
        .next()
}

struct Stats {
    seed: u64,
    samples: usize,
    generation_fn: Box<dyn Fn(u64) -> Result<Graph<usize>, Error>>,
    distributions_single: HashMap<usize, RollingStdErr<f64>>,
    distributions_pair: HashMap<[usize; 3], RollingStdErr<f64>>,
    distributions_quad: HashMap<[usize; 4], RollingStdErr<f64>>,
    distributions_neighbours: HashMap<[usize; 5], RollingStdErr<f64>>,
}

impl Stats {
    pub fn new(
        samples: usize,
        generation_fn: Box<dyn Fn(u64) -> Result<Graph<usize>, Error>>,
    ) -> Self {
        Self {
            samples,
            generation_fn,
            seed: 0u64,
            distributions_single: HashMap::new(),
            distributions_pair: HashMap::new(),
            distributions_quad: HashMap::new(),
            distributions_neighbours: HashMap::new(),
        }
    }

    fn update_distrubtions(&mut self, graph: Graph<usize>) {
        let iter = graph.neighbors.iter().enumerate();
        for (t, neighbours) in iter {
            let tile = graph.tiles[t];
            {
                self.distributions_single
                    .entry(tile)
                    .or_default()
                    .increment(1.0);
            }
            {
                for neighbour in neighbours {
                    if neighbour.direction.rem_euclid(2) == 0
                    // Only positive direction
                    {
                        let key = [
                            tile,
                            graph.tiles[neighbour.index].clone(),
                            neighbour.direction.div_euclid(2), // Configuration number, 0: horizontal, 1: vertical
                        ];
                        self.distributions_pair
                            .entry(key)
                            .or_default()
                            .increment(1.0);
                    }
                }
            }
            {
                let tile_0 = Tile {
                    value: tile,
                    neigbhours: neighbours.as_slice(),
                };
                let tile_1 = tile_in_dir(&graph, &tile_0, 2);
                let tile_2 = tile_in_dir(&graph, &tile_0, 0);
                let tile_3 = tile_1.as_ref().and_then(|t| tile_in_dir(&graph, &t, 0));

                if let [Some(t0), Some(t1), Some(t2), Some(t3)] =
                    [Some(tile_0), tile_1, tile_2, tile_3]
                {
                    let key = [t0.value, t1.value, t2.value, t3.value];
                    self.distributions_quad
                        .entry(key)
                        .or_default()
                        .increment(1.0);
                }
            }
            {
                let tile_0 = Tile {
                    value: tile,
                    neigbhours: neighbours.as_slice(),
                };
                let neigbhours = [0, 1, 2, 3]
                    .map(|d| tile_in_dir(&graph, &tile_0, d).and_then(|t| Some(t.value)));

                if let [Some(t0), Some(t1), Some(t2), Some(t3), Some(t4)] = [
                    Some(tile),
                    neigbhours[0],
                    neigbhours[1],
                    neigbhours[2],
                    neigbhours[3],
                ] {
                    let key = [t0, t1, t2, t3, t4];
                    self.distributions_neighbours
                        .entry(key)
                        .or_default()
                        .increment(1.0);
                }
            }
        }
        for rolling_std_err in Iterator::chain(
            Iterator::chain(
                self.distributions_single.values_mut(),
                self.distributions_pair.values_mut(),
            ),
            Iterator::chain(
                self.distributions_quad.values_mut(),
                self.distributions_neighbours.values_mut(),
            ),
        ) {
            rolling_std_err.commit();
        }
    }

    fn run(&mut self) {
        for _ in 0..self.samples {
            let result = loop {
                if let Ok(result) = (self.generation_fn)(self.seed) {
                    self.seed += 1;
                    break result;
                } else {
                    self.seed += 1;
                }
            };
            self.update_distrubtions(result);
        }
    }

    fn analyse(&self) {
        let dists = [
            self.distributions_single
                .values()
                .map(|r| r.avg())
                .collect::<Vec<_>>(),
            self.distributions_pair
                .values()
                .map(|r| r.avg())
                .collect::<Vec<_>>(),
            self.distributions_quad
                .values()
                .map(|r| r.avg())
                .collect::<Vec<_>>(),
            self.distributions_neighbours
                .values()
                .map(|r| r.avg())
                .collect::<Vec<_>>(),
        ];

        for d in dists {
            let count: usize = d
                .iter()
                .map(|s| {
                    if s.n == 0.0 {
                        0usize
                    } else {
                        (s.s / s.n < 0.1) as usize
                    }
                })
                .sum();
            let total = d.len();
            dbg!((count, total));
        }
    }
}

pub fn main() {
    let tileset = Arc::new(
        MxgmnTileset::new(Path::new("assets/mxgmn/Circuit.xml"), None)
            .ok()
            .unwrap(),
    );

    // let tileset = Arc::new(CarcassonneTileset::default());

    let threaded_backend = Rc::new(RefCell::new(wfc_backend::MultiThreaded::new(THREADS)));

    let mut single_stats = {
        let tileset = tileset.clone();

        Stats::new(
            SAMPLES,
            Box::new(move |seed| generate_single(seed, SIZE, tileset.clone())),
        )
    };
    single_stats.run();

    // let mut chunked_stats = {
    //     let tileset = tileset.clone();
    //     Stats::new(
    //         SAMPLES,
    //         Box::new(move |seed| {
    //             generate_chunked(
    //                 seed,
    //                 SIZE,
    //                 tileset.clone(),
    //                 GenerationMode::Deterministic,
    //                 threaded_backend.clone(),
    //             )
    //         }),
    //     )
    // };
    // chunked_stats.run();

    // let counts: Vec<_> = single_stats
    //     .distributions_pair
    //     .values()
    //     .map(|r| {
    //         let avg = r.avg();
    //         avg.s / avg.n
    //     })
    //     .collect();

    // dbg!(&counts);
    // dbg!(counts.iter().min_by(|a, b| a.partial_cmp(b).unwrap()));
    // dbg!(counts.iter().max_by(|a, b| a.partial_cmp(b).unwrap()));

    // let a_sparse_vec: HashMap<Key, StdErr<f64>> = {
    //     let mut valid_samples: usize = 0;
    //     let mut rolling_std_err: HashMap<[usize; 5], RollingStdErr<f64>> = HashMap::new();
    //     for seed in 0..SAMPLES {
    //         let graph = graph_grid::create(&settings, WaveFunction::filled(tileset.tile_count()));
    //         let task = WfcTask {
    //             graph,
    //             tileset: tileset.clone(),
    //             seed,
    //             metadata: None,
    //             backtracking: wfc_task::BacktrackingSettings::default(),
    //         };

    //         multi_threaded_backend.queue_task(task).unwrap();
    //     }

    //     'skip: for seed in 0..SAMPLES {
    //         dbg!(seed);
    //         let mut frequnecy: HashMap<[usize; 5], usize> = HashMap::new();

    //         'outer: loop {
    //             match output.pop() {
    //                 Some(Ok(result)) => {
    //                     let result = result.graph.validate().unwrap();
    //                     for (tile, neigbours) in result.tiles.iter().zip(result.neighbors.iter()) {
    //                         let mut tiles = [None; 5];
    //                         tiles[0] = Some(*tile);
    //                         for neighbour in neigbours {
    //                             tiles[neighbour.direction + 1] =
    //                                 Some(result.tiles[neighbour.index]);
    //                         }
    //                         if tiles.map(|t| t.is_some()).contains(&false) {
    //                             continue;
    //                         }
    //                         let tiles: [usize; 5] = tiles.map(|n| n.unwrap());
    //                         {
    //                             let value = *frequnecy.get(&tiles).unwrap_or(&0);
    //                             frequnecy.insert(tiles, value + 1); //.entry(tiles).insert(value + 1);
    //                         }
    //                     }
    //                     break 'outer;
    //                 }
    //                 Some(Err(_)) => {
    //                     continue 'skip;
    //                 }
    //                 _ => (),
    //             }
    //         }

    //         for (k, v) in frequnecy {
    //             rolling_std_err.entry(k).or_default().insert(v as f64);
    //         }
    //         valid_samples += 1;
    //     }
    //     rolling_std_err
    //         .into_iter()
    //         .map(|(k, v)| (k, v.avg_manual_sample_count(valid_samples)))
    //         .collect()
    // };
    // let b_sparse_vec: HashMap<Key, StdErr<f64>> = {
    //     let chunked_generator = get_chunked_generator(tileset.clone(), output.clone());

    //     let mut rolling_std_err: HashMap<[usize; 5], RollingStdErr<f64>> = HashMap::new();
    //     let mut valid_samples: usize = 0;
    //     for seed in 0..SAMPLES {
    //         dbg!(seed);
    //         let mut frequnecy: HashMap<[usize; 5], usize> = HashMap::new();
    //         let result =
    //             chunked_generator(seed, CHUNK_SIZE, &settings, &mut multi_threaded_backend);
    //         let result = match result {
    //             Ok(r) => r,
    //             Err(e) => {
    //                 dbg!(e);
    //                 continue;
    //             }
    //         };

    //         for j in 1..result.len() - 1 {
    //             for i in 1..result.first().unwrap().len() - 1 {
    //                 let directions = [
    //                     IVec2::new(0, 0),
    //                     IVec2::new(0, 1),
    //                     IVec2::new(0, -1),
    //                     IVec2::new(-1, 0),
    //                     IVec2::new(1, 0),
    //                 ];

    //                 let tiles = directions.map(|delta| {
    //                     result[j + delta.x as usize][i + delta.y as usize]
    //                         .collapse()
    //                         .unwrap()
    //                 });

    //                 {
    //                     let value = *frequnecy.get(&tiles).unwrap_or(&0);
    //                     frequnecy.insert(tiles, value + 1);
    //                 }
    //             }
    //         }
    //         for (k, v) in frequnecy {
    //             rolling_std_err.entry(k).or_default().insert(v as f64);
    //         }
    //         valid_samples += 1;
    //     }
    //     rolling_std_err
    //         .into_iter()
    //         .map(|(k, v)| (k, v.avg_manual_sample_count(valid_samples)))
    //         .collect()
    // };

    // let a_keys: HashSet<_> = a_sparse_vec.keys().collect();
    // let b_keys: HashSet<_> = b_sparse_vec.keys().collect();

    // let mut results = a_keys
    //     .intersection(&b_keys)
    //     .map(|k| {
    //         (
    //             **k,
    //             a_sparse_vec.get(*k).unwrap(),
    //             b_sparse_vec.get(*k).unwrap(),
    //         )
    //     })
    //     .collect::<Vec<_>>();

    // results.sort_by(|a, b| (a.1.n.max(a.2.n)).total_cmp(&(b.1.n.max(b.2.n))));

    // println!("t-test results:");
    // let mut t_tests: Vec<f64> = vec![];
    // for key in a_keys.intersection(&b_keys).into_iter() {
    //     let a = a_sparse_vec.get(*key).unwrap();
    //     let b = b_sparse_vec.get(*key).unwrap();
    //     if a.n <= 0.0 || b.n <= 0.0 {
    //         continue;
    //     }

    //     let t = a.t_test(b);
    //     println!("\t{t} for \t{a}, {b}");
    //     t_tests.push(t);
    // }
    // println!();

    // let n = t_tests.len() as f64;
    // let avg = t_tests.iter().fold(0.0, |acc, next| acc + next.abs() / n);
    // println!("Average t-test value: {avg:0.3}");

    // for res in results.iter().rev().take(10) {
    //     let (key, a, b) = res;
    //     println!("{a} {b} for {key:?}");
    // }

    // let sum_a = a_sparse_vec
    //     .values()
    //     .map(|v| v.n)
    //     .reduce(|a, b| a + b)
    //     .unwrap();

    // let sum_b = b_sparse_vec
    //     .values()
    //     .map(|v| v.n)
    //     .reduce(|a, b| a + b)
    //     .unwrap();

    // dbg!((sum_a, sum_b));
}

fn generate_single(
    seed: u64,
    size: usize,
    tileset: Arc<dyn TileSet>,
) -> Result<Graph<usize>, anyhow::Error> {
    let settings = GridGraphSettings {
        height: size,
        width: size,
        periodic: false,
    };
    let filled = WaveFunction::filled(tileset.tile_count());
    let graph = graph_grid::create(&settings, filled);

    let mut task = WfcTask {
        graph,
        tileset: tileset.clone(),
        seed,
        metadata: None,
        backtracking: BacktrackingSettings::Enabled { restarts_left: 100 },
    };

    SingleThreaded::execute(&mut task)?;
    let result = task.graph.validate()?;

    Ok(result)
}

fn generate_chunked(
    seed: u64,
    size: usize,
    tileset: Arc<dyn TileSet>,
    generation_mode: GenerationMode,
    backend: Rc<RefCell<dyn Backend>>,
) -> Result<Graph<usize>, anyhow::Error> {
    let settings = GridGraphSettings {
        height: size,
        width: size,
        periodic: false,
    };

    let (world, _) = single_shot::generate_world(
        tileset.clone(),
        &mut *backend.borrow_mut(),
        settings,
        seed,
        generation_mode,
        CHUNK_SIZE,
        OVERLAP,
        BacktrackingSettings::Enabled {
            restarts_left: RESTARTS,
        },
    );
    world.build_world_graph()
}
