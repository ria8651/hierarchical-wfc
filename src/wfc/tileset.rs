use super::Superposition;

pub trait TileSet {
    fn tile_count(&self) -> usize;
    fn arc_types(&self) -> usize;
    fn get_constraints(&self) -> Box<[Box<[Superposition]>]>;
    fn get_weights(&self) -> Vec<u32>;
    fn get_tile_paths(&self) -> Vec<String>;
}
