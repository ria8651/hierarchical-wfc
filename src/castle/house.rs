use std::collections::{HashMap, HashSet};

use crate::{
    json::tileset::{ConstraintNodeModel, DagNodeModel, TileSetModel},
    wfc::{Superposition, TileSet},
};
use bevy::prelude::*;
use itertools::Itertools;

#[derive(Component, Debug)]
pub struct HouseTileset {
    pub assets: HashMap<String, TilesetAssets>,
    pub tile_count: usize,
    pub arc_types: usize,
    pub leaf_sources: Box<[usize]>,
    pub transformed_nodes: Box<[TransformedDagNode]>,
    pub constraints: Box<[Box<[Superposition]>]>,
    pub sematic_node_names: HashMap<String, usize>,
    pub associated_transformed_nodes: Box<[Box<[usize]>]>,
    pub leaf_families: Box<[(usize, Superposition)]>,
}

impl TileSet for HouseTileset {
    fn tile_count(&self) -> usize {
        self.tile_count
    }

    fn arc_types(&self) -> usize {
        self.arc_types
    }

    fn get_constraints(&self) -> Box<[Box<[Superposition]>]> {
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
    symmetries: Box<[usize]>,
    sockets: Box<[Option<String>]>,
    optional: Box<[bool]>,
}

#[derive(Debug, Clone)]
pub struct TransformedDagNode {
    pub source_node: usize,
    pub parents: Vec<usize>,
    pub children: Vec<usize>,
    pub symmetry: Box<[usize]>,
    pub sockets: Box<[Option<String>]>,
    pub required: usize,
}

#[derive(Debug)]
pub struct TilesetAssets {
    pub path: String,
    pub nodes: Vec<Option<String>>,
}

impl HouseTileset {
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
                    Self::traverse_dag_model(*index, node, names, adj, leaf);
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

    fn compose_symmetries(lhs: &[usize], rhs: &[usize]) -> Box<[usize]> {
        assert_eq!(lhs.len(), rhs.len());
        lhs.iter().map(|i| rhs[*i]).collect::<Box<[usize]>>()
    }

    fn get_matching_direction(dir: usize) -> usize {
        dir + 1 - 2 * dir.rem_euclid(2)
    }

    fn from_model(model: TileSetModel) -> Self {
        // Process data for symmetries and directions
        let directions: Box<[String]> = model.directions.into();
        let identity_symmetry = Self::identity_symmetry(directions.len());
        let symmetry_names = model
            .symmetries
            .iter()
            .enumerate()
            .map(|(index, (key, _value))| (key.clone(), index))
            .collect::<HashMap<String, usize>>();
        let symmetries = model
            .symmetries
            .values()
            .map(|sym| {
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

        // dbg!(&semantic_node_names);

        // Process semantic nodes
        let sematic_node_names_map = semantic_node_names
            .iter()
            .enumerate()
            .map(|(index, key)| (key.clone(), index))
            .collect::<HashMap<String, usize>>();

        // dbg!(&sematic_node_names_map);

        let semantic_nodes = model
            .semantic_nodes
            .iter()
            .map(|node| SemanticNode {
                sockets: directions
                    .iter()
                    .map(|dir| node.sockets.get(dir).cloned())
                    .collect::<Box<[Option<String>]>>(),
                symmetries: node
                    .symmetries
                    .iter()
                    .map(|sym| symmetry_names[sym])
                    .collect::<Box<[usize]>>(),
                optional: directions
                    .iter()
                    .map(|dir| node.optional.contains(dir))
                    .collect::<Box<[bool]>>(),
            })
            .collect::<Box<[SemanticNode]>>();

        // Load assets
        let assets = model
            .assets
            .iter()
            .map(|(asset_type, asset)| {
                let mut node_assets: Vec<Option<String>> = vec![None; semantic_nodes.len()];

                asset.nodes.iter().for_each(|(node_name, path)| {
                    let node_id = sematic_node_names_map
                        .get(node_name)
                        .expect("Asset with invalid semantic node name!");
                    node_assets[*node_id] = Some(path.clone());
                });
                (
                    asset_type.clone(),
                    TilesetAssets {
                        path: asset.path.clone(),
                        nodes: node_assets,
                    },
                )
            })
            .collect::<HashMap<String, TilesetAssets>>();

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

        // Compute allowed neighbours
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

                        let source_socket: &Option<String> =
                            &transformed_source.sockets[source_direction];
                        let target_socket: &Option<String> =
                            &transformed_target.sockets[target_direction];

                        println!("Constraint: {source:?} --- {target:?}");
                        println!("Direction: {source_direction:?} --- {target_direction:?}");
                        println!(
                            "Transformed Nodes: {transformed_source:?} --- {transformed_target:?}"
                        );

                        if source_socket.is_some()
                            && target_socket.is_some()
                            && (source_socket == &source.1 || source.1.is_none())
                            && (target_socket == &target.1 || target.1.is_none())
                        {
                            println!("TRUE\n");
                            allowed_neighbours[*transformed_source_index][source_direction]
                                .add_tile(*transformed_target_index);
                        } else {
                            println!("FALSE\n");
                        }
                    }
                }
            }
        }

        // Flatten constraints to concrete leaf nodes, this does not flatten properly:
        //    (a) <- allows -> (b)
        //   /  \
        // (c)  (d)
        // Will add only (c) -- allows -> (b), (d) -- allows (b)
        // We must restore symmetry later!

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
        Self::traverse_flatten_constraints(0, &mut allowed_neighbours, &transformed_nodes);

        let transformed_leaves = leaf_nodes
            .iter()
            .flat_map(|n| associated_transformed_nodes[*n].iter().copied())
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
        let mut leaf_allowed_neighbours = transformed_leaves
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
                    })
                    .collect::<Box<[_]>>()
            })
            .collect::<Box<[_]>>();

        // Restore symmetry to constraints
        for (from, allowed) in leaf_allowed_neighbours.clone().iter().enumerate() {
            for (dir, _) in directions.iter().enumerate() {
                for to in allowed[dir].tile_iter() {
                    leaf_allowed_neighbours[to][Self::get_matching_direction(dir)].add_tile(from);
                }
            }
        }

        // dbg!(&semantic_nodes);
        // dbg!(&semantic_node_names);
        // dbg!(&transformed_nodes);
        // dbg!(&associated_transformed_nodes);
        // dbg!(&transformed_leaves);
        // dbg!(&leaf_families);

        // println!("\nExtracted Constraints:");
        // for (from, to) in constraints.iter() {
        //     let none = "None".to_string();
        //     dbg!(from.0);
        //     println!(
        //         "\t{} ({}) -> {} ({})",
        //         semantic_node_names[from.0],
        //         from.1.as_ref().unwrap_or(&none),
        //         semantic_node_names[to.0],
        //         to.1.as_ref().unwrap_or(&none)
        //     );
        // }

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
            assets,
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
        parent_symmetry: &[usize],
        transformed_nodes: &mut Vec<TransformedDagNode>,
        associated_transformed_nodes: &mut Vec<Vec<usize>>,
        semantic_nodes: &[SemanticNode],
        adj: &Vec<Vec<usize>>,
        symmetries: &[Box<[usize]>],
    ) {
        let semantic_node = &semantic_nodes[node];

        let mut node_symmetries: HashSet<Box<[usize]>> = HashSet::new();
        let mut last_sym: Box<[usize]> = parent_symmetry.into();
        node_symmetries.insert(parent_symmetry.into());

        if let Some(sym) = semantic_node.symmetries.first() {
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

        let mut socket_configurations: HashSet<Box<[Option<String>]>> = HashSet::new();
        socket_configurations.extend(existing_socket_configurations.iter().map(|v| v.1.clone()));
        for sym in node_symmetries.iter() {
            let sockets = sym
                .iter()
                .map(|i| semantic_node.sockets[*i].clone())
                .collect::<Box<[Option<String>]>>();
            let required = sym
                .iter()
                .map(|i| ((!semantic_node.optional[*i]) as usize) << *i)
                .reduce(|p, n| p | n)
                .unwrap();
            if socket_configurations.insert(sockets.clone()) {
                let self_location_transformed_nodes = transformed_nodes.len();
                transformed_nodes.push(TransformedDagNode {
                    source_node: node,
                    parents: parent.map(|p| vec![p]).unwrap_or_default(),
                    children: vec![],
                    symmetry: sym.clone(),
                    sockets,
                    required,
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
                        sym,
                        transformed_nodes,
                        associated_transformed_nodes,
                        semantic_nodes,
                        adj,
                        symmetries,
                    );
                }
            } else if let Some(parent) = parent {
                if let Some(existing_index) =
                    existing_socket_configurations
                        .iter()
                        .find_map(|(index, existing_sockets)| {
                            if existing_sockets == &sockets {
                                Some(index)
                            } else {
                                None
                            }
                        })
                {
                    transformed_nodes[*existing_index].parents.push(parent)
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

    pub fn superposition_from_semantic_name(&self, semantic_string: String) -> Superposition {
        let node = *self.sematic_node_names.get(&semantic_string).unwrap();
        let transformed_nodes = &self.associated_transformed_nodes[node];

        let transformed_nodes = Superposition::from_iter_sized(
            transformed_nodes.iter().copied(),
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

    pub fn superposition_from_directions(&self, directions: usize) -> Superposition {
        let mut sp = Superposition::empty_sized(self.leaf_sources.len());
        for (leaf_id, node_id) in self.leaf_sources.iter().enumerate() {
            let node = &self.transformed_nodes[*node_id];
            let leaf_directions = node
                .sockets
                .iter()
                .enumerate()
                .map(|(index, socket)| (socket.is_some() as usize) << index)
                .reduce(|last, next| last | next)
                .unwrap();
            if leaf_directions & node.required == directions & node.required {
                sp.add_tile(leaf_id);
            }
        }
        sp
    }
}
