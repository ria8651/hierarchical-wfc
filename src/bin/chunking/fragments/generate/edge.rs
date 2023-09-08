use super::{
    super::{
        plugin::{CollapsedData, FragmentGenerateEvent},
        table::FragmentTable,
    },
    FragmentInstantiateEvent, WfcConfig,
};
use crate::{
    debug::debug_mesh,
    fragments::{
        graph_utils::graph_merge,
        table::{
            EdgeFragmentStatus, FaceFragmentEntry, FaceFragmentStatus, NodeFragmentEntry,
            NodeFragmentStatus,
        },
    },
};
use bevy::{math::ivec3, prelude::*};
use hierarchical_wfc::{
    graphs::{
        regular_grid_3d,
        regular_grid_3d::{GraphData, GraphSettings},
    },
    wfc::{Superposition, TileSet, WaveFunctionCollapse},
};
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

pub(crate) fn generate_edge(
    edge_pos: IVec3,
    wfc_config: Arc<RwLock<WfcConfig>>,
    fragment_table: Arc<RwLock<FragmentTable>>,
    tx_fragment_generate_event: broadcast::Sender<FragmentGenerateEvent>,
    tx_fragment_instantiate_event: broadcast::Sender<FragmentInstantiateEvent>,
) {
    let wfc_config = wfc_config.blocking_read();
    let fragment_settings = &wfc_config.fragment_settings;
    let fill_with = Superposition::filled(wfc_config.tileset.tile_count());

    let node_start_pos = ivec3(
        edge_pos.x.div_euclid(2),
        edge_pos.y.div_euclid(2),
        edge_pos.z.div_euclid(2),
    );
    let node_end_pos = ivec3(
        (edge_pos.x + 1).div_euclid(2),
        (edge_pos.y + 1).div_euclid(2),
        (edge_pos.z + 1).div_euclid(2),
    );
    let edge_normal = node_end_pos - node_start_pos;

    let edge_cotangent = IVec3::Y;
    let edge_tangent = edge_cotangent.cross(edge_normal);
    let edge_volume = {
        let edge_padding = fragment_settings.edge_padding as i32;
        let face_size = fragment_settings.face_size as i32;
        let height = fragment_settings.height as i32;
        (
            edge_padding * edge_normal - edge_padding * edge_tangent,
            (face_size - edge_padding) * edge_normal
                + edge_padding * edge_tangent
                + height * edge_cotangent,
        )
    };
    let edge_volume = (
        edge_volume.0.min(edge_volume.1),
        edge_volume.0.max(edge_volume.1),
    );

    let [node_start, node_end] = {
        let fragment_table = fragment_table.blocking_read();
        if let [Some(a), Some(b)] = [node_start_pos, node_end_pos].map(|node| {
            match fragment_table.loaded_nodes.get(&node).unwrap() {
                NodeFragmentEntry {
                    status: NodeFragmentStatus::Generated(a, b, c),
                    ..
                } => Some((a.clone(), b.clone(), c.clone())), // TODO: Maybe don't clone hear
                _ => None, // Current fragment and its dependencies were unloaded
            }
        }) {
            [a, b]
        } else {
            return;
        }
    };

    let node_start_positions = node_end
        .1
        .node_positions
        .iter()
        .copied()
        .map(|pos| {
            pos - ivec3(1, 0, 1)
                * (fragment_settings.edge_padding + fragment_settings.node_padding) as i32
        })
        .collect::<Box<[IVec3]>>();
    let node_end_positions = node_end
        .1
        .node_positions
        .iter()
        .copied()
        .map(|pos| {
            pos + edge_normal * fragment_settings.face_size as i32
                - ivec3(1, 0, 1)
                    * (fragment_settings.edge_padding + fragment_settings.node_padding) as i32
        })
        .collect::<Box<[IVec3]>>();
    let (merged_graph, merged_positions) = graph_merge(
        (&node_start.2.graph, &node_start_positions),
        (&node_end.2.graph, &node_end_positions),
        &|a: Option<&usize>, b| *a.or(b).unwrap(),
    );
    let (edge_data, edge_graph) =
        regular_grid_3d::create_cuboid(edge_volume.0, edge_volume.1, &|(_, _)| fill_with);
    let (mut merged_graph, merged_positions) = graph_merge(
        (&merged_graph, &merged_positions),
        (&edge_graph, &edge_data.node_positions),
        &|a: Option<&usize>, b: Option<&Superposition>| {
            b.copied()
                .or(a.map(|a| {
                    if *a != 404 {
                        Superposition::single(*a)
                    } else {
                        Superposition::empty()
                    }
                }))
                .unwrap()
        },
    );
    let seed = edge_pos.to_array();
    let mut seed: Vec<u8> = seed.map(|i| i.to_be_bytes()).concat();
    seed.extend([0u8; 20]);
    let seed: [u8; 32] = seed.try_into().unwrap();
    WaveFunctionCollapse::collapse(
        &mut merged_graph,
        &wfc_config.constraints,
        &wfc_config.weights,
        &mut StdRng::from_seed(seed),
    );
    if let Ok(result) = merged_graph.validate() {
        let graph = Arc::new(result).clone();

        let data = GraphData {
            node_positions: merged_positions,
        };

        let layout_settings = GraphSettings {
            size: (edge_volume.1 - edge_volume.0).as_uvec3(),
            spacing: wfc_config.fragment_settings.spacing,
        };

        {
            // Update fragment table entry
            let mut fragment_table = fragment_table.blocking_write();
            if let Some(edge) = fragment_table.loaded_edges.get_mut(&edge_pos) {
                edge.status = EdgeFragmentStatus::Generated(
                    layout_settings.clone(),
                    data.clone(),
                    CollapsedData {
                        graph: graph.clone(),
                    },
                );
            } else {
                return; // Fragment was unloaded and our results aren't needed
            }

            // Instantiate data for debugging
            tx_fragment_instantiate_event
                .send(FragmentInstantiateEvent {
                    fragment_location: super::FragmentLocation::Edge(edge_pos),
                    transform: Transform::from_translation(
                        ivec3(
                            edge_pos.x.div_euclid(2),
                            edge_pos.y.div_euclid(2),
                            edge_pos.z.div_euclid(2),
                        )
                        .as_vec3()
                            * fragment_settings.face_size as f32
                            * fragment_settings.spacing,
                    ),
                    settings: layout_settings.clone(),
                    data: data.clone(),
                    collapsed: CollapsedData {
                        graph: graph.clone(),
                    },
                    meshes: debug_mesh(graph.as_ref(), &data, &layout_settings),
                })
                .unwrap();

            // Update fragments that were waiting on this fragment
            for face in fragment_table
                .faces_waiting_by_edge
                .remove(&edge_pos)
                .unwrap()
            {
                if let Some(FaceFragmentEntry {
                    status: FaceFragmentStatus::Waiting(waiting),
                    ..
                }) = fragment_table.loaded_faces.get_mut(&face)
                {
                    waiting.remove(&edge_pos);
                    if waiting.is_empty() {
                        tx_fragment_generate_event
                            .send(FragmentGenerateEvent::Face(face))
                            .unwrap();
                    }
                } else {
                    panic!();
                }
            }
        }
    }
}
