use crate::{Executor, Peasant};
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

    pub fn execute(peasant: &mut Peasant) {
        let mut rng = SmallRng::seed_from_u64(peasant.seed);

        let mut stack: Vec<usize> = (0..peasant.graph.tiles.len()).collect();
        loop {
            // propagate changes
            while let Some(index) = stack.pop() {
                for i in 0..peasant.graph.neighbors[index].len() {
                    // propagate changes
                    let neighbor = peasant.graph.neighbors[index][i];
                    if peasant.propagate(index, neighbor) {
                        stack.push(neighbor.index);
                    }
                    if peasant.graph.tiles[neighbor.index].count_bits() == 0 {
                        // contradiction found
                        return;
                    }
                }
            }

            if let Some(cell) = peasant.lowest_entropy(&mut rng) {
                // collapse cell
                peasant.graph.tiles[cell].select_random(&mut rng, &peasant.weights).unwrap();
                stack.push(cell);
            } else {
                // all cells collapsed
                return;
            }

            // // propagate changes
            // while let Some(index) = stack.pop() {
            //     for i in 0..peasant.graph.neighbors[index].len() {
            //         // propagate changes
            //         let neighbor = peasant.graph.neighbors[index][i];
            //         if peasant.propagate(index, neighbor) {
            //             stack.push(neighbor.index);
            //         }
            //         if peasant.graph.tiles[neighbor.index].count_bits() == 0 {
            //             // contradiction found
            //             return;
            //         }
            //     }
            // }

            // break;
        }
    }
}

impl Executor for CpuExecutor {
    fn queue_peasant(&mut self, peasant: Peasant) -> Result<()> {
        self.queue.send(peasant)?;

        Ok(())
    }
}
