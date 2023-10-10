use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use anyhow::Error;
use hierarchical_wfc::Graph;

use crate::tile_util::Tile;

use super::std_err::{RollingStdErr, StdErr};

pub trait SparseDistribution<K> {
    fn reasonable_keys(&self) -> HashSet<K>;
    fn compare(&self, other: &Self) -> ();
}

impl<K: Eq + Hash + Clone> SparseDistribution<K> for HashMap<K, StdErr<f64>> {
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

pub struct RunStatistics {
    pub single: HashMap<usize, StdErr<f64>>,
    pub pair: HashMap<[usize; 3], StdErr<f64>>,
    pub quad: HashMap<[usize; 4], StdErr<f64>>,
    pub neighbours: HashMap<[usize; 5], StdErr<f64>>,
}

pub struct RunStatisticsBuilder {
    seed: u64,
    samples: usize,
    queue_fn: Box<dyn Fn(u64)>,
    await_fn: Box<dyn Fn(u64) -> Result<Graph<usize>, Error>>,
    distributions_single: HashMap<usize, RollingStdErr<f64>>,
    distributions_pair: HashMap<[usize; 3], RollingStdErr<f64>>,
    distributions_quad: HashMap<[usize; 4], RollingStdErr<f64>>,
    distributions_neighbours: HashMap<[usize; 5], RollingStdErr<f64>>,
}
impl RunStatisticsBuilder {
    pub fn new(
        samples: usize,
        queue_fn: Box<dyn Fn(u64)>,
        await_fn: Box<dyn Fn(u64) -> Result<Graph<usize>, Error>>,
    ) -> Self {
        Self {
            samples,
            queue_fn,
            await_fn,
            seed: 0u64,
            distributions_single: HashMap::new(),
            distributions_pair: HashMap::new(),
            distributions_quad: HashMap::new(),
            distributions_neighbours: HashMap::new(),
        }
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
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
                let tile_1 = tile_0.tile_in_dir(&graph, 2);
                let tile_2 = tile_0.tile_in_dir(&graph, 0);
                let tile_3 = tile_1.as_ref().and_then(|t| t.tile_in_dir(&graph, 0));

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
                let neigbhours =
                    [0, 1, 2, 3].map(|d| tile_0.tile_in_dir(&graph, d).and_then(|t| Some(t.value)));

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

    pub fn run(&mut self) {
        let mut required_samples = self.samples;
        let mut remaning_samples = self.samples;
        while remaning_samples > 0 {
            while required_samples > 0 {
                required_samples -= 1;
                (self.queue_fn)(self.seed);
                self.seed += 1;
            }

            if let Ok(result) = (self.await_fn)(self.seed) {
                self.update_distrubtions(result);
                remaning_samples -= 1;
            } else {
                required_samples += 1;
            }
        }
    }

    pub fn build(&self) -> RunStatistics {
        for d in self.distributions_single.values() {
            assert!(d.n == 16);
        }

        let distributions = RunStatistics {
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
