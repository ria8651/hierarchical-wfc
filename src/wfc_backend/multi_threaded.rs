use super::{Backend, SingleThreaded};
use crate::wfc_task::WfcTask;
use anyhow::Result;
use crossbeam::{
    channel::{self, Sender},
    queue::SegQueue,
};
use std::{sync::Arc, thread};

pub struct MultiThreaded {
    queue: Arc<SegQueue<WfcTask>>,
    update_channel: Sender<()>,
}

impl MultiThreaded {
    pub fn new(output: Arc<SegQueue<Result<WfcTask>>>, num_threads: usize) -> Self {
        let queue = Arc::new(SegQueue::new());
        let (tx, rx) = channel::unbounded();

        for _ in 0..num_threads {
            let queue = queue.clone();
            let output = output.clone();
            let rx = rx.clone();

            thread::spawn(move || loop {
                while let Ok(()) = rx.recv() {
                    if let Some(mut task) = queue.pop() {
                        if let Err(e) = SingleThreaded::execute(&mut task) {
                            output.push(Err(e));
                        }
                        output.push(Ok(task));
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

impl Backend for MultiThreaded {
    fn queue_task(&mut self, task: WfcTask) -> Result<()> {
        self.queue.push(task);
        self.update_channel.send(())?;

        Ok(())
    }
}
