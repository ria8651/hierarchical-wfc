use super::Backend;
use crate::{WaveFunction, WfcTask};
use anyhow::{anyhow, Result};
use crossbeam::{
    channel::{self, Sender},
    queue::SegQueue,
};
use rand::{rngs::SmallRng, SeedableRng};
use std::{sync::Arc, thread};

pub struct SingleThreaded {
    queue: Sender<WfcTask>,
}

struct History {
    stack: Vec<usize>,
    decision_cells: Vec<HistoryCell>,
}

struct HistoryCell {
    index: usize,
    options: WaveFunction,
}

impl SingleThreaded {
    pub fn new(output: Arc<SegQueue<Result<WfcTask>>>) -> Self {
        let (tx, rx) = channel::unbounded::<WfcTask>();

        thread::Builder::new()
            .name("WFC CPU backend".to_string())
            .spawn(move || {
                while let Ok(mut task) = rx.recv() {
                    let task_result = Self::execute(&mut task);
                    match task_result {
                        Err(e) => {
                            dbg!(&e);
                            output.push(Err(e))
                        }
                        Ok(_) => {
                            output.push(Ok(task));
                        }
                    }
                }
            })
            .unwrap();

        Self { queue: tx }
    }

    pub fn execute(task: &mut WfcTask) -> Result<()> {
        let mut rng = SmallRng::seed_from_u64(task.seed);
        let weights = task.tileset.get_weights();
        let tileset = task.tileset.clone();
        let mut attemps_left = task.backtracking.max_restarts;

        let mut initial_state: Vec<HistoryCell> = Vec::new();

        // store initial state of all cells already constrained
        for i in 0..task.graph.tiles.len() {
            if task.graph.tiles[i].count_bits() != tileset.tile_count() {
                initial_state.push(HistoryCell {
                    index: i,
                    options: task.graph.tiles[i].clone(),
                });
            }
        }

        let mut stack: Vec<usize> = (0..task.graph.tiles.len()).collect();
        let mut history = History {
            stack: Vec::new(),
            decision_cells: Vec::new(),
        };
        loop {
            let mut backtrack_flag = false;
            // propagate changes
            while let Some(index) = stack.pop() {
                for i in 0..task.graph.neighbors[index].len() {
                    // propagate changes
                    let neighbor = task.graph.neighbors[index][i];
                    if task.propagate(index, neighbor) {
                        stack.push(neighbor.index);
                        if task.graph.tiles[neighbor.index].count_bits() == 1 {
                            history.stack.push(neighbor.index);
                        }
                        if task.graph.tiles[neighbor.index].count_bits() == 0 {
                            // contradiction found
                            backtrack_flag = true;
                            break;
                        }
                    }
                }
                if backtrack_flag {
                    let result = Self::backtrack(&mut history, task, &mut rng);
                    if let Ok(continue_from) = result {
                        stack.clear();
                        stack.push(continue_from);
                    } else {
                        // If there's no collapsed cell in the history,
                        // there's an unsolvable configuration.
                        // Perform a random restart.
                        attemps_left -= 1;
                        if attemps_left == 0 {
                            task.graph.tiles.fill(WaveFunction::empty());
                            let seed = task.seed;
                            let restarts = task.backtracking.max_restarts;
                            return Err(anyhow!(
                                "Backtracking exceeded {restarts} on seed {seed:x}"
                            ));
                        }
                        task.clear();
                        for cell in initial_state.iter() {
                            task.graph.tiles[cell.index] = cell.options.clone();
                        }
                        history = History {
                            stack: Vec::new(),
                            decision_cells: Vec::new(),
                        };
                        stack = (0..task.graph.tiles.len()).collect();
                    }
                    backtrack_flag = false;
                    continue;
                }
            }

            if let Some(cell) = task.lowest_entropy(&mut rng) {
                let mut options = task.graph.tiles[cell].clone();
                // collapse cell
                task.graph.tiles[cell]
                    .select_random(&mut rng, &weights)
                    .unwrap();
                stack.push(cell);
                options = WaveFunction::difference(&options, &task.graph.tiles[cell]);
                history.stack.push(cell);
                history.decision_cells.push(HistoryCell {
                    index: history.stack.len() - 1,
                    options,
                });
            } else {
                // all cells collapsed
                return Ok(());
            }
        }
    }

    fn backtrack(
        history: &mut History,
        task: &mut WfcTask,
        mut rng: &mut SmallRng,
    ) -> Result<usize, String> {
        // Backtrack to most recent collapsed cell
        if history.decision_cells.is_empty() {
            Err("No collapsed cells in history")?;
        }

        let mut collapsed = history.decision_cells.pop().unwrap();
        // Backtrack further, we skip cells with less than 3 options as this optimization provides great speedup, TODO: make this configurable
        if collapsed.options.count_bits() == 0 {
            while collapsed.options.count_bits() < 3 {
                if history.decision_cells.is_empty() {
                    Err("No collapsed cells in history")?;
                }
                collapsed = history.decision_cells.pop().unwrap();
            }
        }

        // Restore state until the most recent collapsed cell
        let tileset = task.tileset.clone();
        let filled = WaveFunction::filled(tileset.tile_count());
        while history.stack.len() > collapsed.index + 1 {
            let index = history.stack.pop().unwrap();
            task.graph.tiles[index] = filled.clone();
        }
        let collapsed_index = history.stack.pop().unwrap();
        task.graph.tiles[collapsed_index] = filled.clone();

        // Unconstrain all tiles which are not fully collapsed
        for i in 0..task.graph.tiles.len() {
            if task.graph.tiles[i].count_bits() != 1 {
                task.graph.tiles[i] = filled.clone();
            }
        }

        // Reconstrain tiles
        let mut stack: Vec<usize> = (0..task.graph.tiles.len()).collect();
        while let Some(index) = stack.pop() {
            for i in 0..task.graph.neighbors[index].len() {
                // propagate changes
                let neighbor = task.graph.neighbors[index][i];
                if task.propagate(index, neighbor) {
                    stack.push(neighbor.index);
                    if task.graph.tiles[neighbor.index].count_bits() == 0 {
                        panic!(
                            "Contradiction found while backtracking, this should never happen{}",
                            collapsed_index
                        );
                    }
                }
            }
        }

        // Restore state of the most recent collapsed cell
        task.graph.tiles[collapsed_index] = collapsed.options.clone();
        let mut options = collapsed.options.clone();
        // collapse cell
        task.graph.tiles[collapsed_index]
            .select_random(&mut rng, &tileset.get_weights())
            .unwrap();
        options = WaveFunction::difference(&options, &task.graph.tiles[collapsed_index]);
        history.stack.push(collapsed_index);
        history.decision_cells.push(HistoryCell {
            index: history.stack.len() - 1,
            options,
        });
        Ok(collapsed_index)
    }
}

impl Backend for SingleThreaded {
    fn queue_task(&mut self, task: WfcTask) -> Result<()> {
        self.queue.send(task)?;

        Ok(())
    }
}