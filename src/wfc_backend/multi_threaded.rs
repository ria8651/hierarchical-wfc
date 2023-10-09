use super::{Backend, SingleThreaded};
use crate::wfc_task::WfcTask;
use anyhow::Result;
use crossbeam::{
    channel::{self, Receiver, Sender},
    queue::SegQueue,
};
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
};

pub struct MultiThreaded {
    queue: Arc<SegQueue<WfcTask>>,
    update_channel: Sender<()>,
    output: Receiver<(WfcTask, Result<()>)>,
    threads: Vec<JoinHandle<()>>,
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
}

impl MultiThreaded {
    pub fn new(num_threads: usize) -> Self {
        let queue = Arc::new(SegQueue::new());
        let (tx, rx) = channel::unbounded();
        let (output_tx, output_rx) = channel::unbounded();

        let mut threads = vec![];
        for _ in 0..num_threads {
            let queue = queue.clone();
            let rx = rx.clone();
            let output_tx = output_tx.clone();

            threads.push(
                thread::Builder::new()
                    .name("WFC multi threaded CPU backend".to_string())
                    .spawn(move || {
                        while let Ok(()) = rx.recv() {
                            if let Some(mut task) = queue.pop() {
                                let task_result = SingleThreaded::execute(&mut task);
                                output_tx.send((task, task_result)).unwrap();
                            }
                        }
                    })
                    .unwrap(),
            );
        }

        Self {
            queue,
            update_channel: tx,
            output: output_rx,
            threads,
        }
    }
}

impl Drop for MultiThreaded {
    fn drop(&mut self) {
        // let mut threads = vec![];
        // std::mem::swap(&mut threads, &mut self.threads);

        // for thread in threads {
        //     thread.join().unwrap();
        // }
    }
}
