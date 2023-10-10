use grid_wfc::graph_grid::{self, GridGraphSettings};
use hierarchical_wfc::{
    wfc_backend::SingleThreaded,
    wfc_task::{BacktrackingSettings, Entropy, WfcSettings},
    Graph, TileSet, WaveFunction, WfcTask,
};
use std::sync::Arc;

pub struct SingleSettings {
    pub size: usize,
    pub wfc_settings: WfcSettings,
    pub grid_graph_settings: GridGraphSettings,
}

pub fn generate_single(
    seed: u64,
    tileset: Arc<dyn TileSet>,
    settings: SingleSettings,
) -> Result<Graph<usize>, anyhow::Error> {
    let filled = WaveFunction::filled(tileset.tile_count());
    let graph = graph_grid::create(&settings.grid_graph_settings, filled);

    let mut task = WfcTask {
        graph,
        tileset: tileset.clone(),
        seed,
        metadata: None,
        settings: settings.wfc_settings,
    };

    SingleThreaded::execute(&mut task)?;
    let result = task.graph.validate()?;

    Ok(result)
}
