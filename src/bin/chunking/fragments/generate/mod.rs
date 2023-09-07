use std::sync::Arc;

use bevy::{
    self,
    math::{uvec3, vec3},
    prelude::*,
};
use bevy_rapier3d::prelude::Collider;
use hierarchical_wfc::{
    castle::LayoutTileset,
    graphs::regular_grid_3d::{self},
    wfc::{Superposition, TileSet},
};
use tokio::{
    runtime,
    sync::{broadcast, RwLock},
};

use super::{
    plugin::{CollapsedData, FragmentGenerateEvent, LayoutSettings},
    table::FragmentTable,
};

pub mod edge;
pub mod face;
pub mod node;

#[derive(Debug, Clone)]
pub struct FragmentInstantiatedEvent {
    pub fragment_type: FragmentType,
    pub transform: Transform,
    pub settings: regular_grid_3d::GraphSettings,
    pub data: regular_grid_3d::GraphData,
    pub collapsed: CollapsedData,
    pub meshes: (Mesh, Mesh, Option<Collider>),
}
#[derive(Debug, Clone)]

pub enum FragmentType {
    Node,
    Edge,
    Face,
}

const FRAGMENT_NODE_PADDING: u32 = 4;
const FRAGMENT_EDGE_PADDING: u32 = 4;
const FRAGMENT_FACE_SIZE: u32 = 32;
const NODE_RADIUS: i32 = FRAGMENT_EDGE_PADDING as i32 + FRAGMENT_NODE_PADDING as i32;

pub async fn generate_fragments(
    rt: Arc<runtime::Runtime>,
    fragment_table: Arc<RwLock<FragmentTable>>,
    wfc_config: Arc<RwLock<WfcConfig>>,
    mut rx_generate_fragment: broadcast::Receiver<FragmentGenerateEvent>,
    tx_generate_fragment: broadcast::Sender<FragmentGenerateEvent>,
    tx_fragment_instantiate: broadcast::Sender<FragmentInstantiatedEvent>,
) {
    loop {
        tokio::select! {
                event = rx_generate_fragment.recv() =>
                {
                    let fragment_table = fragment_table.clone();
                    let (
                        wfc_config,
                        tx_generate_fragment_events,
                        tx_fragment_instantiate_event,
                    ) = (
                        wfc_config.clone(),
                        tx_generate_fragment.clone(),
                        tx_fragment_instantiate.clone(),
                    );

                    match event.unwrap() {
                        FragmentGenerateEvent::Node(node_pos) => {
                            rt.spawn_blocking( move || {
                                node::generate_node(
                                    node_pos,
                                    wfc_config,
                                    fragment_table,
                                    tx_generate_fragment_events,
                                    tx_fragment_instantiate_event,
                                );
                            });
                        }
                        FragmentGenerateEvent::Edge(edge_pos) => {
                            rt.spawn_blocking( move || {
                                edge::generate_edge(
                                    edge_pos,
                                    wfc_config,
                                    fragment_table,
                                    tx_generate_fragment_events,
                                    tx_fragment_instantiate_event,
                                );
                            });
                        },
                        FragmentGenerateEvent::Face(face_pos) => {
                            rt.spawn_blocking( move || {
                                face::generate_face(
                                    face_pos,
                                    wfc_config,
                                    fragment_table,
                                    tx_fragment_instantiate_event,
                                );
                            });
                    }
               }
            }
        }
    }
}

pub struct WfcConfig {
    layout_settings: LayoutSettings,
    constraints: Box<[Box<[Superposition]>]>,
    weights: Vec<u32>,
}
impl Default for WfcConfig {
    fn default() -> Self {
        let tileset = LayoutTileset;
        let constraints = tileset.get_constraints();
        let weights = tileset.get_weights();
        Self {
            constraints,
            layout_settings: LayoutSettings {
                tileset,
                settings: regular_grid_3d::GraphSettings {
                    size: uvec3(16, 4, 16),
                    spacing: vec3(2., 3., 2.),
                },
            },
            weights,
        }
    }
}
