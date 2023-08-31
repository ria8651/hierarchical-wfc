use crate::{Executer, Peasant};
use anyhow::Result;
use crossbeam::{
    channel::{self, Sender},
    queue::SegQueue,
};
use rand::{rngs::SmallRng, SeedableRng};
use std::{sync::Arc, thread};

pub struct CpuExecuter {
    queue: Sender<Peasant>,
}

impl CpuExecuter {
    pub fn new(output: Arc<SegQueue<Peasant>>) -> Self {
        let (tx, rx) = channel::unbounded();

        thread::Builder::new()
            .name("CPU WFC Executer".to_string())
            .spawn(move || {
                while let Ok(mut peasant) = rx.recv() {
                    Self::execute(&mut peasant);
                    output.push(peasant);
                }
            })
            .unwrap();

        Self { queue: tx }
    }

    fn execute(peasant: &mut Peasant) {
        let mut rng = SmallRng::seed_from_u64(peasant.seed);

        let mut stack: Vec<usize> = (0..peasant.graph.tiles.len()).collect();
        while let Some(cell) = peasant.lowest_entropy(&mut rng) {
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

            // collapse cell
            peasant.graph.tiles[cell].select_random(&mut rng, &peasant.weights);
            stack.push(cell);
        }

        // for y in (0..grid_wfc.grid[0].len()).rev() {
        //     for x in 0..grid_wfc.grid.len() {
        //         let tiles = &grid_wfc.grid[x][y];
        //         print!("{:<22}", format!("{:?}", tiles));
        //     }
        //     println!();
        // }
    }
}

impl Executer for CpuExecuter {
    fn queue_peasant(&mut self, peasant: Peasant) -> Result<()> {
        self.queue.send(peasant)?;

        Ok(())
    }
}
