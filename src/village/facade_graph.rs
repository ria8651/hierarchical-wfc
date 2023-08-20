use std::{
    collections::{HashMap, HashSet},
    ops::Add,
};

use crate::{
    json::tileset::{ConstraintNodeModel, DagNodeModel, SemanticNodeModel, TileSetModel},
    tools::{
        index_tools::{ivec3_in_bounds, ivec3_to_index},
        MeshBuilder,
    },
    wfc::{Neighbour, Superposition, TileSet, WfcGraph},
};
use bevy::{
    math::{ivec3, vec3},
    prelude::*,
};
use itertools::Itertools;

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
    pub pos: IVec3,
    pub neighbours: [Option<usize>; 6],
    pub edges: [Option<usize>; 6],
}

#[derive(Debug)]
pub struct FacadeEdge {
    pub pos: IVec3,
    pub from: usize,
    pub to: usize,
    pub left: usize,
    pub right: usize,
    pub tangent: usize,
    pub cotangent: usize,
}
#[derive(Debug)]
pub struct FacadeQuad {
    pub pos: IVec3,
    pub verts: [usize; 4],
    pub edges: [usize; 4],
}

#[derive(Component)]
pub struct FacadePassSettings;

#[derive(Component)]
pub struct FacadePassData {
    pub vertices: Vec<FacadeVertex>,
    pub edges: Vec<FacadeEdge>,
    pub quads: Vec<FacadeQuad>,
}

impl FacadePassData {
    pub fn create_wfc_graph(&self, tileset: &FacadeTileset) -> WfcGraph<Superposition> {
        let vertex_superposition = tileset.superposition_from_string("vertex".to_string());
        let edge_superposition = tileset.superposition_from_string("edge".to_string());
        // let quad_superposition = tileset.superposition_from_string("quad".to_string());

        let num_verts = self.vertices.len();
        let num_edges = self.edges.len();
        let num_quads = self.quads.len();

        WfcGraph {
            nodes: [
                self.vertices
                    .iter()
                    .map(|_| vertex_superposition.clone())
                    .collect_vec(),
                self.edges
                    .iter()
                    .map(|_| edge_superposition.clone())
                    .collect_vec(),
                // self.quads
                //     .iter()
                //     .map(|_| quad_superposition.clone())
                //     .collect_vec(),
            ]
            .into_iter()
            .flat_map(|v| v.into_iter())
            .collect_vec(),
            order: Vec::new(),
            neighbors: [
                self.vertices
                    .iter()
                    .map(|vert| {
                        vert.edges
                            .iter()
                            .enumerate()
                            .filter_map(|(index, edge)| {
                                if let Some(edge) = edge {
                                    Some(Neighbour {
                                        arc_type: index,
                                        index: edge + num_verts,
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect_vec()
                    })
                    .collect_vec(),
                self.edges
                    .iter()
                    .map(|edge| {
                        vec![
                            Neighbour {
                                arc_type: FacadeTileset::get_matching_direction(edge.tangent),
                                index: edge.from,
                            },
                            Neighbour {
                                arc_type: edge.tangent,
                                index: edge.to,
                            },
                            // Neighbour {
                            //     arc_type: 0,
                            //     index: num_verts + num_edges + edge.left,
                            // },
                            // Neighbour {
                            //     arc_type: 0,
                            //     index: num_verts + num_edges + edge.right,
                            // },
                        ]
                    })
                    .collect_vec(),
                // self.quads
                //     .iter()
                //     .map(|quad| {
                //         let tangent =
                //             (self.edges[quad.edges[1]].pos - self.edges[quad.edges[0]].pos);
                //         let tangent = tangent.signum()
                //             * [IVec3::X, IVec3::Y, IVec3::Z][tangent.max_element() as usize];
                //         let tangent = FacadeTileset::ivec3_to_direction(tangent).unwrap();

                //         let cotangent =
                //             (self.edges[quad.edges[3]].pos - self.edges[quad.edges[1]].pos);
                //         let cotangent = cotangent.signum()
                //             * [IVec3::X, IVec3::Y, IVec3::Z][cotangent.max_element() as usize];
                //         let cotangent = FacadeTileset::ivec3_to_direction(cotangent).unwrap();

                //         vec![
                //             Neighbour {
                //                 arc_type: FacadeTileset::get_matching_direction(tangent),
                //                 index: quad.edges[0] + num_verts,
                //             },
                //             Neighbour {
                //                 arc_type: tangent,
                //                 index: quad.edges[1] + num_verts,
                //             },
                //             // Neighbour {
                //             //     arc_type: FacadeTileset::get_matching_direction(cotangent),
                //             //     index: quad.edges[2] + num_verts,
                //             // },
                //             // Neighbour {
                //             //     arc_type: cotangent,
                //             //     index: quad.edges[3] + num_verts,
                //             // },
                //         ]
                //     })
                //     .collect_vec(),
            ]
            .into_iter()
            .flat_map(|v| v.into_iter())
            .collect_vec(),
        }
    }

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
                        tangent: FacadeTileset::ivec3_to_direction(dir).unwrap(),
                        cotangent: 0,
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

// pub fn create_facade_graph<F: Clone>(
//     data: &FacadePassData,
//     tileset: &FacadeTileset,
//     _settings: &FacadePassSettings,
// ) -> WfcGraph<Superposition> {
//     // let tileset = FacadeTileset::from_asset("semantics/facade.json");
//     WfcGraph {
//         nodes: data.init_tiles(tileset),
//         order: Vec::new(),
//         neighbors: data.init_neighbours(),
//     }
// }

#[derive(Component, Debug)]
pub struct FacadeTileset {
    tile_count: usize,
    arc_types: usize,
    leaf_sources: Box<[usize]>,
    transformed_nodes: Box<[TransformedDagNode]>,
    constraints: Vec<Vec<Superposition>>,
    sematic_node_names: HashMap<String, usize>,
    associated_transformed_nodes: Box<[Box<[usize]>]>,
    leaf_families: Box<[(usize, Superposition)]>,
}

impl TileSet for FacadeTileset {
    type GraphSettings = FacadePassSettings;

    fn tile_count(&self) -> usize {
        self.tile_count
    }

    fn arc_types(&self) -> usize {
        self.arc_types
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> WfcGraph<Superposition> {
        todo!()
    }

    fn get_constraints(&self) -> Vec<Vec<Superposition>> {
        self.constraints.clone()
    }

    fn get_weights(&self) -> Vec<u32> {
        vec![100; self.tile_count]
    }

    fn get_tile_paths(&self) -> Vec<String> {
        todo!()
    }
}

#[derive(Debug)]
struct SemanticNode {
    sockets: Box<[String]>,
    symmetries: Box<[usize]>,
    assets: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct TransformedDagNode {
    source_node: usize,
    parents: Vec<usize>,
    children: Vec<usize>,
    symmetry: Box<[usize]>,
    sockets: Box<[String]>,
}

impl FacadeTileset {
    pub fn from_asset(asset_path: impl Into<String>) -> Self {
        // TODO: handle errors
        Self::from_model(TileSetModel::from_asset(asset_path.into()).unwrap())
    }

    fn traverse_dag_model(
        current: usize,
        node: DagNodeModel,
        names: &HashMap<String, usize>,
        adj: &mut Vec<Vec<usize>>,
        leaf: &mut Vec<usize>,
    ) {
        match node {
            DagNodeModel::Meta(nodes) => {
                adj[current].extend(nodes.keys().map(|k| names.get(k).unwrap()));

                let new_nodes = nodes
                    .into_iter()
                    .map(|(key, value)| (names.get(&key).unwrap(), value));

                for (index, node) in new_nodes {
                    Self::traverse_dag_model(index.clone(), node, names, adj, leaf);
                }
            }
            DagNodeModel::Leaf => {
                leaf.push(current);
            }
        }
    }

    fn identity_symmetry(num_dirs: usize) -> Box<[usize]> {
        (0..num_dirs).collect::<Box<[usize]>>()
    }

    fn compose_symmetries(lhs: &Box<[usize]>, rhs: &Box<[usize]>) -> Box<[usize]> {
        assert_eq!(lhs.len(), rhs.len());
        lhs.iter().map(|i| rhs[*i]).collect::<Box<[usize]>>()
    }

    fn get_matching_direction(dir: usize) -> usize {
        return dir + 1 - 2 * dir.rem_euclid(2);
    }

    fn ivec3_to_direction(dir: IVec3) -> Option<usize> {
        match dir {
            IVec3::X => Some(0),
            IVec3::NEG_X => Some(1),
            IVec3::Y => Some(2),
            IVec3::NEG_Y => Some(3),
            IVec3::Z => Some(4),
            IVec3::NEG_Z => Some(5),
            _ => None,
        }
    }

    fn from_model(model: TileSetModel) -> Self {
        // Process data for symmetries and directions
        let directions: Box<[String]> = model.directions.into();
        let identity_symmetry = Self::identity_symmetry(directions.len());
        let symmetry_names = model
            .symmetries
            .iter()
            .enumerate()
            .map(|(index, (key, value))| (key.clone(), index))
            .collect::<HashMap<String, usize>>();
        let symmetries = model
            .symmetries
            .iter()
            .map(|(_, sym)| {
                directions
                    .iter()
                    .map(|k| {
                        let new_dir = sym.get(k).unwrap_or(k);
                        directions.iter().position(|dir| dir == new_dir).unwrap()
                    })
                    .collect::<Box<[usize]>>()
            })
            .collect::<Box<[Box<[usize]>]>>();

        let semantic_node_names: Vec<String> =
            Vec::from_iter(model.semantic_nodes.iter().map(|node| node.label.clone()));

        dbg!(&semantic_node_names);

        // Process semantic nodes
        let sematic_node_names_map = semantic_node_names
            .iter()
            .enumerate()
            .map(|(index, key)| (key.clone(), index))
            .collect::<HashMap<String, usize>>();

        dbg!(&sematic_node_names_map);

        let semantic_nodes = model
            .semantic_nodes
            .iter()
            .map(|node| SemanticNode {
                sockets: directions
                    .iter()
                    .map(|dir| match node.sockets.get(dir) {
                        Some(string) => string.clone(),
                        None => "".to_string(),
                    })
                    .collect::<Box<[String]>>(),
                symmetries: node
                    .symmetries
                    .iter()
                    .map(|sym| symmetry_names[sym])
                    .collect::<Box<[usize]>>(),
                assets: node.assets.clone(),
            })
            .collect::<Box<[SemanticNode]>>();

        // Traverse DAG to build in new format and extract information
        let mut leaf_nodes: Vec<usize> = Vec::new();
        let mut semantic_dag_adj: Vec<Vec<usize>> = vec![Vec::new(); sematic_node_names_map.len()];
        Self::traverse_dag_model(
            0,
            model.semantic_dag,
            &sematic_node_names_map,
            &mut semantic_dag_adj,
            &mut leaf_nodes,
        );

        // Traverse new DAG and compute symmetries of tiles
        let mut associated_transformed_nodes: Vec<Vec<usize>> =
            vec![Vec::new(); semantic_nodes.len()];
        let mut transformed_nodes: Vec<TransformedDagNode> =
            Vec::with_capacity(semantic_nodes.len());

        Self::traverse_symmetries(
            0,
            None,
            &identity_symmetry.clone(),
            &mut transformed_nodes,
            &mut associated_transformed_nodes,
            &semantic_nodes,
            &semantic_dag_adj,
            &symmetries,
        );

        for parent in transformed_nodes
            .iter()
            .flat_map(|node| node.parents.iter())
        {
            assert!(parent < &transformed_nodes.len(), "Failed Assert A!!!!!");
        }

        // Build constraints for all DAG nodes

        let mut constraints: Box<[((usize, Option<String>), (usize, Option<String>))]> =
            vec![((0, None), (0, None)); 2 * model.constraints.len()].into_boxed_slice();

        for (index, [u, v]) in model.constraints.into_iter().enumerate() {
            let constraint = (
                match u {
                    ConstraintNodeModel::Node(node) => {
                        (*sematic_node_names_map.get(&node).unwrap(), None)
                    }

                    ConstraintNodeModel::NodeSocket { node, socket } => {
                        (*sematic_node_names_map.get(&node).unwrap(), Some(socket))
                    }
                },
                match v {
                    ConstraintNodeModel::Node(node) => {
                        (*sematic_node_names_map.get(&node).unwrap(), None)
                    }

                    ConstraintNodeModel::NodeSocket { node, socket } => {
                        (*sematic_node_names_map.get(&node).unwrap(), Some(socket))
                    }
                },
            );
            constraints[2 * index] = constraint.clone();
            constraints[2 * index + 1] = (constraint.1, constraint.0);
        }

        let mut allowed_neighbours: Box<[Box<[Superposition]>]> = vec![
                vec![Superposition::empty_sized(transformed_nodes.len()); directions.len()]
                    .into_boxed_slice();
                transformed_nodes.len()
            ]
        .into_boxed_slice();
        for (source, target) in constraints.iter() {
            for transformed_source_index in associated_transformed_nodes[source.0].iter() {
                let transformed_source = &transformed_nodes[*transformed_source_index];
                for transformed_target_index in associated_transformed_nodes[target.0].iter() {
                    let transformed_target = &transformed_nodes[*transformed_target_index];
                    for (source_direction, _) in directions.iter().enumerate() {
                        let target_direction = Self::get_matching_direction(source_direction);

                        let source_socket = &transformed_source.sockets[source_direction];
                        let target_socket = &transformed_target.sockets[target_direction];

                        if (Some(source_socket) == source.1.as_ref() || source.1 == None)
                            && (Some(target_socket) == target.1.as_ref() || target.1 == None)
                        {
                            allowed_neighbours[*transformed_source_index][source_direction]
                                .add_tile(*transformed_target_index);
                        }
                    }
                }
            }
        }

        // Flatten constraints to concrete leaf nodes
        Self::traverse_flatten_constraints(0, &mut allowed_neighbours, &transformed_nodes);

        let transformed_leaves = leaf_nodes
            .iter()
            .flat_map(|n| associated_transformed_nodes[*n].iter().map(|l| *l))
            .collect_vec();

        for parent in transformed_nodes
            .iter()
            .flat_map(|node| node.parents.iter())
        {
            assert!(parent < &transformed_nodes.len(), "Failed Assert B!!!!!");
        }

        let leaf_families = transformed_leaves
            .iter()
            .map(|leaf| {
                let mut family = Superposition::empty_sized(transformed_nodes.len());
                Self::traverse_create_family_mask(*leaf, &mut family, &transformed_nodes);
                (*leaf, family)
            })
            .collect::<Box<[(usize, Superposition)]>>();

        for transformed_leaf in transformed_leaves.iter() {
            let transformed_leaf = *transformed_leaf;
            for (direction, _) in directions.iter().enumerate() {
                for (leaf, family) in leaf_families.iter() {
                    let allowed = allowed_neighbours[transformed_leaf][direction];
                    if Superposition::intersect(&allowed, family).count_bits() > 0 {
                        allowed_neighbours[transformed_leaf][direction].add_tile(*leaf);
                    }
                }
            }
        }

        // Strip out non-leaves
        let leaf_allowed_neighbours = transformed_leaves
            .iter()
            .map(|leaf| {
                allowed_neighbours[*leaf]
                    .iter()
                    .map(|sp| {
                        let mut new_sp = Superposition::empty_sized(transformed_leaves.len());
                        for (leaf_id, transformed_id) in transformed_leaves.iter().enumerate() {
                            if sp.contains(*transformed_id) {
                                new_sp.add_tile(leaf_id)
                            }
                        }
                        new_sp
                        // Superposition::from_iter(transformed_leaves.iter().filter_map(|leaf| {
                        //     if sp.contains(*leaf) {
                        //         Some(*leaf)
                        //     } else {
                        //         None
                        //     }
                        // }))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // dbg!(&semantic_nodes);
        dbg!(&semantic_node_names);
        dbg!(&associated_transformed_nodes);
        dbg!(&transformed_leaves);
        dbg!(&leaf_families);

        println!("\nExtracted Constraints:");
        for (from, to) in constraints.iter() {
            let none = "None".to_string();
            println!(
                "\t{} ({}) -> {} ({})",
                semantic_node_names[transformed_nodes[from.0].source_node],
                from.1.as_ref().unwrap_or(&none),
                semantic_node_names[transformed_nodes[to.0].source_node],
                to.1.as_ref().unwrap_or(&none)
            );
        }

        println!("\nGenerated allowed neighbours:");
        for (index, allowed) in allowed_neighbours.iter().enumerate() {
            println!(
                "\t{}",
                semantic_node_names[transformed_nodes[index].source_node]
            );
            for (dir, allowed) in allowed.iter().enumerate() {
                println!("\t\t{}: {}", dir, allowed);
            }
        }

        println!("\nExtracted allowed leaf neighbours:");
        for (index, allowed) in leaf_allowed_neighbours.iter().enumerate() {
            println!(
                "\t{}",
                semantic_node_names[transformed_nodes[transformed_leaves[index]].source_node]
            );
            for (dir, allowed) in allowed.iter().enumerate() {
                println!("\t\t{}: {}", dir, allowed);
            }
        }

        Self {
            arc_types: directions.len(),
            tile_count: transformed_leaves.len(),
            leaf_sources: transformed_leaves.into_boxed_slice(),
            transformed_nodes: transformed_nodes.into_boxed_slice(),
            constraints: leaf_allowed_neighbours,
            sematic_node_names: sematic_node_names_map,
            associated_transformed_nodes: associated_transformed_nodes
                .into_iter()
                .map(|associated| associated.into_boxed_slice())
                .collect::<Box<[_]>>(),
            leaf_families,
        }
    }

    fn traverse_create_family_mask(
        node: usize,
        mask: &mut Superposition,
        transformed_nodes: &Vec<TransformedDagNode>,
    ) {
        mask.add_tile(node);

        for parent in transformed_nodes[node].parents.iter() {
            Self::traverse_create_family_mask(*parent, mask, transformed_nodes);
        }
    }

    fn traverse_flatten_constraints(
        node: usize,
        allowed_neighbours: &mut Box<[Box<[Superposition]>]>,
        transformed_nodes: &Vec<TransformedDagNode>,
    ) {
        let allowed = allowed_neighbours[node].clone();
        let transformed_node = transformed_nodes[node].clone();

        for child in transformed_node.children.iter() {
            for (dir, allowed) in allowed.iter().enumerate() {
                allowed_neighbours[*child][dir].add_other(allowed);
            }
        }
        for child in transformed_node.children.iter() {
            Self::traverse_flatten_constraints(*child, allowed_neighbours, transformed_nodes);
        }
    }

    fn traverse_symmetries(
        node: usize,
        parent: Option<usize>,
        parent_symmetry: &Box<[usize]>,
        transformed_nodes: &mut Vec<TransformedDagNode>,
        associated_transformed_nodes: &mut Vec<Vec<usize>>,
        semantic_nodes: &Box<[SemanticNode]>,
        adj: &Vec<Vec<usize>>,
        symmetries: &Box<[Box<[usize]>]>,
    ) {
        let semantic_node = &semantic_nodes[node];

        let mut node_symmetries: HashSet<Box<[usize]>> = HashSet::new();
        let mut last_sym = parent_symmetry.clone();
        node_symmetries.insert(parent_symmetry.clone());

        if let Some(sym) = semantic_node.symmetries.get(0) {
            let current_symmetry = &symmetries[*sym];
            loop {
                let next_sym = Self::compose_symmetries(current_symmetry, &last_sym);
                if !node_symmetries.insert(next_sym.clone()) {
                    break;
                }
                last_sym = next_sym;
            }
        }

        let existing_socket_configurations = associated_transformed_nodes[node]
            .iter()
            .map(|i| (*i, transformed_nodes[*i].sockets.clone()))
            .collect_vec();

        let mut socket_configurations: HashSet<Box<[String]>> = HashSet::new();
        socket_configurations.extend(existing_socket_configurations.iter().map(|v| v.1.clone()));
        for sym in node_symmetries.iter() {
            let sockets = sym
                .iter()
                .map(|i| semantic_node.sockets[*i].to_string())
                .collect::<Box<[String]>>();
            if socket_configurations.insert(sockets.clone()) {
                let self_location_transformed_nodes = transformed_nodes.len();
                transformed_nodes.push(TransformedDagNode {
                    source_node: node,
                    parents: parent.and_then(|p| Some(vec![p])).unwrap_or(vec![]),
                    children: vec![],
                    symmetry: sym.clone(),
                    sockets,
                });
                associated_transformed_nodes[node].push(self_location_transformed_nodes);
                if let Some(parent) = parent {
                    transformed_nodes[parent]
                        .children
                        .push(self_location_transformed_nodes);
                }

                for child in adj[node].iter() {
                    Self::traverse_symmetries(
                        *child,
                        Some(self_location_transformed_nodes),
                        &sym,
                        transformed_nodes,
                        associated_transformed_nodes,
                        semantic_nodes,
                        adj,
                        symmetries,
                    );
                }
            } else {
                if let Some(parent) = parent {
                    if let Some(existing_index) = existing_socket_configurations.iter().find_map(
                        |(index, existing_sockets)| {
                            if existing_sockets == &sockets {
                                Some(index)
                            } else {
                                None
                            }
                        },
                    ) {
                        transformed_nodes[*existing_index].parents.push(parent)
                    }
                }
            }
        }
    }

    pub fn get_leaf_semantic_name(&self, leaf_id: usize) -> String {
        let transformed_id = self.leaf_sources[leaf_id];
        let transformed_node = &self.transformed_nodes[transformed_id];
        let source_id = transformed_node.source_node;

        self.sematic_node_names
            .iter()
            .find_map(|(k, v)| if v == &source_id { Some(k) } else { None })
            .unwrap_or(&"Unknown".to_string())
            .clone()
    }

    pub fn superposition_from_string(&self, semantic_string: String) -> Superposition {
        let node = *self.sematic_node_names.get(&semantic_string).unwrap();
        let transformed_nodes = &self.associated_transformed_nodes[node];

        let transformed_nodes = Superposition::from_iter_sized(
            transformed_nodes.iter().map(|n| *n),
            self.transformed_nodes.len(),
        );

        let possible_leaves = self.leaf_families.iter().enumerate().filter_map(
            |(leaf_id, (_transformed_id, family))| {
                if Superposition::intersect(family, &transformed_nodes).count_bits() > 0 {
                    Some(leaf_id)
                } else {
                    None
                }
            },
        );
        Superposition::from_iter_sized(possible_leaves, self.leaf_sources.len())
    }
}
