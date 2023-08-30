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

        thread::spawn(move || {
            while let Ok(mut peasant) = rx.recv() {
                Self::execute(&mut peasant);
                output.push(peasant);
            }
        });

        Self { queue: tx }
    }

    fn execute(peasant: &mut Peasant) {
        let mut rng = SmallRng::seed_from_u64(peasant.seed);

        let mut stack = Vec::new();
        while let Some(cell) = peasant.lowest_entropy(&mut rng) {
            // collapse cell
            peasant.graph.tiles[cell].select_random(&mut rng, &peasant.weights);

            // propagate changes
            stack.push(cell);
            while let Some(index) = stack.pop() {
                for i in 0..peasant.graph.neighbors[index].len() {
                    // propagate changes
                    let neighbor = peasant.graph.neighbors[index][i];
                    if peasant.propagate(index, neighbor) {
                        stack.push(neighbor.index);
                    }
                }
            }
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

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum Direction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

impl Direction {
    pub fn other(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn rotate(&self, rotation: usize) -> Self {
        match rotation {
            0 => *self,
            1 => match self {
                Self::Up => Self::Right,
                Self::Down => Self::Left,
                Self::Left => Self::Up,
                Self::Right => Self::Down,
            },
            2 => match self {
                Self::Up => Self::Down,
                Self::Down => Self::Up,
                Self::Left => Self::Right,
                Self::Right => Self::Left,
            },
            3 => match self {
                Self::Up => Self::Left,
                Self::Down => Self::Right,
                Self::Left => Self::Down,
                Self::Right => Self::Up,
            },
            _ => panic!("Invalid rotation: {}", rotation),
        }
    }
}

impl From<usize> for Direction {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            _ => panic!("Invalid direction: {}", value),
        }
    }
}
