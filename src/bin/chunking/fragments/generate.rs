use bevy::{
    math::{uvec3, vec3},
    prelude::*,
    utils::{HashMap, HashSet},
};
use hierarchical_wfc::{
    castle::LayoutTileset,
    graphs::regular_grid_3d,
    tools::index_tools::uvec3_to_index,
    wfc::{Neighbour, Superposition, TileSet, WaveFunctionCollapse, WfcGraph},
};
use itertools::{iproduct, Itertools};
use rand::{rngs::StdRng, SeedableRng};

type NodeKey = IVec3;
type EdgeKey = IVec3;
type FaceKey = IVec3;

#[derive(Debug)]
pub enum NodeFragmentEntry {
    Generating,
    Generated(Entity),
}

#[derive(Debug)]
pub enum EdgeFragmentEntry {
    Waiting(HashSet<IVec3>),
    Generated(Entity),
}

#[derive(Debug)]
pub enum FaceFragmentEntry {
    Waiting(HashSet<IVec3>),
    Generated(Entity),
}

#[derive(Resource, Default)]
pub struct FragmentTable {
    loaded_nodes: HashMap<NodeKey, NodeFragmentEntry>,
    loaded_edges: HashMap<EdgeKey, EdgeFragmentEntry>,
    loaded_faces: HashMap<FaceKey, FaceFragmentEntry>,

    edges_waiting_on_node: HashMap<NodeKey, HashSet<EdgeKey>>,
    faces_waiting_by_edges: HashMap<EdgeKey, HashSet<FaceKey>>,
}

#[derive(Event)]
pub enum ChunkLoadEvent {
    Load(IVec3),
}

pub enum ChunkEntry {
    Waiting,
}

#[derive(Resource, Default)]
pub struct ChunkTable {
    loaded: HashMap<IVec3, ChunkEntry>,
}

#[derive(Event, Clone, Copy)]
pub enum FragmentGenerateEvent {
    Node(IVec3),
    Edge(IVec3),
    Face(IVec3),
}

#[derive(Event)]
pub enum FragmentGeneratedEvent {
    Node(IVec3),
    Edge(IVec3),
    Face(IVec3),
}

#[derive(Resource)]
pub struct LayoutSettings {
    pub tileset: LayoutTileset,
    pub settings: regular_grid_3d::GraphSettings,
}

#[derive(Component)]
pub struct CollapsedData {
    pub graph: WfcGraph<usize>,
}

#[derive(Event)]
pub struct ChunkGenerateEvent(IVec3);

#[derive(Component)]
pub struct GenerateDebugMarker;

#[derive(Resource, Default)]
pub struct GenerationDebugSettings {
    pub debug_fragment_nodes: bool,
    pub debug_fragment_edges: bool,
    pub debug_fragment_faces: bool,
    pub debug_chunks: bool,
}

#[derive(Component)]
pub struct ChunkMarker;

#[derive(Component)]
pub struct FragmentMarker;

pub mod systems {
    use std::ops::Div;

    use crate::fragments::graph_operations::{graph_merge, subgraph_with_positions};

    use super::*;
    use bevy::{math::ivec3, prelude::*};
    use hierarchical_wfc::graphs::regular_grid_3d::{GraphData, GraphSettings};

    /// Transforms chunk load events into fragments which are registered for generation in the fragment table
    pub fn transform_chunk_loads(
        mut ev_load_chunk: EventReader<ChunkLoadEvent>,
        mut ev_generate_fragment: EventWriter<FragmentGenerateEvent>,
        mut chunk_table: ResMut<ChunkTable>,
        mut fragment_table: ResMut<FragmentTable>,
    ) {
        for load_chunk in ev_load_chunk.iter() {
            if let ChunkLoadEvent::Load(chunk_pos) = load_chunk {
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

                let face_loaded =
                    faces_pos.map(|pos| match fragment_table.loaded_faces.get(&pos) {
                        Some(FaceFragmentEntry::Generated(_)) => true,
                        _ => false,
                    });

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
                        // Keep track of fragments waiting for this node
                        let waiting_on_node = fragment_table
                            .edges_waiting_on_node
                            .entry(node)
                            .or_insert(HashSet::new());

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

    #[derive(Default)]
    pub struct FragmentQueue {
        queue: Vec<FragmentGenerateEvent>,
    }

    /// Dispatches fragment generation in response to incoming generation events
    pub fn generate_fragments(
        mut commands: Commands,
        layout_settings: Res<LayoutSettings>,
        mut ev_generate_fragment: EventReader<FragmentGenerateEvent>,
        mut generate_fragment_queue: Local<FragmentQueue>,
        mut fragment_table: ResMut<FragmentTable>,
        debug_settings: ResMut<GenerationDebugSettings>,
        q_fragments: Query<(
            &regular_grid_3d::GraphSettings,
            &regular_grid_3d::GraphData,
            &CollapsedData,
        )>,
    ) {
        const FRAGMENT_NODE_PADDING: u32 = 4;
        const FRAGMENT_EDGE_PADDING: u32 = 4;
        const FRAGMENT_FACE_SIZE: u32 = 32;
        const NODE_RADIUS: i32 = FRAGMENT_EDGE_PADDING as i32 + FRAGMENT_NODE_PADDING as i32;

        let tileset = &layout_settings.tileset;
        let weights = tileset.get_weights();
        let constraints = tileset.get_constraints();
        let fill_with = Superposition::filled(tileset.tile_count());

        for ev in ev_generate_fragment.iter() {
            generate_fragment_queue.queue.push(ev.clone());
        }

        let queue = generate_fragment_queue.queue.clone();
        generate_fragment_queue.queue.clear();

        for ev in queue {
            let event = ev.clone();
            match event {
                FragmentGenerateEvent::Node(node_pos) => {
                    let (data, mut graph) = regular_grid_3d::create_graph(
                        &regular_grid_3d::GraphSettings {
                            size: uvec3(
                                2 * (FRAGMENT_NODE_PADDING + FRAGMENT_EDGE_PADDING),
                                layout_settings.settings.size.y,
                                2 * (FRAGMENT_NODE_PADDING + FRAGMENT_EDGE_PADDING),
                            ),
                            spacing: layout_settings.settings.spacing,
                        },
                        &|(_, _)| Superposition::filled(layout_settings.tileset.tile_count()),
                    );

                    let seed = node_pos.to_array();
                    let mut seed: Vec<u8> = seed.map(|i| i.to_be_bytes()).concat().into();
                    seed.extend([0u8; 20].into_iter());
                    let seed: [u8; 32] = seed.try_into().unwrap();

                    WaveFunctionCollapse::collapse(
                        &mut graph,
                        &constraints,
                        &weights,
                        &mut StdRng::from_seed(seed),
                    );

                    if let Ok(result) = graph.validate() {
                        let mut fragment_commands = commands.spawn((
                            FragmentMarker,
                            Transform::from_translation(
                                node_pos.as_vec3()
                                    * FRAGMENT_FACE_SIZE as f32
                                    * layout_settings.settings.spacing
                                    - vec3(1.0, 0.0, 1.0)
                                        * (FRAGMENT_NODE_PADDING + FRAGMENT_EDGE_PADDING) as f32
                                        * layout_settings.settings.spacing,
                            ),
                            layout_settings.settings.clone(),
                            data,
                            CollapsedData { graph: result },
                        ));
                        if debug_settings.debug_fragment_nodes {
                            fragment_commands.insert(GenerateDebugMarker);
                        }
                        let fragment = fragment_commands.id();

                        fragment_table
                            .loaded_nodes
                            .insert(node_pos, NodeFragmentEntry::Generated(fragment));

                        for edge in fragment_table
                            .edges_waiting_on_node
                            .remove(&node_pos)
                            .unwrap()
                        {
                            if let Some(EdgeFragmentEntry::Waiting(waiting)) =
                                fragment_table.loaded_edges.get_mut(&edge)
                            {
                                assert!(waiting.remove(&node_pos));
                                if waiting.is_empty() {
                                    generate_fragment_queue
                                        .queue
                                        .push(FragmentGenerateEvent::Edge(edge));
                                }
                            } else {
                                panic!();
                            }
                        }
                    } else {
                        panic!();
                    }
                }
                FragmentGenerateEvent::Edge(edge_pos) => {
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
                        FRAGMENT_EDGE_PADDING as i32 * edge_normal
                            - FRAGMENT_EDGE_PADDING as i32 * edge_tangent,
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
                        .map(|pos| {
                            pos + edge_normal * FRAGMENT_FACE_SIZE as i32
                                - ivec3(1, 0, 1) * NODE_RADIUS
                        })
                        .collect::<Box<[IVec3]>>();

                    let (merged_graph, merged_positions) = graph_merge(
                        (&node_start.2.graph, &node_start_positions),
                        (&node_end.2.graph, &node_end_positions),
                        &|a: Option<&usize>, b| a.or(b).unwrap().clone(),
                    );

                    let (edge_data, edge_graph) =
                        regular_grid_3d::create_cuboid(edge_volume.0, edge_volume.1, &|(_, _)| {
                            fill_with.clone()
                        });

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
                        &constraints,
                        &weights,
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
                FragmentGenerateEvent::Face(face_pos) => {
                    // Origin of faces in the edge coordinate system

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

                    let edge_entity_ids =
                        edges.map(
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
                        &constraints,
                        &weights,
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
            }
        }
    }

    /// Outputs chunk generation events from fragments.
    pub fn apply_loaded_fragments(
        mut ev_generated_fragment: EventReader<FragmentGeneratedEvent>,
        mut ev_generate_chunk: EventWriter<ChunkGenerateEvent>,
        mut loaded_chunks: ResMut<ChunkTable>,
    ) {
    }

    /// Generates chunk entities in response to chunk generate events.
    pub fn generate_chunks(
        mut commands: Commands,
        mut ev_generate_chunk: EventReader<ChunkGenerateEvent>,
        loaded_fragments: ResMut<FragmentTable>,
        mut loaded_chunks: ResMut<ChunkTable>,
        debug_settings: Res<GenerationDebugSettings>,
        q_fragments: Query<(
            &regular_grid_3d::GraphSettings,
            &regular_grid_3d::GraphData,
            &CollapsedData,
        )>,
        layout_settings: Res<LayoutSettings>,
    ) {
    }
}

pub struct GenerationPlugin;
impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                systems::apply_loaded_fragments,
                systems::generate_chunks,
                systems::generate_fragments,
                systems::transform_chunk_loads,
            ),
        )
        .insert_resource(LayoutSettings {
            settings: regular_grid_3d::GraphSettings {
                size: uvec3(8, 1, 8),
                spacing: vec3(2.0, 3.0, 2.0),
            },
            tileset: LayoutTileset,
        })
        .init_resource::<ChunkTable>()
        .init_resource::<FragmentTable>()
        .init_resource::<GenerationDebugSettings>()
        .add_event::<FragmentGenerateEvent>()
        .add_event::<FragmentGeneratedEvent>()
        .add_event::<ChunkLoadEvent>()
        .add_event::<ChunkGenerateEvent>();
    }
}
