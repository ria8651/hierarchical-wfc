use grid_wfc::{
    graph_grid::GridGraphSettings,
    single_shot,
    world::{ChunkSettings, GenerationMode},
};
use hierarchical_wfc::{
    wfc_backend::Backend,
    wfc_task::{BacktrackingSettings, Entropy, WfcSettings},
    Graph, TileSet,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub struct ChunkedSettings {
    pub generation_mode: GenerationMode,
    pub grid_graph_settings: GridGraphSettings,
    pub chunk_settings: ChunkSettings,
    pub wfc_settings: WfcSettings,
}

pub fn generate_chunked(
    seed: u64,
    tileset: Arc<dyn TileSet>,
    backend: Rc<RefCell<dyn Backend>>,
    setings: ChunkedSettings,
) -> Result<Graph<usize>, anyhow::Error> {
    let (world, _) = single_shot::generate_world(
        tileset.clone(),
        &mut *backend.borrow_mut(),
        setings.grid_graph_settings,
        seed,
        setings.generation_mode,
        setings.chunk_settings,
        setings.wfc_settings,
    );
    world.build_world_graph()
}
