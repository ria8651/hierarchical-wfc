use anyhow::Error;
use bevy::utils::{hashbrown::HashMap, HashSet};
use grid_wfc::{
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
    single_shot,
    world::GenerationMode,
};
use hierarchical_wfc::{
    wfc_backend::{self, Backend, SingleThreaded},
    wfc_task::{BacktrackingSettings, Entropy, WfcSettings},
    Graph, Neighbor, TileSet, WaveFunction, WfcTask,
};
use std::{cell::RefCell, hash::Hash, path::Path, rc::Rc, sync::Arc};

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
const SAMPLES: usize = 256;
const SIZE: usize = 64;
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

struct StatsDistributions {
    single: HashMap<usize, StdErr<f64>>,
    pair: HashMap<[usize; 3], StdErr<f64>>,
    quad: HashMap<[usize; 4], StdErr<f64>>,
    neighbours: HashMap<[usize; 5], StdErr<f64>>,
}

struct StatsBuilder {
    seed: u64,
    samples: usize,
    generation_fn: Box<dyn Fn(u64) -> Result<Graph<usize>, Error>>,
    distributions_single: HashMap<usize, RollingStdErr<f64>>,
    distributions_pair: HashMap<[usize; 3], RollingStdErr<f64>>,
    distributions_quad: HashMap<[usize; 4], RollingStdErr<f64>>,
    distributions_neighbours: HashMap<[usize; 5], RollingStdErr<f64>>,
}
trait Distribution<K> {
    fn reasonable_keys(&self) -> HashSet<K>;
    fn compare(&self, other: &Self) -> ();
}

impl<K: Eq + Hash + Clone> Distribution<K> for HashMap<K, StdErr<f64>> {
    fn reasonable_keys(&self) -> HashSet<K> {
        HashSet::from_iter(self.iter().flat_map(|(k, v)| {
            if v.n == 0.0 {
                None
            } else {
                if v.s / v.n < 0.1 {
                    Some(k.clone())
                } else {
                    None
                }
            }
        }))
    }

    fn compare(&self, other: &Self) {
        let a_keys = self.reasonable_keys();
        let b_keys = other.reasonable_keys();
        let keys = a_keys.intersection(&b_keys);

        let mut count = 0.0;
        let mut avg = 0.0;
        for k in keys {
            let a = self.get(k).unwrap();
            let b = other.get(k).unwrap();
            // println!("{:.2} {:.2}: {:.4}", a.n, b.n, a.t_test(b));
            avg += a.t_test(b).abs();
            count += 1.0;
        }
        avg /= count;
        println!("avg: {avg:.4} ({count})");
    }
}

impl StatsBuilder {
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

    fn build(&self) -> StatsDistributions {
        let distributions = StatsDistributions {
            single: self
                .distributions_single
                .iter()
                .map(|(k, r)| (*k, r.avg()))
                .collect::<HashMap<_, _>>(),
            pair: self
                .distributions_pair
                .iter()
                .map(|(k, r)| (*k, r.avg()))
                .collect::<HashMap<_, _>>(),
            quad: self
                .distributions_quad
                .iter()
                .map(|(k, r)| (*k, r.avg()))
                .collect::<HashMap<_, _>>(),
            neighbours: self
                .distributions_neighbours
                .iter()
                .map(|(k, r)| (*k, r.avg()))
                .collect::<HashMap<_, _>>(),
        };

        let dists = [
            distributions.single.values().collect::<Vec<_>>(),
            distributions.pair.values().collect::<Vec<_>>(),
            distributions.quad.values().collect::<Vec<_>>(),
            distributions.neighbours.values().collect::<Vec<_>>(),
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
        distributions
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

    let single = {
        let mut single_stats = {
            let tileset = tileset.clone();

            StatsBuilder::new(
                SAMPLES,
                Box::new(move |seed| generate_single(seed, SIZE, tileset.clone())),
            )
        };
        single_stats.run();
        single_stats.build()
    };

    let single_2 = {
        let mut single_stats = {
            let tileset = tileset.clone();

            StatsBuilder::new(
                SAMPLES,
                Box::new(move |seed| generate_single(seed, SIZE, tileset.clone())),
            )
        };
        single_stats.seed = 172341234;
        single_stats.run();
        single_stats.build()
    };

    let threaded = {
        let mut threaded_stats = {
            let tileset = tileset.clone();
            let backend = threaded_backend.clone();
            StatsBuilder::new(
                SAMPLES,
                Box::new(move |seed| {
                    generate_chunked(
                        seed,
                        SIZE,
                        tileset.clone(),
                        GenerationMode::NonDeterministic,
                        backend.clone(),
                    )
                }),
            )
        };
        threaded_stats.run();
        threaded_stats.build()
    };

    print!("[single_1 vs single_2] ");
    single.single.compare(&single_2.single);
    print!("[single_1 vs threaded] ");
    single.single.compare(&threaded.single);
    print!("[single_2 vs threaded] ");
    single_2.single.compare(&threaded.single);
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
        settings: WfcSettings {
            backtracking: BacktrackingSettings::Enabled {
                restarts_left: RESTARTS,
            },
            entropy: Entropy::Shannon,
        },
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
        WfcSettings {
            backtracking: BacktrackingSettings::Enabled {
                restarts_left: RESTARTS,
            },
            entropy: Entropy::Shannon,
        },
    );
    world.build_world_graph()
}
