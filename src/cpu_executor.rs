use crate::{Executor, Peasant, WaveFunction};
use anyhow::Result;
use crossbeam::{
    channel::{self, Sender},
    queue::SegQueue,
};
use rand::{rngs::SmallRng, SeedableRng};
use std::{sync::Arc, thread};

pub struct CpuExecutor {
    queue: Sender<Peasant>,
}

struct History {
    stack: Vec<usize>,
    collapsed_cells: Vec<CollapsedCell>,
}

struct CollapsedCell {
    index: usize,
    options: WaveFunction,
}

impl CpuExecutor {
    pub fn new(output: Arc<SegQueue<Peasant>>) -> Self {
        let (tx, rx) = channel::unbounded();

        thread::Builder::new()
            .name("CPU WFC Executor".to_string())
            .spawn(move || {
                while let Ok(mut peasant) = rx.recv() {
                    Self::execute(&mut peasant);
                    output.push(peasant);
                }
            })
            .unwrap();

        Self { queue: tx }
    }

    pub fn execute(mut peasant: &mut Peasant) {
        let mut rng = SmallRng::seed_from_u64(peasant.seed);

        let mut stack: Vec<usize> = (0..peasant.graph.tiles.len()).collect();
        let mut history = History {
            stack: Vec::new(),
            collapsed_cells: Vec::new(),
        };
        loop {
            let mut backtrack_flag = false;
            // propagate changes
            while let Some(index) = stack.pop() {
                for i in 0..peasant.graph.neighbors[index].len() {
                    // propagate changes
                    let neighbor = peasant.graph.neighbors[index][i];
                    if peasant.propagate(index, neighbor) {
                        stack.push(neighbor.index);
                        if peasant.graph.tiles[neighbor.index].count_bits() == 1 {
                            history.stack.push(neighbor.index);
                        }
                        if peasant.graph.tiles[neighbor.index].count_bits() == 0 {
                            // contradiction found
                            backtrack_flag = true;
                            break;
                        }
                    }
                }
                if backtrack_flag {
                    let result = Self::backtrack(&mut history, &mut peasant, &mut rng);
                    if let Ok(continue_from) = result {
                        stack.clear();
                        stack.push(continue_from);
                    } else {
                        // If there's no collapsed cell in the history,
                        // there's an unsolvable configuration.
                        // Perform a random restart.
                        peasant.clear();
                        history = History {
                            stack: Vec::new(),
                            collapsed_cells: Vec::new(),
                        };
                        stack = (0..peasant.graph.tiles.len()).collect();
                    }
                    backtrack_flag = false;
                    continue;
                }
            }

            if let Some(cell) = peasant.lowest_entropy(&mut rng) {
                let mut options = peasant.graph.tiles[cell].clone();
                // collapse cell
                peasant.graph.tiles[cell]
                    .select_random(&mut rng, &peasant.weights)
                    .unwrap();
                stack.push(cell);
                options = WaveFunction::difference(&options, &peasant.graph.tiles[cell]);
                history.stack.push(cell);
                history.collapsed_cells.push(CollapsedCell {
                    index: history.stack.len() - 1,
                    options,
                });
            } else {
                // all cells collapsed
                return;
            }
        }
    }

    fn backtrack(
        history: &mut History,
        peasant: &mut Peasant,
        mut rng: &mut SmallRng,
    ) -> Result<usize, String> {
        // Backtrack to most recent collapsed cell
        if history.collapsed_cells.is_empty() {
            Err("No collapsed cells in history")?;
        }

        let mut collapsed = history.collapsed_cells.pop().unwrap();
        // Backtrack further, we skip cells with less than 3 options as this optimization provides great speedup, TODO: make this configurable
        if collapsed.options.count_bits() == 0 {
            while collapsed.options.count_bits() < 3 {
                if history.collapsed_cells.is_empty() {
                    Err("No collapsed cells in history")?;
                }
                collapsed = history.collapsed_cells.pop().unwrap();
            }
        }

        // Restore state until the most recent collapsed cell
        while history.stack.len() > collapsed.index + 1 {
            let index = history.stack.pop().unwrap();
            peasant.graph.tiles[index] = WaveFunction::filled(peasant.tile_count);
        }
        let collapsed_index = history.stack.pop().unwrap();
        peasant.graph.tiles[collapsed_index] = WaveFunction::filled(peasant.tile_count);

        // Unconstrain all tiles which are not fully collapsed
        for i in 0..peasant.graph.tiles.len() {
            if peasant.graph.tiles[i].count_bits() != 1 {
                peasant.graph.tiles[i] = WaveFunction::filled(peasant.tile_count);
            }
        }

        // Reconstrain tiles
        let mut stack: Vec<usize> = (0..peasant.graph.tiles.len()).collect();
        while let Some(index) = stack.pop() {
            for i in 0..peasant.graph.neighbors[index].len() {
                // propagate changes
                let neighbor = peasant.graph.neighbors[index][i];
                if peasant.propagate(index, neighbor) {
                    stack.push(neighbor.index);
                    if peasant.graph.tiles[neighbor.index].count_bits() == 0 {
                        panic!(
                            "Contradiction found while backtracking, this should never happen{}",
                            collapsed_index
                        );
                    }
                }
            }
        }

        // Restore state of the most recent collapsed cell
        peasant.graph.tiles[collapsed_index] = collapsed.options;
        let mut options = collapsed.options.clone();
        // collapse cell
        peasant.graph.tiles[collapsed_index]
            .select_random(&mut rng, &peasant.weights)
            .unwrap();
        options = WaveFunction::difference(&options, &peasant.graph.tiles[collapsed_index]);
        history.stack.push(collapsed_index);
        history.collapsed_cells.push(CollapsedCell {
            index: history.stack.len() - 1,
            options,
        });
        Ok(collapsed_index)
    }
}

impl Executor for CpuExecutor {
    fn queue_peasant(&mut self, peasant: Peasant) -> Result<()> {
        self.queue.send(peasant)?;

        Ok(())
    }
}
