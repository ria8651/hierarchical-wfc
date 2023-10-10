use grid_wfc::graph_grid::{self, GridGraphSettings};
use hierarchical_wfc::{
    wfc_backend::{Backend, SingleThreaded},
    wfc_task::WfcSettings,
    Graph, TileSet, WaveFunction, WfcTask,
};
use std::{cell::RefCell, error::Error, rc::Rc, sync::Arc};

pub struct SingleSettings {
    pub size: usize,
    pub wfc_settings: WfcSettings,
    pub grid_graph_settings: GridGraphSettings,
}

struct SingleData {
    seed: u64,
}

pub fn dispatch_single(
    seed: u64,
    tileset: Arc<dyn TileSet>,
    backend: Rc<RefCell<dyn Backend>>,
    settings: SingleSettings,
) {
    let filled = WaveFunction::filled(tileset.tile_count());
    let graph = graph_grid::create(&settings.grid_graph_settings, filled);

    let task = WfcTask {
        graph,
        tileset: tileset.clone(),
        seed,
        metadata: Some(Arc::new(SingleData { seed })),
        settings: settings.wfc_settings,
    };
    backend.borrow_mut().queue_task(task).unwrap();
}

pub fn await_single(
    backend: Rc<RefCell<dyn Backend>>,
    seed: u64,
) -> Result<Graph<usize>, anyhow::Error> {
    loop {
        let (task, status) = backend.borrow_mut().wait_for_output().try_into().unwrap();

        if let Some(metadata) = task.metadata.as_ref() {
            match metadata.downcast_ref() {
                Some(&SingleData { seed: result_seed }) => {
                    status?;
                    let result = task.graph.validate()?;
                    dbg!(result_seed);
                    return Ok(result);
                }
                _ => {}
            }
        }
    }
}
