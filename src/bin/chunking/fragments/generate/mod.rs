use std::sync::Arc;

use bevy::{self, math::vec3, prelude::*};
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
    plugin::{CollapsedData, FragmentGenerateEvent},
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

#[derive(Reflect, Resource, Clone)]
pub struct FragmentSettings {
    pub spacing: Vec3,
    pub node_padding: u32,
    pub edge_padding: u32,
    pub face_size: u32,
    pub height: u32,
}

impl Default for FragmentSettings {
    fn default() -> Self {
        Self {
            spacing: vec3(2., 3., 2.),
            node_padding: 4,
            edge_padding: 4,
            face_size: 32,
            height: 8,
        }
    }
}
pub struct WfcConfig {
    pub fragment_settings: FragmentSettings,
    pub tileset: LayoutTileset,
    pub constraints: Box<[Box<[Superposition]>]>,
    pub weights: Vec<u32>,
}
impl Default for WfcConfig {
    fn default() -> Self {
        let tileset = LayoutTileset;
        let constraints = tileset.get_constraints();
        let weights = tileset.get_weights();
        Self {
            fragment_settings: FragmentSettings::default(),
            tileset,
            constraints,
            weights,
        }
    }
}
