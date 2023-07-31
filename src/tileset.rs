use crate::wfc::Cell;

pub trait TileSet {
    const TILE_COUNT: usize;
    const DIRECTIONS: usize;

    fn get_constraints(&self) -> &[[Cell; Self::DIRECTIONS]; Self::TILE_COUNT];
    fn get_tile_paths(&self) -> Vec<String>;

    fn tile_count(&self) -> usize {
        Self::TILE_COUNT
    }

    fn directions(&self) -> usize {
        Self::DIRECTIONS
    }
}
