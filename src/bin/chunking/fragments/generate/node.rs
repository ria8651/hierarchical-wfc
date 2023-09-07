use std::sync::Arc;

use super::{
    super::{
        plugin::{CollapsedData, FragmentGenerateEvent},
        table::FragmentTable,
    },
    FragmentInstantiatedEvent, WfcConfig, FRAGMENT_EDGE_PADDING, FRAGMENT_FACE_SIZE,
    FRAGMENT_NODE_PADDING,
};
use crate::{
    debug::debug_mesh,
    fragments::table::{EdgeFragmentEntry, NodeFragmentEntry},
};
use bevy::{
    math::{uvec3, vec3},
    prelude::*,
};
use hierarchical_wfc::{
    graphs::regular_grid_3d,
    wfc::{Superposition, TileSet, WaveFunctionCollapse},
};
use rand::{rngs::StdRng, SeedableRng};
use tokio::sync::{broadcast, RwLock};

pub(crate) fn generate_node(
    node_pos: IVec3,
    wfc_config: Arc<RwLock<WfcConfig>>,
    fragment_table: Arc<RwLock<FragmentTable>>,
    tx_fragment_generate_event: broadcast::Sender<FragmentGenerateEvent>,
    tx_fragment_instantiate_event: broadcast::Sender<FragmentInstantiatedEvent>,
) {
    let wfc_config = wfc_config.blocking_read();

    let (data, mut graph) = regular_grid_3d::create_graph(
        &regular_grid_3d::GraphSettings {
            size: uvec3(
                2 * (FRAGMENT_NODE_PADDING + FRAGMENT_EDGE_PADDING),
                wfc_config.layout_settings.settings.size.y,
                2 * (FRAGMENT_NODE_PADDING + FRAGMENT_EDGE_PADDING),
            ),
            spacing: wfc_config.layout_settings.settings.spacing,
        },
        &|(_, _)| Superposition::filled(wfc_config.layout_settings.tileset.tile_count()),
    );
    let seed = node_pos.to_array();
    let mut seed: Vec<u8> = seed.map(|i| i.to_be_bytes()).concat();
    seed.extend([0u8; 20]);
    let seed: [u8; 32] = seed.try_into().unwrap();
    WaveFunctionCollapse::collapse(
        &mut graph,
        &wfc_config.constraints,
        &wfc_config.weights,
        &mut StdRng::from_seed(seed),
    );
    if let Ok(result) = graph.validate() {
        let graph = Arc::new(result).clone();

        tx_fragment_instantiate_event
            .send(FragmentInstantiatedEvent {
                fragment_type: super::FragmentType::Node,
                transform: Transform::from_translation(
                    node_pos.as_vec3()
                        * FRAGMENT_FACE_SIZE as f32
                        * wfc_config.layout_settings.settings.spacing
                        - vec3(1.0, 0.0, 1.0)
                            * (FRAGMENT_NODE_PADDING + FRAGMENT_EDGE_PADDING) as f32
                            * wfc_config.layout_settings.settings.spacing,
                ),
                settings: wfc_config.layout_settings.settings.clone(),
                data: data.clone(),
                collapsed: CollapsedData {
                    graph: graph.clone(),
                },
                meshes: debug_mesh(graph.as_ref(), &data, &wfc_config.layout_settings.settings),
            })
            .unwrap();

        // Scope with write lock on fragment table
        {
            let mut fragment_table = fragment_table.blocking_write();
            fragment_table.loaded_nodes.insert(
                node_pos,
                NodeFragmentEntry::Generated(
                    wfc_config.layout_settings.settings.clone(),
                    data,
                    CollapsedData { graph },
                ),
            );

            for edge in fragment_table
                .edges_waiting_on_node
                .remove(&node_pos)
                .unwrap()
            {
                if let Some(EdgeFragmentEntry::Waiting(waiting)) =
                    fragment_table.loaded_edges.get_mut(&edge)
                {
                    assert!(waiting.remove(&node_pos));
                    if waiting.is_empty() {
                        tx_fragment_generate_event
                            .send(FragmentGenerateEvent::Edge(edge))
                            .unwrap();
                    }
                } else {
                    panic!();
                }
            }
        }
    } else {
        panic!();
    }
}
