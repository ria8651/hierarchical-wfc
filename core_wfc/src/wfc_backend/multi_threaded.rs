use super::{Backend, SingleThreaded};
use crate::wfc_task::WfcTask;
use anyhow::Result;
use crossbeam::{
    channel::{self, Receiver, Sender},
    queue::SegQueue,
};
use std::{sync::Arc, thread};

pub struct MultiThreaded {
    num_threads: usize,
    queue: Arc<SegQueue<WfcTask>>,
    update_channel: Sender<()>,
    output: Receiver<(WfcTask, Result<()>)>,
}

impl Backend for MultiThreaded {
    fn queue_task(&mut self, task: WfcTask) -> Result<()> {
        self.queue.push(task);
        self.update_channel.send(())?;

        Ok(())
    }

    fn get_output(&mut self) -> Option<(WfcTask, Result<()>)> {
        self.output.try_recv().ok()
    }

    fn wait_for_output(&mut self) -> (WfcTask, Result<()>) {
        self.output.recv().unwrap()
    }

    fn clear(&mut self) {
        *self = Self::new(self.num_threads);
    }
}

impl MultiThreaded {
    pub fn new(num_threads: usize) -> Self {
        let queue = Arc::new(SegQueue::new());
        let (tx, rx) = channel::unbounded();
        let (output_tx, output_rx) = channel::unbounded();

        for _ in 0..num_threads {
            let queue = queue.clone();
            let rx = rx.clone();
            let output_tx = output_tx.clone();

            thread::Builder::new()
                .name("WFC multi threaded CPU backend".to_string())
                .spawn(move || {
                    while let Ok(()) = rx.recv() {
                        if let Some(mut task) = queue.pop() {
                            let task_result = SingleThreaded::execute(&mut task);
                            if let Err(_) = output_tx.send((task, task_result)) {
                                // channel is closed, stop execution
                                return;
                            }
                        }
                    }
                })
                .unwrap();
        }

        Self {
            num_threads,
            queue,
            update_channel: tx,
            output: output_rx,
        }
    }
}
