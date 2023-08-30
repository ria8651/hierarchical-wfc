use bevy::{math::ivec3, prelude::*};

use crate::{
    graphs::{
        regular_grid_3d,
        regular_quad_mesh::utils::{ivec3_to_direction, DIRECTIONS},
    },
    tools::index_tools::{ivec3_in_bounds, ivec3_to_index},
    wfc::WfcGraph,
};

use super::{super::types::*, GraphBuilder};

impl GraphBuilder {
    // Labeling the corners of the cube with bits according to:
    // 0: +x +y +z
    // 1: -x +y +z
    // 2: +x -y +z
    // 3: -x -y +z
    // 4: +x +y -z
    // 5: -x +y -z
    // 6: +x -y -z
    // 7: -x -y -z
    const CONFIG_PLUS_X_MASK: usize = 0b01010101;
    const CONFIG_PLUS_Y_MASK: usize = 0b00110011;
    const CONFIG_PLUS_Z_MASK: usize = 0b00001111;

    const CONFIG_PLUS_ZYX_MASK: usize = Self::CONFIG_PLUS_X_MASK
        + (Self::CONFIG_PLUS_Y_MASK << 8)
        + (Self::CONFIG_PLUS_Z_MASK << 16);

    fn edge_configuration(from: usize, to: usize, direction: usize) -> usize {
        // Vertex configuration bit index: delta

        // Select the root vertex so that the edge is increasing
        let root = if direction.rem_euclid(2) == 0 {
            from
        } else {
            to
        };

        // Result after comparing
        // x: from[2n]      & to[2n+1]     : from & (to>>1) .3.2.1.0
        // y: from[0,1,4,5] & to[2,3,6,7]  : from & (to>>2) ..32..01
        // z: from[0,1,2,3] & to[4,5,6,7]  : from & (to>>4) ....3210
        match direction {
            0..=1 => root & Self::CONFIG_PLUS_X_MASK,
            2..=3 => root & Self::CONFIG_PLUS_Y_MASK,
            4..=5 => root & Self::CONFIG_PLUS_Z_MASK,
            _ => unreachable!(),
        }
    }

    fn edge_occluded(edge_configuration: usize) -> bool {
        println!("Checking: {:08b}", edge_configuration);
        (edge_configuration & Self::CONFIG_PLUS_X_MASK == Self::CONFIG_PLUS_X_MASK)
            || (edge_configuration & Self::CONFIG_PLUS_Y_MASK == Self::CONFIG_PLUS_Y_MASK)
            || (edge_configuration & Self::CONFIG_PLUS_Z_MASK == Self::CONFIG_PLUS_Z_MASK)
    }

    fn quad_normal(root_configuration: usize, u_step: usize, v_step: usize) -> (bool, usize) {
        // We always step along positive directions and X=0 Y=2 Z=4, so adding all steps give 6
        // therefore subtracting two step directions from 6 gives the last direction .
        let w_step = 6 - u_step - v_step;

        // Three masks in the form [Z][Y][X], masks vertex configuration bits corresponding
        // to the contents in neighbouring cells along + that axis
        let face_configuration_mask = 0b11111111
            & (Self::CONFIG_PLUS_ZYX_MASK >> (u_step * 4))
            & (Self::CONFIG_PLUS_ZYX_MASK >> (v_step * 4));

        // Mask two to only the two cells across the edge of the vertex configuration corresponding  to +U +V.
        let config = face_configuration_mask & root_configuration;

        // Check if the remaining bit lies in the +W or -W direction, the opposite
        // direction to this will be the normal of the face.
        let invert = (Self::CONFIG_PLUS_ZYX_MASK >> (w_step * 4)) & config != 0;
        let normal = w_step + invert as usize;

        // If there are occupied cells in both +W and -W the face is occluded
        let occluded = (Self::CONFIG_PLUS_ZYX_MASK >> (w_step * 4)) & config != 0
            && (!Self::CONFIG_PLUS_ZYX_MASK >> (w_step * 4)) & config != 0;

        (occluded, normal)
    }

    pub fn from_regular_3d_grid(
        grid_settings: &regular_grid_3d::GraphSettings,
        grid_collapsed: &WfcGraph<usize>,
    ) -> Self {
        let mut nodes: Vec<bool> = vec![
            false;
            (grid_settings.size.x as usize + 1)
                * (grid_settings.size.y as usize + 1)
                * (grid_settings.size.z as usize + 1)
        ];
        let size = ivec3(
            grid_settings.size.x as i32,
            grid_settings.size.y as i32,
            grid_settings.size.z as i32,
        );

        let node_pos = itertools::iproduct!(0..size.z + 1, 0..size.y + 1, 0..size.x + 1)
            .map(|(z, y, x)| ivec3(x, y, z));

        let mut new_node_indices: Vec<Option<usize>> = Vec::new();
        let mut new_node_index: usize = 0;

        let mut vertex_configurations: Vec<usize> = Vec::new();

        for (index, pos) in node_pos.clone().enumerate() {
            let mut connected: i32 = 0;
            let mut vertex_configuration: usize = 0;
            for delta in
                itertools::iproduct!(-1..=0, -1..=0, -1..=0).map(|(z, y, x)| ivec3(x, y, z))
            {
                vertex_configuration <<= 1;
                let pos = pos + delta;
                if (0..size.x).contains(&pos.x)
                    && (0..size.y).contains(&pos.y)
                    && (0..size.z).contains(&pos.z)
                {
                    let index = pos.dot(ivec3(1, size.x, size.x * size.y)) as usize;

                    let tile = grid_collapsed.nodes[index];
                    if (0..=8).contains(&tile) {
                        vertex_configuration += 1;
                        connected += 1;
                    }
                }
            }
            if 0 < connected && connected < 8 {
                println!("{:08b}", vertex_configuration);
                vertex_configurations.push(vertex_configuration);
                new_node_indices.push(Some(new_node_index));
                new_node_index += 1;
                nodes[index] = true;
            } else {
                new_node_indices.push(None)
            }
        }

        // Create list of verts and edges without face data
        let mut vertices: Vec<Vertex> = Vec::with_capacity(new_node_index);
        let mut edges: Vec<Edge> = Vec::new();

        for (u, u_pos) in node_pos.clone().enumerate() {
            if let Some(u) = new_node_indices[u] {
                let neighbours: [Option<usize>; 6] = DIRECTIONS
                    .into_iter()
                    .enumerate()
                    .map(|(direction_index, dir)| {
                        if ivec3_in_bounds(u_pos + dir, IVec3::ZERO, size + 1) {
                            let v = ivec3_to_index(u_pos + dir, size + 1);
                            new_node_indices[v].filter(|&v| {
                                !Self::edge_occluded(Self::edge_configuration(
                                    vertex_configurations[u],
                                    vertex_configurations[v],
                                    direction_index,
                                ))
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                let mut edge_index = edges.len();
                edges.extend(
                    [
                        (IVec3::X, neighbours[0]),
                        (IVec3::Y, neighbours[2]),
                        (IVec3::Z, neighbours[4]),
                    ]
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, neighbour)| neighbour.1.map(|v| (index, neighbour.0, v)))
                    .map(|(_, dir, v)| Edge {
                        from: u,
                        to: v,
                        pos: 2 * u_pos + dir,
                        quads: Box::new([]),
                        tangent: ivec3_to_direction(dir).unwrap(),
                        cotangent: 0,
                    }),
                );
                let vertex_edges: [Option<usize>; 6] = neighbours
                    .into_iter()
                    .enumerate()
                    .map(|(i, neighbour)| {
                        if let Some(neighbour) = neighbour {
                            if i.rem_euclid(2) == 0 {
                                edge_index += 1;
                                Some(edge_index - 1)
                            } else {
                                vertices[neighbour].edges[i - 1]
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();

                vertices.push(Vertex {
                    pos: u_pos,
                    neighbours,
                    edges: vertex_edges,
                });
            }
        }

        // Create list of quads
        let mut quads: Vec<Quad> = Vec::new();
        let mut edge_quads = vec![Vec::new(); edges.len()];
        for (u, vertex) in vertices.iter().enumerate() {
            'quad_loop: for steps in [[0usize, 2, 1, 3], [2, 4, 3, 5], [4, 0, 5, 1]].into_iter() {
                let mut pos: IVec3 = IVec3::ZERO;
                let mut quad_edges: [usize; 4] = [0; 4];
                let mut quad_vertices: [usize; 4] = [0; 4];
                let mut v = u;
                let mut vertex = vertex;

                for (i, step) in steps.into_iter().enumerate() {
                    quad_vertices[i] = v;
                    pos += vertex.pos;
                    if let Some(next_v) = vertex.neighbours[step] {
                        quad_edges[i] = vertex.edges[step].unwrap();
                        v = next_v;
                        vertex = &vertices[v];
                    } else {
                        continue 'quad_loop;
                    }
                }
                let (occluded, normal) =
                    Self::quad_normal(vertex_configurations[quad_vertices[0]], steps[0], steps[1]);

                if !occluded {
                    for (step_index, e) in quad_edges.iter().enumerate() {
                        edge_quads[*e].push((steps[(step_index + 1).rem_euclid(4)], quads.len()));
                    }

                    quads.push(Quad {
                        pos,
                        normal,
                        tangent: steps[0],
                        cotangent: steps[1],
                        verts: quad_vertices,
                        edges: quad_edges,
                    });
                }
            }
        }

        for (edge_index, edge_quads) in edge_quads.into_iter().enumerate() {
            assert!(
                edge_quads.len() <= 2,
                "Degenerate facade edge with >2 faces"
            );
            edges[edge_index].quads = edge_quads.into_boxed_slice();
        }

        Self {
            vertices,
            edges,
            quads,
        }
    }
}
