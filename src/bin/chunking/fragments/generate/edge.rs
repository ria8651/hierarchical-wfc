use super::{
    super::{
        plugin::{CollapsedData, FragmentGenerateEvent, GenerationDebugSettings, LayoutSettings},
        table::FragmentTable,
    },
    FragmentInstantiatedEvent, WfcConfig, FRAGMENT_EDGE_PADDING, FRAGMENT_FACE_SIZE, NODE_RADIUS,
};
use crate::{
    debug::debug_mesh,
    fragments::{
        graph_utils::graph_merge,
        plugin::{FragmentMarker, GenerateDebugMarker},
        table::{EdgeFragmentEntry, FaceFragmentEntry, NodeFragmentEntry},
    },
};
use bevy::{math::ivec3, prelude::*};
use hierarchical_wfc::{
    graphs::{
        regular_grid_3d,
        regular_grid_3d::{GraphData, GraphSettings},
    },
    wfc::{self, Superposition, TileSet, WaveFunctionCollapse},
};
use rand::{rngs::StdRng, SeedableRng};
use std::{ops::Div, sync::Arc};
use tokio::sync::{broadcast, RwLock};

pub(crate) fn generate_edge(
    edge_pos: IVec3,
    wfc_config: Arc<RwLock<WfcConfig>>,
    fragment_table: Arc<RwLock<FragmentTable>>,
    tx_fragment_generate_event: broadcast::Sender<FragmentGenerateEvent>,
    tx_fragment_instantiate_event: broadcast::Sender<FragmentInstantiatedEvent>,
) {
    let wfc_config = wfc_config.blocking_read();
    let fill_with = Superposition::filled(wfc_config.layout_settings.tileset.tile_count());
    let layout_settings = &wfc_config.layout_settings;

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
    let edge_volume = (
        FRAGMENT_EDGE_PADDING as i32 * edge_normal - FRAGMENT_EDGE_PADDING as i32 * edge_tangent,
        (FRAGMENT_FACE_SIZE as i32 - FRAGMENT_EDGE_PADDING as i32) * edge_normal
            + FRAGMENT_EDGE_PADDING as i32 * edge_tangent
            + layout_settings.settings.size.y as i32 * edge_cotangent,
    );
    let edge_volume = (
        edge_volume.0.min(edge_volume.1),
        edge_volume.0.max(edge_volume.1),
    );

    let [node_start, node_end] = {
        let fragment_table = fragment_table.blocking_read();
        [node_start_pos, node_end_pos].map(|node| {
            match fragment_table.loaded_nodes.get(&node).unwrap() {
                NodeFragmentEntry::Generated(a, b, c) => (a.clone(), b.clone(), c.clone()), // TODO: Maybe don't clone hear
                _ => unreachable!(),
            }
        })
    };

    let node_start_positions = node_end
        .1
        .node_positions
        .iter()
        .copied()
        .map(|pos| pos - ivec3(1, 0, 1) * NODE_RADIUS)
        .collect::<Box<[IVec3]>>();
    let node_end_positions = node_end
        .1
        .node_positions
        .iter()
        .copied()
        .map(|pos| pos + edge_normal * FRAGMENT_FACE_SIZE as i32 - ivec3(1, 0, 1) * NODE_RADIUS)
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

        tx_fragment_instantiate_event
            .send(FragmentInstantiatedEvent {
                fragment_type: super::FragmentType::Edge,
                transform: Transform::from_translation(
                    (edge_pos.div(2)).as_vec3()
                        * FRAGMENT_FACE_SIZE as f32
                        * layout_settings.settings.spacing
                        + layout_settings.settings.spacing * Vec3::Y,
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
            fragment_table.loaded_edges.insert(
                edge_pos,
                EdgeFragmentEntry::Generated(
                    wfc_config.layout_settings.settings.clone(),
                    data,
                    CollapsedData { graph },
                ),
            );
            // Update fragments that were waiting on this fragment
            for face in fragment_table
                .faces_waiting_by_edges
                .remove(&edge_pos)
                .unwrap()
            {
                if let Some(FaceFragmentEntry::Waiting(waiting)) =
                    fragment_table.loaded_faces.get_mut(&face)
                {
                    waiting.remove(&edge_pos);
                    if waiting.is_empty() {
                        tx_fragment_generate_event
                            .send(FragmentGenerateEvent::Face(face))
                            .unwrap();
                    }
                } else {
                    unreachable!();
                }
            }
        }
    }
}
