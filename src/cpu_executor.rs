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

                            if Self::backtrack(&mut history, &mut peasant) {
                            } else {
                                // If there's no collapsed cell in the history,
                                // there's an unsolvable configuration.
                                return;
                            }
                        }
                    }
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
                    index: history.stack.len(),
                    options,
                });
            } else {
                // all cells collapsed
                return;
            }
        }
    }

    fn backtrack(history: &mut History, peasant: &mut Peasant) -> bool {
        // Backtrack to most recent collapsed cell
        if history.collapsed_cells.is_empty() {
            return false;
        }
        let mut collapsed = history.collapsed_cells.pop().unwrap();

        // Backtrack further if needed
        while collapsed.options.count_bits() == 0 {
            if history.collapsed_cells.is_empty() {
                return false;
            }
            collapsed = history.collapsed_cells.pop().unwrap();
        }

        // Restore state until the most recent collapsed cell
        while history.stack.len() > collapsed.index {
            let index = history.stack.pop().unwrap();
            peasant.graph.tiles[index] = WaveFunction::filled(peasant.tile_count);
        }
        // Unconstrain all tiles which are not fully collapsed
        for i in 0..peasant.graph.tiles.len() {
            if peasant.graph.tiles[i].count_bits() > 1 {
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
                    if peasant.graph.tiles[neighbor.index].count_bits() <= 1 {
                        panic!("Contradiction found or tile set while backtracking this should never happen{}", peasant.graph.tiles[neighbor.index].count_bits());
                    }
                }
            }
        }

        // Restore state of the most recent collapsed cell
        let collapsed_index = history.stack.pop().unwrap();
        peasant.graph.tiles[collapsed_index] = collapsed.options;
        true
    }
}

impl Executor for CpuExecutor {
    fn queue_peasant(&mut self, peasant: Peasant) -> Result<()> {
        self.queue.send(peasant)?;

        Ok(())
    }
}
