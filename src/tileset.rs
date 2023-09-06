use crate::graph::{Graph, WaveFunction};
use bevy::prelude::*;
use dyn_clone::DynClone;

pub trait TileSet: DynClone + Send + Sync {
    type GraphSettings;

    fn tile_count(&self) -> usize;
    fn directions(&self) -> usize;
    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<WaveFunction>;
    fn get_constraints(&self) -> Vec<Vec<WaveFunction>>;
    fn get_weights(&self) -> Vec<f32>;
    fn get_tile_paths(&self) -> Vec<(String, Transform)>;
}

impl<T> Clone for Box<dyn TileSet<GraphSettings = T>> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}
