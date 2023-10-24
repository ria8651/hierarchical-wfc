use super::Backend;
use crate::{
    wfc_task::{BacktrackingHeuristic, BacktrackingSettings},
    WaveFunction, WfcTask,
};
use anyhow::{anyhow, Result};
use bevy::utils::Instant;
use crossbeam::channel::{self, Receiver, Sender};
use rand::{rngs::SmallRng, SeedableRng};
use std::thread;

pub struct SingleThreaded {
    queue: Sender<WfcTask>,
    output: Receiver<(WfcTask, Result<()>)>,
}

impl Backend for SingleThreaded {
    fn queue_task(&mut self, task: WfcTask) -> Result<()> {
        self.queue.send(task)?;

        Ok(())
    }

    fn get_output(&mut self) -> Option<(WfcTask, Result<()>)> {
        self.output.try_recv().ok()
    }

    fn wait_for_output(&mut self) -> (WfcTask, Result<()>) {
        self.output.recv().unwrap()
    }

    fn clear(&mut self) {
        *self = Self::new();
    }
}

impl SingleThreaded {
    pub fn new() -> Self {
        let (tx, rx) = channel::unbounded();
        let (output_tx, output_rx) = channel::unbounded();

        thread::Builder::new()
            .name("WFC CPU backend".to_string())
            .spawn(move || {
                while let Ok(mut task) = rx.recv() {
                    let task_result = Self::execute(&mut task);
                    if let Err(_) = output_tx.send((task, task_result)) {
                        // channel is closed, stop execution
                        return;
                    }
                }
            })
            .unwrap();

        Self {
            queue: tx,
            output: output_rx,
        }
    }

    pub fn execute(task: &mut WfcTask) -> Result<()> {
        let mut rng = SmallRng::seed_from_u64(task.seed);
        let weights = task.tileset.get_weights();

        let mut time = Instant::now();

        // store initial state of all cells already constrained
        let mut history = Vec::new();

        let mut initial = true;
        let mut stack: Vec<usize> = (0..task.graph.tiles.len()).collect();
        loop {
            // propagate changes
            while let Some(index) = stack.pop() {
                for i in 0..task.graph.neighbors[index].len() {
                    // propagate changes
                    let neighbor = task.graph.neighbors[index][i];
                    if task.propagate(index, neighbor) {
                        stack.push(neighbor.index);

                        let bits = task.graph.tiles[neighbor.index].count_bits();
                        if bits == 1 && task.settings.backtracking != BacktrackingSettings::Disabled
                        {
                            history.push((neighbor.index, WaveFunction::empty()));
                        }
                        if bits == 0 {
                            if initial {
                                return Err(anyhow!("Invalid initial state"));
                            }

                            if task.settings.backtracking == BacktrackingSettings::Disabled {
                                return Err(anyhow!("Contradiction found"));
                            }

                            // contradiction found
                            stack = Self::backtrack(&mut history, task).unwrap();
                            break;
                        }
                    }
                }
            }

            initial = false;

            if let Some(cell) = task.lowest_entropy(&mut rng) {
                let mut options = task.graph.tiles[cell].clone();

                // collapse cell
                task.graph.tiles[cell]
                    .select_random(&mut rng, &weights)
                    .unwrap();
                stack.push(cell);

                // if we backtrack to this cell the option we just selected will be removed
                options = WaveFunction::difference(&options, &task.graph.tiles[cell]);
                if task.settings.backtracking != BacktrackingSettings::Disabled {
                    history.push((cell, options));
                }
            } else {
                // all cells collapsed
                return Ok(());
            }

            if let Some(update_interval) = task.settings.progress_updates {
                if time.elapsed().as_secs_f64() > update_interval {
                    time = Instant::now();
                    let update_channel = task.update_channel.as_ref().expect("No update channel");
                    if let Err(e) = update_channel.send((task.graph.clone(), task.metadata.clone()))
                    {
                        // channel is closed, stop execution
                        return Err(anyhow!("Update channel closed: {}", e));
                    }
                }
            }
        }
    }

    fn backtrack(
        history: &mut Vec<(usize, WaveFunction)>,
        task: &mut WfcTask,
    ) -> Result<Vec<usize>> {
        if history.is_empty() {
            return Err(anyhow!("No history found when backtracking"));
        }

        // unconstrain all tiles which are not fully collapsed
        let filled = WaveFunction::filled(task.tileset.tile_count());
        for i in 0..task.graph.tiles.len() {
            if task.graph.tiles[i].count_bits() != 1 {
                task.graph.tiles[i] = filled.clone();
            }
        }

        let heuristic = match &task.settings.backtracking {
            BacktrackingSettings::Disabled => return Err(anyhow!("Backtracking disabled")),
            BacktrackingSettings::Enabled { heuristic, .. } => heuristic,
        };

        // decide how many steps to backtrack based on the heuristic
        let mut steps = match heuristic {
            BacktrackingHeuristic::Standard => 0,
            BacktrackingHeuristic::Fixed { distance } => *distance,
            BacktrackingHeuristic::Proportional { proportion } => {
                (history.len() as f32 * proportion) as usize
            }
            BacktrackingHeuristic::Degree { degree } => {
                let mut steps = history.len();
                for (index, (_, options)) in history.iter().rev().enumerate() {
                    if options.count_bits() >= *degree {
                        steps = index;
                        break;
                    }
                }
                steps
            }
        };

        // step back till we find a cell with more than one option
        loop {
            let (index, options) = history
                .pop()
                .ok_or(anyhow!("Ran out of options when backtracking"))?;

            // clear cell
            task.graph.tiles[index] = filled.clone();

            if history.is_empty() {
                // we have backtracked to the initial state, this is a random restart
                break;
            }

            // if we have more than one option we can stop backtracking
            if options.count_bits() > 0 && steps == 0 {
                // this is to allow the cell to be backtracked past again
                if options.count_bits() == 1 {
                    history.push((index, WaveFunction::empty()));
                }
                task.graph.tiles[index] = options;
                break;
            }

            if steps > 0 {
                steps -= 1;
            }
        }

        // re-propagate changes
        return Ok((0..task.graph.tiles.len())
            .filter(|i| task.graph.tiles[*i].count_bits() < task.tileset.tile_count())
            .collect());
    }
}
