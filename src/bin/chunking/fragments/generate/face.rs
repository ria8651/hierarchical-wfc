use super::{
    super::{
        plugin::{CollapsedData, GenerationDebugSettings, LayoutSettings},
        table::FragmentTable,
    },
    FRAGMENT_FACE_SIZE,
};
use crate::fragments::{
    graph_utils::{graph_merge, subgraph_with_positions},
    plugin::{FragmentMarker, GenerateDebugMarker},
    table::{EdgeFragmentEntry, FaceFragmentEntry},
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
    wfc::{Superposition, WaveFunctionCollapse},
};
use rand::{rngs::StdRng, SeedableRng};
use std::ops::Div;

pub(crate) fn generate_face(
    face_pos: IVec3,
    fragment_table: &mut ResMut<'_, FragmentTable>,
    q_fragments: &Query<'_, '_, (&GraphSettings, &GraphData, &CollapsedData)>,
    layout_settings: &Res<'_, LayoutSettings>,
    fill_with: Superposition,
    constraints: &Box<[Box<[Superposition]>]>,
    weights: &Vec<u32>,
    commands: &mut Commands<'_, '_>,
    debug_settings: &ResMut<'_, GenerationDebugSettings>,
) {
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
    let edge_entity_ids = edges.map(
        |edge| match fragment_table.loaded_edges.get(&edge).unwrap() {
            EdgeFragmentEntry::Generated(entity) => entity.clone(),
            _ => unreachable!(),
        },
    );
    let edge_entities = q_fragments.get_many(edge_entity_ids).unwrap();
    let merged_x_normal = {
        let z_edge_positions = edge_entities[2]
            .1
            .node_positions
            .iter()
            .copied()
            .map(|pos| pos + IVec3::X * FRAGMENT_FACE_SIZE as i32)
            .collect::<Box<[IVec3]>>();
        let neg_z_edge_positions = &edge_entities[0].1.node_positions;

        graph_merge(
            (&edge_entities[0].2.graph, neg_z_edge_positions),
            (&edge_entities[2].2.graph, &z_edge_positions),
            &|a: Option<&usize>, b| a.or(b).unwrap().clone(),
        )
    };
    let merged_z_normal = {
        let x_edge_positions = edge_entities[3]
            .1
            .node_positions
            .iter()
            .copied()
            .map(|pos| pos + IVec3::Z * FRAGMENT_FACE_SIZE as i32)
            .collect::<Box<[IVec3]>>();
        let neg_x_edge_positions = &edge_entities[1].1.node_positions;

        graph_merge(
            (&edge_entities[1].2.graph, neg_x_edge_positions),
            (&edge_entities[3].2.graph, &x_edge_positions),
            &|a: Option<&usize>, b| a.or(b).unwrap().clone(),
        )
    };
    let (merged_graph, merged_positions) = graph_merge(
        (&merged_x_normal.0, &merged_x_normal.1),
        (&merged_z_normal.0, &merged_z_normal.1),
        &|a: Option<&usize>, b| a.or(b).unwrap().clone(),
    );
    let (face_data, face_graph) = regular_grid_3d::create_graph(
        &GraphSettings {
            size: uvec3(
                FRAGMENT_FACE_SIZE,
                layout_settings.settings.size.y,
                FRAGMENT_FACE_SIZE,
            ),
            spacing: layout_settings.settings.spacing,
        },
        &|(_, _)| fill_with.clone(),
    );
    let (mut merged_graph, merged_positions) = graph_merge(
        (&merged_graph, &merged_positions),
        (&face_graph, &face_data.node_positions),
        &|a: Option<&usize>, b: Option<&Superposition>| {
            b.copied()
                .or(a.and_then(|a| {
                    if *a != 404 {
                        Some(Superposition::single(*a))
                    } else {
                        Some(Superposition::empty())
                    }
                }))
                .unwrap()
                .clone()
        },
    );
    let seed = face_pos.to_array();
    let mut seed: Vec<u8> = seed.map(|i| i.to_be_bytes()).concat().into();
    seed.extend([0u8; 20].into_iter());
    let seed: [u8; 32] = seed.try_into().unwrap();
    WaveFunctionCollapse::collapse(
        &mut merged_graph,
        constraints,
        weights,
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
                                FRAGMENT_FACE_SIZE as i32,
                                layout_settings.settings.size.y as i32,
                                FRAGMENT_FACE_SIZE as i32,
                            ))
                            .all()
                    {
                        Some(tile.clone())
                    } else {
                        None
                    }
                },
                &merged_positions,
            )
        };

        let mut fragment_commands = commands.spawn((
            FragmentMarker,
            Transform::from_translation(
                (face_pos.div(4)).as_vec3()
                    * FRAGMENT_FACE_SIZE as f32
                    * layout_settings.settings.spacing
                    + 2.0 * layout_settings.settings.spacing * Vec3::Y,
            ),
            layout_settings.settings.clone(),
            GraphData {
                node_positions: sub_graph_positions,
            },
            CollapsedData { graph: sub_graph },
        ));
        if debug_settings.debug_fragment_faces {
            fragment_commands.insert(GenerateDebugMarker);
        }
        let fragment = fragment_commands.id();
        fragment_table
            .loaded_faces
            .insert(face_pos, FaceFragmentEntry::Generated(fragment));
    } else {
        unimplemented!();
    }
}
