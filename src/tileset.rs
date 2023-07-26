use crate::graph_wfc::{Cell, Direction};
use bevy::utils::HashMap;

pub trait TileSet {
    type Direction: Eq + std::hash::Hash + Copy + std::fmt::Debug;

    const TILE_COUNT: usize;

    fn allowed_neighbors() -> AllowedNeighbors;
    fn get_tile_paths() -> Vec<String>;
}

pub type AllowedNeighbors = HashMap<usize, HashMap<Direction, Cell>>;
