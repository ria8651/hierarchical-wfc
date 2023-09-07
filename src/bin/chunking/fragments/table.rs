use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use hierarchical_wfc::graphs::regular_grid_3d;

use super::plugin::CollapsedData;

pub type NodeKey = IVec3;
pub type EdgeKey = IVec3;
pub type FaceKey = IVec3;

#[derive(Debug)]
pub enum NodeFragmentEntry {
    Generating,
    Generated(
        regular_grid_3d::GraphSettings,
        regular_grid_3d::GraphData,
        CollapsedData,
    ),
}

#[derive(Debug)]
pub enum EdgeFragmentEntry {
    Waiting(HashSet<IVec3>),
    Generated(
        regular_grid_3d::GraphSettings,
        regular_grid_3d::GraphData,
        CollapsedData,
    ),
}

#[derive(Debug)]
pub enum FaceFragmentEntry {
    Waiting(HashSet<IVec3>),
    Generated(
        regular_grid_3d::GraphSettings,
        regular_grid_3d::GraphData,
        CollapsedData,
    ),
}

#[derive(Resource, Default)]
pub struct FragmentTable {
    pub loaded_nodes: HashMap<NodeKey, NodeFragmentEntry>,
    pub loaded_edges: HashMap<EdgeKey, EdgeFragmentEntry>,
    pub loaded_faces: HashMap<FaceKey, FaceFragmentEntry>,

    pub edges_waiting_on_node: HashMap<NodeKey, HashSet<EdgeKey>>,
    pub faces_waiting_by_edges: HashMap<EdgeKey, HashSet<FaceKey>>,
}
