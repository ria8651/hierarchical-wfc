use bevy::prelude::*;
use itertools::Itertools;

use crate::{
    castle::facade_graph::FacadeTileset,
    graphs::regular_quad_mesh::{utils::get_matching_direction, GraphData},
    wfc::{Neighbour, Superposition, WfcGraph},
};

use super::GraphBuilder;

impl GraphBuilder {
    pub fn build_graph(self, tileset: &FacadeTileset) -> (GraphData, WfcGraph<Superposition>) {
        let mut nodes = self.build_nodes(
            &|(_, _)| tileset.superposition_from_semantic_name("vertex".to_string()),
            &|(_, _)| tileset.superposition_from_semantic_name("edge".to_string()),
            &|(_, _)| tileset.superposition_from_semantic_name("quad".to_string()),
        );
        let neighbours: Box<[Box<[Neighbour]>]> = [
            self.build_vertex_neighbours(),
            self.build_edge_neighbours(),
            self.build_quad_neighbours(),
        ]
        .concat()
        .into();

        Self::constrain_node_directions(&mut nodes, &neighbours, tileset);

        // Some nodes might already be fully constrained
        let order = nodes
            .iter()
            .enumerate()
            .filter_map(|(node_id, node)| {
                if node.count_bits() <= 1 {
                    Some(node_id)
                } else {
                    None
                }
            })
            .collect_vec();

        (
            GraphData {
                vertices: self.vertices.into(),
                edges: self.edges.into(),
                quads: self.quads.into(),
            },
            WfcGraph {
                nodes,
                order,
                neighbours,
            },
        )
    }

    pub fn build_vertex_neighbours(&self) -> Box<[Box<[Neighbour]>]> {
        self.vertices
            .iter()
            .map(|vert| {
                vert.edges
                    .iter()
                    .enumerate()
                    .filter_map(|(index, edge)| {
                        edge.as_ref().map(|edge| Neighbour {
                            arc_type: index,
                            index: edge + self.vertices.len(),
                        })
                    })
                    .collect::<Box<[_]>>()
            })
            .collect::<Box<[_]>>()
    }

    pub fn build_edge_neighbours(&self) -> Box<[Box<[Neighbour]>]> {
        self.edges
            .iter()
            .map(|edge| {
                let mut neighbours = vec![
                    Neighbour {
                        arc_type: get_matching_direction(edge.tangent),
                        index: edge.from,
                    },
                    Neighbour {
                        arc_type: edge.tangent,
                        index: edge.to,
                    },
                ];
                neighbours.extend(edge.quads.iter().map(|(direction, quad)| Neighbour {
                    arc_type: *direction,
                    index: self.vertices.len() + self.edges.len() + quad,
                }));
                neighbours.into()
            })
            .collect::<Box<[_]>>()
    }

    pub fn build_quad_neighbours(&self) -> Box<[Box<[Neighbour]>]> {
        self.quads
            .iter()
            .map(|quad| {
                quad.edges
                    .iter()
                    .enumerate()
                    .map(|(index, edge)| Neighbour {
                        arc_type: [
                            quad.cotangent + 1,
                            quad.tangent,
                            quad.cotangent,
                            quad.tangent + 1,
                        ][index],
                        index: self.vertices.len() + *edge,
                    })
                    .collect::<Box<[_]>>()
            })
            .collect::<Box<[_]>>()
    }

    pub fn build_nodes(
        &self,
        vertex_generator: &dyn Fn((usize, IVec3)) -> Superposition,
        edge_generator: &dyn Fn((usize, IVec3)) -> Superposition,
        quad_generator: &dyn Fn((usize, IVec3)) -> Superposition,
    ) -> Vec<Superposition> {
        [
            self.vertices
                .iter()
                .enumerate()
                .map(|(i, v)| vertex_generator((i, v.pos)))
                .collect_vec(),
            self.edges
                .iter()
                .enumerate()
                .map(|(i, v)| edge_generator((i, v.pos)))
                .collect_vec(),
            self.quads
                .iter()
                .enumerate()
                .map(|(i, v)| quad_generator((i, v.pos)))
                .collect_vec(),
        ]
        .into_iter()
        .flat_map(|v| v.into_iter())
        .collect_vec()
    }

    fn constrain_node_directions(
        nodes: &mut [Superposition],
        neighbours: &[Box<[Neighbour]>],
        tileset: &FacadeTileset,
    ) {
        for (node_index, node) in nodes.iter_mut().enumerate() {
            let directions = neighbours[node_index]
                .iter()
                .map(|neighbour| 1 << neighbour.arc_type)
                .reduce(|a, b| a | b)
                .unwrap();

            *node =
                Superposition::intersect(node, &tileset.superposition_from_directions(directions));
        }
    }
}
