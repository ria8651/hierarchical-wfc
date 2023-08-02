use crate::graph::{Cell, Graph};

pub trait TileSet {
    type GraphSettings;

    const TILE_COUNT: usize;
    const DIRECTIONS: usize;

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Cell>;
    fn get_constraints(&self) -> &[[Cell; Self::DIRECTIONS]; Self::TILE_COUNT];
    fn get_tile_paths(&self) -> Vec<String>;

    fn tile_count(&self) -> usize {
        Self::TILE_COUNT
    }
    fn directions(&self) -> usize {
        Self::DIRECTIONS
    }
}
