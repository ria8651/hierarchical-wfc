use crate::graph_wfc::Cell;

pub trait TileSet {
    const TILE_COUNT: usize;
    const DIRECTIONS: usize;

    fn allowed_neighbors() -> [[Cell; Self::DIRECTIONS]; Self::TILE_COUNT];
    fn get_tile_paths() -> Vec<String>;
}
