use crate::{wfc_graph::WaveFunction, Graph};
use bevy::prelude::*;
use dyn_clone::DynClone;
use std::{any::Any, sync::Arc};

#[derive(Debug, Clone)]
pub enum TileRender {
    Sprite(String),
    Color(Color),
}

pub trait TileSet: DynClone + Send + Sync {
    fn tile_count(&self) -> usize;
    fn directions(&self) -> usize;
    fn create_graph(&self, settings: Box<dyn Any>) -> Graph<WaveFunction>;
    fn get_constraints(&self) -> Arc<Vec<Vec<WaveFunction>>>;
    fn get_weights(&self) -> Arc<Vec<f32>>;
    fn set_weights(&mut self, weights: Vec<f32>);
    fn get_tile_paths(&self) -> Vec<(TileRender, Transform)>;
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn TileSet> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}
