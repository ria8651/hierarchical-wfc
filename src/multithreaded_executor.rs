use crate::{CpuExecutor, Executor, Peasant};
use anyhow::Result;
use crossbeam::{
    channel::{self, Sender},
    queue::SegQueue,
};
use std::{sync::Arc, thread};

pub struct MultiThreadedExecutor {
    queue: Arc<SegQueue<Peasant>>,
    update_channel: Sender<()>,
}

impl MultiThreadedExecutor {
    pub fn new(output: Arc<SegQueue<Peasant>>, num_threads: usize) -> Self {
        let queue = Arc::new(SegQueue::new());
        let (tx, rx) = channel::unbounded();

        for _ in 0..num_threads {
            let queue = queue.clone();
            let output = output.clone();
            let rx = rx.clone();

            thread::spawn(move || loop {
                while let Ok(()) = rx.recv() {
                    if let Some(mut peasant) = queue.pop() {
                        CpuExecutor::execute(&mut peasant);
                        output.push(peasant);
                    }
                }
            });
        }

        Self {
            queue,
            update_channel: tx,
        }
    }
}

impl Executor for MultiThreadedExecutor {
    fn queue_peasant(&mut self, peasant: Peasant) -> Result<()> {
        self.queue.push(peasant);
        self.update_channel.send(())?;

        Ok(())
    }
}
