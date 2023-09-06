use crate::{CpuExecutor, Executor, Peasant};
use anyhow::Result;
use crossbeam::queue::SegQueue;
use std::{sync::Arc, thread};

pub struct MultiThreadedExecutor {
    queue: Arc<SegQueue<Peasant>>,
}

impl MultiThreadedExecutor {
    pub fn new(output: Arc<SegQueue<Peasant>>, num_threads: usize) -> Self {
        let queue = Arc::new(SegQueue::new());

        for _ in 0..num_threads {
            let queue = queue.clone();
            let output = output.clone();

            thread::spawn(move || loop {
                if let Some(mut peasant) = queue.pop() {
                    CpuExecutor::execute(&mut peasant);
                    output.push(peasant);
                }

                thread::yield_now();
            });
        }

        Self { queue }
    }
}

impl Executor for MultiThreadedExecutor {
    fn queue_peasant(&mut self, peasant: Peasant) -> Result<()> {
        self.queue.push(peasant);

        Ok(())
    }
}
