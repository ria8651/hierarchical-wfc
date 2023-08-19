use crate::{
    tools::{
        index_tools::{ivec3_in_bounds, ivec3_to_index},
        MeshBuilder,
    },
    wfc::{Superposition, WfcGraph},
};
use bevy::{
    math::{ivec3, vec3},
    prelude::*,
};

use super::LayoutGraphSettings;

pub enum VertexVariants {
    FlatTop,      // Vertex between edges on the same plane
    FlatSide,     //
    FlatBottom,   //
    TopCorner,    // Vertex is located in corner on block
    BottomCorner, //
    TopEdge,      // Vertex is located between exactly two blocks
    BottomEdge,   //
    GutterJoin,   // Between two blocks meeting the floor
    GutterBend,   // Located on bottom corner of one block and top corner of 4 blocks
    GutterOutlet, // Vertex located on the edge of one wall and the intersection of another wall with the floor
                  //     | /
                  //  ---O---
                  //     |
}

pub enum EdgeVariants {
    FlatTop,
    FlatSide,
    FlatBottom,

    CornerTop,
    CornerSide,
    CornerBottom,

    Gutter, // Bottom of wall meets floor
}

pub enum FaceVariants {
    Top,
    Side,
    Bottom,
}

pub struct FacadeVertex {
    pos: IVec3,
    neighbours: [Option<usize>; 6],
    edges: [Option<usize>; 6],
}

#[derive(Debug)]
pub struct FacadeEdge {
    pos: IVec3,
    from: usize,
    to: usize,
    left: usize,
    right: usize,
}
#[derive(Debug)]
pub struct FacadeQuad {
    pos: IVec3,
    verts: [usize; 4],
    edges: [usize; 4],
}

#[derive(Component)]
pub struct FacadePassSettings;

#[derive(Component)]
pub struct FacadePassData {
    vertices: Vec<FacadeVertex>,
    edges: Vec<FacadeEdge>,
    quads: Vec<FacadeQuad>,
}

impl FacadePassData {
    pub fn debug_vertex_mesh(&self, vertex_mesh: Mesh) -> Mesh {
        let mut vertex_mesh_builder = MeshBuilder::new();

        for vertex in self.vertices.iter() {
            vertex_mesh_builder.add_mesh(
                &vertex_mesh,
                Transform::from_translation(vertex.pos.as_vec3() * vec3(2.0, 3.0, 2.0)),
                0,
            );
        }

        vertex_mesh_builder.build()
    }

    pub fn debug_edge_mesh(&self, edge_mesh: Mesh) -> Mesh {
        let mut vertex_mesh_builder = MeshBuilder::new();

        for vertex in self.edges.iter() {
            vertex_mesh_builder.add_mesh(
                &edge_mesh,
                Transform::from_translation(vertex.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.5),
                0,
            );
        }

        vertex_mesh_builder.build()
    }

    pub fn debug_quad_mesh(&self, quad_mesh: Mesh) -> Mesh {
        let mut vertex_mesh_builder = MeshBuilder::new();

        for vertex in self.quads.iter() {
            vertex_mesh_builder.add_mesh(
                &quad_mesh,
                Transform::from_translation(vertex.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.25),
                0,
            );
        }

        vertex_mesh_builder.build()
    }

    pub fn from_layout(
        layout_data: &WfcGraph<usize>,
        layout_settings: &LayoutGraphSettings,
    ) -> Self {
        let mut nodes: Vec<bool> = vec![
            false;
            (layout_settings.x_size + 1)
                * (layout_settings.y_size + 1)
                * (layout_settings.z_size + 1)
        ];
        let size = ivec3(
            layout_settings.x_size as i32,
            layout_settings.y_size as i32,
            layout_settings.z_size as i32,
        );

        let node_pos = itertools::iproduct!(0..size.z + 1, 0..size.y + 1, 0..size.x + 1)
            .map(|(z, y, x)| ivec3(x, y, z));

        let mut new_node_indices: Vec<Option<usize>> = Vec::new();
        let mut new_node_index: usize = 0;

        for (index, pos) in node_pos.clone().enumerate() {
            let mut connected = 0;
            for delta in
                itertools::iproduct!(-1..=0, -1..=0, -1..=0).map(|(x, y, z)| ivec3(x, y, z))
            {
                let pos = pos + delta;
                if (0..size.x).contains(&pos.x)
                    && (0..size.y).contains(&pos.y)
                    && (0..size.z).contains(&pos.z)
                {
                    let index = pos.dot(ivec3(1, size.x, size.x * size.y)) as usize;

                    let tile = layout_data.nodes[index];
                    if (0..=8).contains(&tile) {
                        connected += 1;
                    }
                }
            }
            // let index = pos.dot(ivec3(1, size.x + 1, (size.x + 1) * (size.y + 1)));
            if 0 < connected && connected < 8 {
                new_node_indices.push(Some(new_node_index));
                new_node_index += 1;
                nodes[index as usize] = true;
            } else {
                new_node_indices.push(None)
            }
        }

        // Create list of verts with neighbours
        let mut vertices: Vec<FacadeVertex> = Vec::with_capacity(new_node_index);
        let mut edges: Vec<FacadeEdge> = Vec::new();

        for (u, u_pos) in node_pos.clone().enumerate() {
            if let Some(u) = new_node_indices[u] {
                let neighbours: [Option<usize>; 6] = DIRECTIONS
                    .into_iter()
                    .map(|dir: IVec3| {
                        if ivec3_in_bounds(u_pos + dir, IVec3::ZERO, size + 1) {
                            let v = ivec3_to_index(u_pos + dir, size + 1);
                            new_node_indices[v]
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
                    .map(|(index, neighbour)| {
                        if let Some(v) = neighbour.1 {
                            Some((index, neighbour.0, v))
                        } else {
                            None
                        }
                    })
                    .filter_map(|item| item)
                    .map(|(_, dir, v)| FacadeEdge {
                        from: u,
                        to: v,
                        pos: 2 * u_pos + dir,
                        left: 0,
                        right: 0,
                    }),
                );
                let vertex_edges: [Option<usize>; 6] = neighbours
                    .clone()
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

                vertices.push(FacadeVertex {
                    pos: u_pos,
                    neighbours,
                    edges: vertex_edges,
                });
            }
        }

        // Create list of quads
        let mut quads: Vec<FacadeQuad> = Vec::new();
        for (u, vertex) in vertices.iter().enumerate() {
            'quad_loop: for steps in [[0usize, 2, 1, 3], [2, 4, 3, 5], [4, 0, 5, 1]].into_iter() {
                let mut pos = IVec3::ZERO;
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

                quads.push(FacadeQuad {
                    pos,
                    verts: quad_vertices,
                    edges: quad_edges,
                });
            }
        }

        Self {
            vertices,
            edges,
            quads,
        }
    }
}

// #[derive(Reflect, Clone, Copy)]
// #[reflect(Default)]
// pub struct FacadeGraphSettings;

// impl FacadeGraphSettings {}

impl Default for FacadePassSettings {
    fn default() -> Self {
        Self {}
    }
}

const DIRECTIONS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

pub fn create_facade_graph<F: Clone>(
    _data: &FacadePassData,
    _settings: &FacadePassSettings,
) -> WfcGraph<Superposition> {
    WfcGraph {
        nodes: vec![],
        order: vec![],
        neighbors: vec![],
    }
}
