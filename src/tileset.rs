use crate::graph::{Cell, Graph};

pub trait TileSet {
    type GraphSettings;

    fn tile_count(&self) -> usize;
    fn directions(&self) -> usize;
    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Cell>;
    fn get_constraints(&self) -> Vec<Vec<Cell>>;
    fn get_weights(&self) -> Vec<u32>;
    fn get_tile_paths(&self) -> Vec<String>;
}
