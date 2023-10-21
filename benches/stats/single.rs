use grid_wfc::grid_graph::{self, GridGraphSettings};
use core_wfc::{
    wfc_backend::Backend, wfc_task::WfcSettings, Graph, TileSet, WaveFunction, WfcTask,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

use crate::stats_builder::StatisticRunner;

pub struct SingleSettings {
    pub size: usize,
    pub wfc_settings: WfcSettings,
    pub grid_graph_settings: GridGraphSettings,
}

pub struct SingleRunner {
    pub tileset: Arc<dyn TileSet>,
    pub backend: Rc<RefCell<dyn Backend>>,
    pub settings: SingleSettings,
}

impl StatisticRunner for SingleRunner {
    fn queue(&mut self, seed: u64) {
        let filled = WaveFunction::filled(self.tileset.tile_count());
        let graph = grid_graph::create(&self.settings.grid_graph_settings, filled);

        let task = WfcTask {
            graph,
            tileset: self.tileset.clone(),
            seed,
            metadata: Some(Arc::new(SingleRunnerTag)),
            settings: self.settings.wfc_settings.clone(),
        };
        self.backend.borrow_mut().queue_task(task).unwrap();
    }

    fn next_result(&mut self) -> Result<Graph<usize>, anyhow::Error> {
        loop {
            let (task, status) = self
                .backend
                .borrow_mut()
                .wait_for_output()
                .try_into()
                .unwrap();

            if let Some(metadata) = task.metadata.as_ref() {
                match metadata.downcast_ref() {
                    Some(&SingleRunnerTag {}) => {
                        status?;
                        let result = task.graph.validate()?;
                        return Ok(result);
                    }
                    _ => {}
                }
            }
        }
    }
}

struct SingleRunnerTag;
