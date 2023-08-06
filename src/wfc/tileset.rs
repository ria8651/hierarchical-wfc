use super::{Graph, Superposition};

pub trait TileSet {
    type GraphSettings;

    fn tile_count(&self) -> usize;
    fn arc_types(&self) -> usize;
    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Superposition>;
    fn get_constraints(&self) -> Vec<Vec<Superposition>>;
    fn get_weights(&self) -> Vec<u32>;
    fn get_tile_paths(&self) -> Vec<String>;
}
