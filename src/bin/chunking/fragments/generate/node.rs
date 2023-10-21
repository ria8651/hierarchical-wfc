use std::sync::Arc;

use super::{
    super::{
        plugin::{CollapsedData, FragmentGenerateEvent},
        table::FragmentTable,
    },
    FragmentInstantiateEvent, WfcConfig,
};
use crate::{
    debug::debug_mesh,
    fragments::table::{EdgeFragmentEntry, EdgeFragmentStatus, NodeFragmentStatus},
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
    tx_fragment_instantiate_event: broadcast::Sender<FragmentInstantiateEvent>,
) {
    let wfc_config = wfc_config.blocking_read();

    let layout_settings = regular_grid_3d::GraphSettings {
        size: uvec3(
            2 * (wfc_config.fragment_settings.node_padding
                + wfc_config.fragment_settings.edge_padding),
            wfc_config.fragment_settings.height,
            2 * (wfc_config.fragment_settings.node_padding
                + wfc_config.fragment_settings.edge_padding),
        ),
        spacing: wfc_config.fragment_settings.spacing,
    };

    let (data, mut graph) = regular_grid_3d::create_graph(&layout_settings, &|(_, _, _)| {
        Superposition::filled(wfc_config.tileset.tile_count())
    });
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

        {
            // Update fragment table entry
            let mut fragment_table = fragment_table.blocking_write();
            if let Some(node) = fragment_table.loaded_nodes.get_mut(&node_pos) {
                node.status = NodeFragmentStatus::Generated(
                    layout_settings.clone(),
                    data.clone(),
                    CollapsedData {
                        graph: graph.clone(),
                    },
                );
            } else {
                return; // Fragment was unloaded and our results aren't needed
            }

            tx_fragment_instantiate_event
                .send(FragmentInstantiateEvent {
                    fragment_location: super::FragmentLocation::Node(node_pos),
                    transform: Transform::from_translation(
                        (node_pos.as_vec3() * wfc_config.fragment_settings.face_size as f32
                            - vec3(1.0, 0.0, 1.0)
                                * (wfc_config.fragment_settings.node_padding
                                    + wfc_config.fragment_settings.edge_padding)
                                    as f32)
                            * wfc_config.fragment_settings.spacing,
                    ),
                    settings: layout_settings.clone(),
                    data: data.clone(),
                    collapsed: CollapsedData {
                        graph: graph.clone(),
                    },
                    meshes: debug_mesh(graph.as_ref(), &data, &layout_settings),
                })
                .unwrap();

            for edge in fragment_table
                .edges_waiting_by_node
                .remove(&node_pos)
                .unwrap()
            {
                if let Some(EdgeFragmentEntry {
                    status: EdgeFragmentStatus::Waiting(waiting),
                    ..
                }) = fragment_table.loaded_edges.get_mut(&edge)
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
