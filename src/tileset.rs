use crate::graph::WaveFunction;
use bevy::prelude::*;
use dyn_clone::DynClone;
use std::sync::Arc;

pub trait TileSet: DynClone + Send + Sync {
    fn tile_count(&self) -> usize;
    fn directions(&self) -> usize;
    fn get_constraints(&self) -> Arc<Vec<Vec<WaveFunction>>>;
    fn get_weights(&self) -> Arc<Vec<f32>>;
    fn set_weights(&mut self, weights: Vec<f32>);
    fn get_tile_paths(&self) -> Vec<(String, Transform)>;
}

impl Clone for Box<dyn TileSet> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}
