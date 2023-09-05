use std::ops::Div;

use bevy::prelude::*;
use rand::SeedableRng;

use super::super::plugin::FragmentGenerateEvent;

use crate::fragments::table::FaceFragmentEntry;

use crate::fragments::table::EdgeFragmentEntry;

use crate::fragments::plugin::GenerateDebugMarker;

use crate::fragments::plugin::FragmentMarker;

use rand::rngs::StdRng;

use hierarchical_wfc::wfc::WaveFunctionCollapse;

use hierarchical_wfc::graphs::regular_grid_3d;

use crate::fragments::graph_utils::graph_merge;

use super::NODE_RADIUS;

use super::FRAGMENT_FACE_SIZE;

use super::FRAGMENT_EDGE_PADDING;

use crate::fragments::table::NodeFragmentEntry;

use bevy::math::ivec3;

use super::FragmentQueue;

use super::super::plugin::GenerationDebugSettings;

use hierarchical_wfc::wfc::Superposition;

use super::super::plugin::CollapsedData;

use hierarchical_wfc::graphs::regular_grid_3d::GraphData;

use hierarchical_wfc::graphs::regular_grid_3d::GraphSettings;

use super::super::plugin::LayoutSettings;

use super::super::table::FragmentTable;

pub(crate) fn generate_edge(
    edge_pos: IVec3,
    fragment_table: &mut ResMut<'_, FragmentTable>,
    layout_settings: &Res<'_, LayoutSettings>,
    q_fragments: &Query<'_, '_, (&GraphSettings, &GraphData, &CollapsedData)>,
    fill_with: Superposition,
    constraints: &Box<[Box<[Superposition]>]>,
    weights: &Vec<u32>,
    commands: &mut Commands<'_, '_>,
    debug_settings: &ResMut<'_, GenerationDebugSettings>,
    generate_fragment_queue: &mut Local<'_, FragmentQueue>,
) {
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
    let node_entity_ids = [node_start_pos, node_end_pos].map(|node| {
        match fragment_table.loaded_nodes.get(&node).unwrap() {
            NodeFragmentEntry::Generated(entity) => entity.clone(),
            _ => unreachable!(),
        }
    });
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
    let [node_start, node_end] = q_fragments.get_many(node_entity_ids).unwrap();
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
        &|a: Option<&usize>, b| a.or(b).unwrap().clone(),
    );
    let (edge_data, edge_graph) =
        regular_grid_3d::create_cuboid(edge_volume.0, edge_volume.1, &|(_, _)| fill_with.clone());
    let (mut merged_graph, merged_positions) = graph_merge(
        (&merged_graph, &merged_positions),
        (&edge_graph, &edge_data.node_positions),
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
    let seed = edge_pos.to_array();
    let mut seed: Vec<u8> = seed.map(|i| i.to_be_bytes()).concat().into();
    seed.extend([0u8; 20].into_iter());
    let seed: [u8; 32] = seed.try_into().unwrap();
    WaveFunctionCollapse::collapse(
        &mut merged_graph,
        constraints,
        weights,
        &mut StdRng::from_seed(seed),
    );
    if let Ok(result) = merged_graph.validate() {
        let mut fragment_commands = commands.spawn((
            FragmentMarker,
            Transform::from_translation(
                (edge_pos.div(2)).as_vec3()
                    * FRAGMENT_FACE_SIZE as f32
                    * layout_settings.settings.spacing
                    + layout_settings.settings.spacing * Vec3::Y,
            ),
            layout_settings.settings.clone(),
            GraphData {
                node_positions: merged_positions,
            },
            CollapsedData { graph: result },
        ));
        if debug_settings.debug_fragment_edges {
            fragment_commands.insert(GenerateDebugMarker);
        }
        let fragment = fragment_commands.id();
        fragment_table
            .loaded_edges
            .insert(edge_pos, EdgeFragmentEntry::Generated(fragment));

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
                    generate_fragment_queue
                        .queue
                        .push(FragmentGenerateEvent::Face(face));
                }
            } else {
                unreachable!();
            }
        }
    }
}
