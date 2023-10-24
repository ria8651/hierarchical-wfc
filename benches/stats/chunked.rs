use core_wfc::{wfc_backend::Backend, wfc_task::WfcSettings, Graph, TileSet};
use grid_wfc::{
    grid_graph::GridGraphSettings,
    single_shot,
    world::{ChunkSettings, GenerationMode},
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

use crate::stats_builder::StatisticRunner;

pub struct ChunkedSettings {
    pub generation_mode: GenerationMode,
    pub grid_graph_settings: GridGraphSettings,
    pub chunk_settings: ChunkSettings,
    pub wfc_settings: WfcSettings,
}

pub struct ChunkedRunner {
    pub seeds: Vec<u64>,
    pub tileset: Arc<dyn TileSet>,
    pub backend: Rc<RefCell<dyn Backend>>,
    pub setings: ChunkedSettings,
}

impl StatisticRunner for ChunkedRunner {
    fn queue(&mut self, seed: u64) {
        self.seeds.push(seed)
    }

    fn next_result(&mut self) -> Result<Graph<usize>, anyhow::Error> {
        let (world, _) = single_shot::generate_world(
            self.tileset.clone(),
            &mut *self.backend.borrow_mut(),
            self.setings.grid_graph_settings.clone(),
            self.seeds.pop().unwrap(),
            self.setings.generation_mode,
            self.setings.chunk_settings,
            self.setings.wfc_settings.clone(),
        );
        world.build_world_graph()
    }
}
