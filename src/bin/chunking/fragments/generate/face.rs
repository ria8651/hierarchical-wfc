use super::{
    super::{plugin::CollapsedData, table::FragmentTable},
    FragmentInstantiatedEvent, WfcConfig,
};
use crate::{
    debug::debug_mesh,
    fragments::{
        graph_utils::{graph_merge, subgraph_with_positions},
        table::{EdgeFragmentEntry, FaceFragmentEntry},
    },
};
use bevy::{
    math::{ivec3, uvec3},
    prelude::*,
};
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

pub(crate) fn generate_face(
    face_pos: IVec3,
    wfc_config: Arc<RwLock<WfcConfig>>,
    fragment_table: Arc<RwLock<FragmentTable>>,
    tx_fragment_instantiate_event: broadcast::Sender<FragmentInstantiatedEvent>,
) {
    let wfc_config = wfc_config.blocking_read();
    let fill_with = Superposition::filled(wfc_config.tileset.tile_count());
    let fragment_settings = &wfc_config.fragment_settings;
    // let layout_settings = &wfc_config.layout_settings;

    let face_origin = ivec3(
        face_pos.x.div_euclid(4),
        face_pos.y.div_euclid(4),
        face_pos.z.div_euclid(4),
    );
    let edges = [
        2 * face_origin + IVec3::Z,
        2 * face_origin + IVec3::X,
        2 * face_origin + 2 * IVec3::X + IVec3::Z,
        2 * face_origin + IVec3::X + 2 * IVec3::Z,
    ];
    let edge_data = {
        let fragment_table = fragment_table.blocking_read();
        edges.map(
            |edge| match fragment_table.loaded_edges.get(&edge).unwrap() {
                EdgeFragmentEntry::Generated(a, b, c) => (a.clone(), b.clone(), c.clone()),
                _ => unreachable!(),
            },
        )
    };

    let merged_x_normal = {
        let z_edge_positions = edge_data[2]
            .1
            .node_positions
            .iter()
            .copied()
            .map(|pos| pos + IVec3::X * fragment_settings.face_size as i32)
            .collect::<Box<[IVec3]>>();
        let neg_z_edge_positions = &edge_data[0].1.node_positions;

        graph_merge(
            (&edge_data[0].2.graph, neg_z_edge_positions),
            (&edge_data[2].2.graph, &z_edge_positions),
            &|a: Option<&usize>, b| *a.or(b).unwrap(),
        )
    };
    let merged_z_normal = {
        let x_edge_positions = edge_data[3]
            .1
            .node_positions
            .iter()
            .copied()
            .map(|pos| pos + IVec3::Z * fragment_settings.face_size as i32)
            .collect::<Box<[IVec3]>>();
        let neg_x_edge_positions = &edge_data[1].1.node_positions;

        graph_merge(
            (&edge_data[1].2.graph, neg_x_edge_positions),
            (&edge_data[3].2.graph, &x_edge_positions),
            &|a: Option<&usize>, b| *a.or(b).unwrap(),
        )
    };
    let (merged_graph, merged_positions) = graph_merge(
        (&merged_x_normal.0, &merged_x_normal.1),
        (&merged_z_normal.0, &merged_z_normal.1),
        &|a: Option<&usize>, b| *a.or(b).unwrap(),
    );

    let layout_settings = GraphSettings {
        size: uvec3(
            fragment_settings.face_size,
            fragment_settings.height,
            fragment_settings.face_size,
        ),
        spacing: fragment_settings.spacing,
    };

    let (face_data, face_graph) =
        regular_grid_3d::create_graph(&layout_settings, &|(_, _)| fill_with);
    let (mut merged_graph, merged_positions) = graph_merge(
        (&merged_graph, &merged_positions),
        (&face_graph, &face_data.node_positions),
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
    let seed = face_pos.to_array();
    let mut seed: Vec<u8> = seed.map(|i| i.to_be_bytes()).concat();
    seed.extend([0u8; 20]);
    let seed: [u8; 32] = seed.try_into().unwrap();
    WaveFunctionCollapse::collapse(
        &mut merged_graph,
        &wfc_config.constraints,
        &wfc_config.weights,
        &mut StdRng::from_seed(seed),
    );
    if let Ok(result_graph) = merged_graph.validate() {
        let (sub_graph, sub_graph_positions) = {
            subgraph_with_positions(
                &result_graph,
                &|index, tile| {
                    let position = merged_positions[index];

                    if IVec3::ZERO.cmple(position).all()
                        && position
                            .cmplt(ivec3(
                                fragment_settings.face_size as i32,
                                fragment_settings.height as i32,
                                fragment_settings.face_size as i32,
                            ))
                            .all()
                    {
                        Some(*tile)
                    } else {
                        None
                    }
                },
                &merged_positions,
            )
        };

        let graph = Arc::new(sub_graph).clone();
        let data = GraphData {
            node_positions: sub_graph_positions,
        };
        tx_fragment_instantiate_event
            .send(FragmentInstantiatedEvent {
                fragment_type: super::FragmentType::Face,
                transform: Transform::from_translation(
                    (face_pos / 4).as_vec3()
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

        // Scope with write lock on fragment table
        {
            let mut fragment_table = fragment_table.blocking_write();
            fragment_table.loaded_faces.insert(
                face_pos,
                FaceFragmentEntry::Generated(
                    layout_settings.clone(),
                    data,
                    CollapsedData { graph },
                ),
            );
        }
    } else {
        unimplemented!();
    }
}
