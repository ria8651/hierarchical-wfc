use crate::graph_wfc::Direction;
use bevy::utils::{HashMap, HashSet};
use rand::Rng;

pub trait TileSet {
    type Tile: Eq + std::hash::Hash + Copy + std::fmt::Debug;
    type Direction: Eq + std::hash::Hash + Copy + std::fmt::Debug;

    fn allowed_neighbors() -> AllowedNeighbors<Self>;
    fn random_tile<R: Rng>(rng: &mut R) -> Self::Tile;
    fn all_tiles() -> HashSet<Self::Tile>;
    fn get_tile_paths() -> Vec<String>;
}

pub type AllowedNeighbors<T> =
    HashMap<<T as TileSet>::Tile, HashMap<Direction, HashSet<<T as TileSet>::Tile>>>;
