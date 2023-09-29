use bevy::{
    prelude::IVec2,
    utils::{hashbrown::HashMap, HashSet},
};
use crossbeam::queue::SegQueue;
use grid_wfc::{
    graph_grid::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
    world::{ChunkState, World},
};
use hierarchical_wfc::{
    wfc_backend::{self, Backend},
    wfc_task, TileSet, WaveFunction, WfcTask,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::{path::Path, sync::Arc};

// https://en.wikipedia.org/wiki/Standard_deviation#Rapid_calculation_methods
#[derive(Default)]
struct RollingStdErr<T> {
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
    fn insert(&mut self, value: f64) {
        self.s_1 += value;
        self.s_2 += value * value;
        self.n += 1;
    }
    fn avg_manual_sample_count(&self, n: usize) -> StdErr<f64> {
        assert!(n > 0);

        let avg = self.s_1 / n as f64;
        let sigma = (n as f64 * self.s_2 - self.s_1 * self.s_1).sqrt() / n as f64;
        let std_err = sigma / (n as f64).sqrt();
        StdErr::<f64> { n: avg, s: std_err }
    }
    #[allow(dead_code)]
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
const SAMPLES: u64 = 512;
type Key = [usize; 5];
pub fn main() {
    let tileset = Arc::new(
        MxgmnTileset::new(Path::new("assets/mxgmn/Circuit.xml"), None)
            .ok()
            .unwrap(),
    );

    // let tileset = Arc::new(CarcassonneTileset::default());

    let output = Arc::new(SegQueue::new());
    let mut single_threade_backend = wfc_backend::SingleThreaded::new(output.clone());
    let mut multi_threaded_backend = wfc_backend::MultiThreaded::new(output.clone(), THREADS);

    let size = 64;
    let settings = GridGraphSettings {
        height: size,
        width: size,
        periodic: false,
    };
    let a_sparse_vec: HashMap<Key, StdErr<f64>> = {
        let mut valid_samples: usize = 0;
        let mut rolling_std_err: HashMap<[usize; 5], RollingStdErr<f64>> = HashMap::new();
        'skip: for seed in 0..SAMPLES {
            dbg!(seed);
            let mut frequnecy: HashMap<[usize; 5], usize> = HashMap::new();
            let graph = graph_grid::create(&settings, WaveFunction::filled(tileset.tile_count()));
            let task = WfcTask {
                graph,
                tileset: tileset.clone(),
                seed,
                metadata: None,
                backtracking: wfc_task::BacktrackingSettings::default(),
            };

            single_threade_backend.queue_task(task).unwrap();
            'outer: loop {
                match output.pop() {
                    Some(Ok(result)) => {
                        let result = result.graph.validate().unwrap();
                        for (tile, neigbours) in result.tiles.iter().zip(result.neighbors.iter()) {
                            let mut tiles = [None; 5];
                            tiles[0] = Some(*tile);
                            for neighbour in neigbours {
                                tiles[neighbour.direction + 1] =
                                    Some(result.tiles[neighbour.index]);
                            }
                            if tiles.map(|t| t.is_some()).contains(&false) {
                                continue;
                            }
                            let tiles: [usize; 5] = tiles.map(|n| n.unwrap());
                            {
                                let value = frequnecy.get(&tiles).unwrap_or(&0).clone();
                                frequnecy.insert(tiles, value + 1); //.entry(tiles).insert(value + 1);
                            }
                        }
                        break 'outer;
                    }
                    Some(Err(_)) => {
                        continue 'skip;
                    }
                    _ => (),
                }
            }

            for (k, v) in frequnecy {
                rolling_std_err.entry(k).or_default().insert(v as f64);
            }
            valid_samples += 1;
        }
        rolling_std_err
            .into_iter()
            .map(|(k, v)| (k, v.avg_manual_sample_count(valid_samples)))
            .collect()
    };
    let b_sparse_vec: HashMap<Key, StdErr<f64>> = {
        let chunked_generator = get_chunked_generator(tileset.clone(), output.clone());

        let mut rolling_std_err: HashMap<[usize; 5], RollingStdErr<f64>> = HashMap::new();
        let mut valid_samples: usize = 0;
        for seed in 0..SAMPLES {
            dbg!(seed);
            let mut frequnecy: HashMap<[usize; 5], usize> = HashMap::new();
            let result = chunked_generator(seed, 16, &settings, &mut multi_threaded_backend);
            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    dbg!(e);
                    continue;
                }
            };

            for j in 1..result.len() - 1 {
                for i in 1..result.first().unwrap().len() - 1 {
                    let directions = [
                        IVec2::new(0, 0),
                        IVec2::new(0, 1),
                        IVec2::new(0, -1),
                        IVec2::new(-1, 0),
                        IVec2::new(1, 0),
                    ];

                    let tiles = directions.map(|delta| {
                        result[j + delta.x as usize][i + delta.y as usize]
                            .collapse()
                            .unwrap()
                    });

                    {
                        let value = *frequnecy.get(&tiles).unwrap_or(&0);
                        frequnecy.insert(tiles, value + 1);
                    }
                }
            }
            for (k, v) in frequnecy {
                rolling_std_err.entry(k).or_default().insert(v as f64);
            }
            valid_samples += 1;
        }
        rolling_std_err
            .into_iter()
            .map(|(k, v)| (k, v.avg_manual_sample_count(valid_samples)))
            .collect()
    };

    let a_keys: HashSet<_> = a_sparse_vec.keys().collect();
    let b_keys: HashSet<_> = b_sparse_vec.keys().collect();
    let overlap = a_keys.intersection(&b_keys);

    let mut results = overlap
        .clone()
        .map(|k| {
            (
                **k,
                a_sparse_vec.get(*k).unwrap(),
                b_sparse_vec.get(*k).unwrap(),
            )
        })
        .collect::<Vec<_>>();

    results.sort_by(|a, b| (a.1.n.max(a.2.n)).total_cmp(&(b.1.n.max(b.2.n))));

    let mut t_tests: Vec<f64> = vec![];
    for key in overlap.into_iter() {
        let a = a_sparse_vec.get(*key).unwrap();
        let b = b_sparse_vec.get(*key).unwrap();
        if a.n < 5.0 || b.n < 5.0 {
            continue;
        }

        let t = a.t_test(b);
        println!("{a} t {b} = {t}");
        t_tests.push(t);
    }

    let n = t_tests.len() as f64;
    let avg = t_tests.iter().fold(0.0, |acc, next| acc + next.abs() / n);
    print!("Average t test value: {avg}");

    for res in results.iter().rev().take(10) {
        let (key, a, b) = res;
        println!("{a} {b} for {key:?}");
    }

    let sum_a = a_sparse_vec
        .values()
        .map(|v| v.n.clone())
        .reduce(|a, b| a + b)
        .unwrap();

    let sum_b = b_sparse_vec
        .values()
        .map(|v| v.n.clone())
        .reduce(|a, b| a + b)
        .unwrap();

    dbg!((sum_a, sum_b));

    // dbg!(a_sparse_vec.len());
    // dbg!(max_a.and_then(|max| Some((max.0, max.1.n))).unwrap());
    // dbg!(a_sparse_vec.len());
    // dbg!(max_b.and_then(|max| Some((max.0, max.1.n))).unwrap());

    // let mut values = rolling_std_err
    //     .values()
    //     .map(|v| v.avg())
    //     .map(|(v, sigma)| (v.ln(), ((v - sigma).ln(), (v + sigma).ln())))
    //     .collect::<Vec<_>>();
    // values.sort_by(|a, b| PartialOrd::partial_cmp(&b.0, &a.0).unwrap());

    // let x_data = (0..values.len()).map(|v| (v as f64)).collect::<Vec<_>>();

    // let x_min = *x_data.iter().min_by(|a, b| a.total_cmp(b)).unwrap();
    // let x_max = *x_data.iter().min_by(|a, b| a.total_cmp(b)).unwrap();

    // let y_max = values
    //     .iter()
    //     .map(|v| v.1 .1)
    //     .max_by(|a, b| a.total_cmp(b))
    //     .unwrap();
    // let y_min = values
    //     .iter()
    //     .map(|v| v.1 .0)
    //     .min_by(|a, b| a.total_cmp(b))
    //     .unwrap();

    // let root_drawing_area = BitMapBackend::new("plots/plot.png", (1024, 768)).into_drawing_area();
    // root_drawing_area.fill(&WHITE).unwrap();

    // let mut chart = ChartBuilder::on(&root_drawing_area)
    //     .build_cartesian_2d(0..values.len(), y_min..y_max)
    //     .unwrap();

    // chart
    //     .draw_series(LineSeries::new(
    //         values.iter().enumerate().map(|(x, y)| (x, y.0)),
    //         &RED,
    //     ))
    //     .unwrap();
    // chart
    //     .draw_series(LineSeries::new(
    //         values.iter().enumerate().map(|(x, y)| (x, y.1 .0)),
    //         &BLACK,
    //     ))
    //     .unwrap();
    // chart
    //     .draw_series(LineSeries::new(
    //         values.iter().enumerate().map(|(x, y)| (x, y.1 .1)),
    //         &BLACK,
    //     ))
    //     .unwrap();
}

fn get_chunked_generator(
    tileset: Arc<dyn TileSet>,
    output: Arc<SegQueue<anyhow::Result<WfcTask>>>,
) -> impl Fn(
    u64,
    usize,
    &GridGraphSettings,
    &mut dyn wfc_backend::Backend,
) -> anyhow::Result<Vec<Vec<WaveFunction>>> {
    move |seed: u64,
          chunk_size: usize,
          settings: &GridGraphSettings,
          backend: &mut dyn wfc_backend::Backend| {
        let mut rng = SmallRng::seed_from_u64(seed);
        let chunks = IVec2::new(
            settings.width as i32 / chunk_size as i32,
            settings.height as i32 / chunk_size as i32,
        )
        .max(IVec2::ONE);
        let start_chunk = IVec2::new(rng.gen_range(0..chunks.x), rng.gen_range(0..chunks.y));

        let filled = WaveFunction::filled(tileset.tile_count());
        let mut world = World {
            world: vec![vec![filled; settings.height]; settings.width],
            generated_chunks: HashMap::from_iter(vec![(start_chunk, ChunkState::Scheduled)]),
            chunk_size,
            overlap: 1,
            seed,
            tileset: tileset.clone(),
            outstanding: 0,
        };
        world.start_generation(start_chunk, backend, Some(Box::new(start_chunk)));

        // process output
        let chunk_count = (chunks.x * chunks.y) as usize;
        'outer: loop {
            match output.pop() {
                Some(Ok(task)) => {
                    let chunk = task.metadata.as_ref().unwrap().downcast_ref().unwrap();

                    world.process_chunk(
                        *chunk,
                        task,
                        backend,
                        Box::new(|chunk| Some(Box::new(chunk))),
                    );
                }
                Some(Err(e)) => {
                    drain_output(output.clone(), world.outstanding);
                    return Err(e);
                }
                _ => (),
            }

            if world.generated_chunks.len() >= chunk_count {
                for (_, state) in world.generated_chunks.iter() {
                    if *state != ChunkState::Done {
                        continue 'outer;
                    }
                }

                break;
            }
        }
        Ok(world.world)
    }
}

fn drain_output(output: Arc<SegQueue<anyhow::Result<WfcTask>>>, remaining: usize) {
    for _ in 0..remaining {
        while output.pop().is_none() {}
    }
}
