use std::ops::Div;

use crate::fragments::{
    graph_utils::{graph_merge, subgraph_with_positions},
    plugin::{ChunkEntry, FragmentMarker, GenerateDebugMarker},
    table::{EdgeFragmentEntry, EdgeKey, FaceFragmentEntry, FaceKey, NodeFragmentEntry, NodeKey},
};

use bevy::{
    math::{ivec3, uvec3, vec3},
    prelude::*,
    utils::HashSet,
};
use hierarchical_wfc::{
    graphs::regular_grid_3d::{self, GraphData, GraphSettings},
    wfc::{Superposition, TileSet, WaveFunctionCollapse},
};
use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use super::{
    plugin::{
        ChunkLoadEvent, ChunkTable, CollapsedData, FragmentGenerateEvent, GenerationDebugSettings,
        LayoutSettings,
    },
    table::FragmentTable,
};

/// Transforms chunk load events into fragments which are registered for generation in the fragment table
pub fn transform_chunk_loads(
    mut ev_load_chunk: EventReader<ChunkLoadEvent>,
    mut ev_generate_fragment: EventWriter<FragmentGenerateEvent>,
    mut chunk_table: ResMut<ChunkTable>,
    mut fragment_table: ResMut<FragmentTable>,
) {
    for load_chunk in ev_load_chunk.iter() {
        match load_chunk {
            ChunkLoadEvent::Load(chunk_pos) => {
                if let Some(chunk) = chunk_table.loaded.get(chunk_pos) {
                    match chunk {
                        ChunkEntry::Waiting => continue,
                    }
                }
                chunk_table.loaded.insert(*chunk_pos, ChunkEntry::Waiting);

                // Positions of chunks component fragments
                let faces_pos = [4 * *chunk_pos + 2 * IVec3::X + 2 * IVec3::Z];
                let edges_pos = [
                    2 * *chunk_pos + IVec3::Z,
                    2 * *chunk_pos + IVec3::X,
                    2 * *chunk_pos + 2 * IVec3::X + IVec3::Z,
                    2 * *chunk_pos + IVec3::X + 2 * IVec3::Z,
                ];
                let nodes_pos = [
                    *chunk_pos,
                    *chunk_pos + IVec3::X,
                    *chunk_pos + IVec3::X + IVec3::Z,
                    *chunk_pos + IVec3::Z,
                ];

                let edge_loaded =
                    edges_pos.map(|pos| match fragment_table.loaded_edges.get(&pos) {
                        Some(EdgeFragmentEntry::Generated(_)) => true,
                        _ => false,
                    });
                let node_loaded =
                    nodes_pos.map(|pos| match fragment_table.loaded_nodes.get(&pos) {
                        Some(NodeFragmentEntry::Generated(_)) => true,
                        _ => false,
                    });

                let face: FaceKey = nodes_pos.iter().sum();
                assert_eq!(face, faces_pos[0]);

                for index in 0..4 {
                    let node: NodeKey = nodes_pos[index];

                    let prev_node_index = (index + 3).rem_euclid(4);
                    let next_node_index = (index + 1).rem_euclid(4);
                    let prev_node: NodeKey = nodes_pos[prev_node_index];
                    let next_node: NodeKey = nodes_pos[next_node_index];

                    let edge: EdgeKey = node + prev_node;
                    let next_edge: EdgeKey = node + next_node;

                    assert_eq!(edge, edges_pos[(index).rem_euclid(4)]);
                    assert_eq!(next_edge, edges_pos[(index + 1).rem_euclid(4)]);

                    if !fragment_table.loaded_nodes.contains_key(&node) {
                        // Announce new node to generate
                        fragment_table
                            .loaded_nodes
                            .insert(node, NodeFragmentEntry::Generating);
                        ev_generate_fragment.send(FragmentGenerateEvent::Node(node));
                    }

                    if !fragment_table.loaded_edges.contains_key(&edge) {
                        // Keep track of what the edge is waiting for to generate
                        let waiting_for = [
                            match node_loaded[prev_node_index] {
                                true => None,
                                false => Some(prev_node),
                            },
                            match node_loaded[index] {
                                true => None,
                                false => Some(node),
                            },
                        ]
                        .into_iter()
                        .flatten()
                        .collect_vec();

                        for node in waiting_for.clone() {
                            let waiting_on_node = fragment_table
                                .edges_waiting_on_node
                                .entry(node)
                                .or_insert(HashSet::new());
                            waiting_on_node.insert(edge);
                        }

                        // Check if dependencies have already been satisfied.
                        if !waiting_for.is_empty() {
                            fragment_table.loaded_edges.insert(
                                edge,
                                EdgeFragmentEntry::Waiting(HashSet::from_iter(waiting_for)),
                            );
                        } else {
                            ev_generate_fragment.send(FragmentGenerateEvent::Edge(edge));
                        }
                    }
                }

                if !fragment_table.loaded_faces.contains_key(&face) {
                    // Keep track of fragments the face is waiting for
                    let waiting_for = edges_pos
                        .into_iter()
                        .zip(edge_loaded.into_iter())
                        .map(|(pos, loaded)| match loaded {
                            true => None,
                            false => Some(pos),
                        })
                        .flatten()
                        .collect_vec();

                    for edge in waiting_for.clone() {
                        let faces_awaiting_edge = fragment_table
                            .faces_waiting_by_edges
                            .entry(edge)
                            .or_insert(HashSet::new());
                        faces_awaiting_edge.insert(face);
                    }

                    // Check if dependencies have already been satisfied.
                    if !waiting_for.is_empty() {
                        fragment_table.loaded_faces.insert(
                            face,
                            FaceFragmentEntry::Waiting(HashSet::from_iter(waiting_for)),
                        );
                    } else {
                        ev_generate_fragment.send(FragmentGenerateEvent::Face(face));
                    }
                }
            }
        }
    }
}
