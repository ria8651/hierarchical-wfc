use crate::fragments::{
    generate::FragmentLocation,
    plugin::ChunkEntry,
    table::{
        EdgeFragmentEntry, EdgeFragmentStatus, EdgeKey, FaceFragmentEntry, FaceFragmentStatus,
        FaceKey, NodeFragmentEntry, NodeFragmentStatus, NodeKey,
    },
};

use bevy::{prelude::*, utils::HashSet};
use std::sync::Arc;
use tokio::runtime;

use itertools::Itertools;

use super::{
    generate::{
        generate_fragments, FragmentDestroyEvent, FragmentInstantiateEvent, FragmentSettings,
        WfcConfig,
    },
    plugin::{ChunkLoadEvent, ChunkTable, FragmentGenerateEvent},
    table::FragmentTable,
};
use tokio::sync::{broadcast, mpsc, RwLock};

pub struct AsyncTaskChannels {
    pub tx_generate_fragment: broadcast::Sender<FragmentGenerateEvent>,
    pub tx_instantiate_fragment: broadcast::Sender<FragmentInstantiateEvent>,
    pub tx_destroy_fragment: broadcast::Sender<FragmentDestroyEvent>,
}

#[derive(Resource)]
pub struct AsyncWorld {
    channels: Arc<AsyncTaskChannels>,
    pub tx_chunk_load: mpsc::Sender<ChunkLoadEvent>,
    pub rx_fragment_instantiate: broadcast::Receiver<FragmentInstantiateEvent>,
    pub rx_fragment_destroy: broadcast::Receiver<FragmentDestroyEvent>,
    fragment_table: Arc<RwLock<FragmentTable>>,
    chunk_table: Arc<RwLock<ChunkTable>>,
    wfc_config: Arc<RwLock<WfcConfig>>,
    rt: Arc<runtime::Runtime>,
}
impl Default for AsyncWorld {
    fn default() -> Self {
        let (tx_chunk_load, rx_chunk_load) = mpsc::channel(10);

        let fragment_table = RwLock::new(FragmentTable::default());
        let chunk_table = RwLock::new(ChunkTable::default());

        let (tx_inst_frag, rx_inst_frag) = broadcast::channel(100);
        let (tx_dest_frag, rx_dest_frag) = broadcast::channel(100);
        let world = AsyncWorld {
            channels: Arc::new(AsyncTaskChannels {
                tx_generate_fragment: broadcast::channel(100).0,
                tx_instantiate_fragment: tx_inst_frag,
                tx_destroy_fragment: tx_dest_frag,
            }),
            chunk_table: Arc::new(chunk_table),
            fragment_table: Arc::new(fragment_table),
            tx_chunk_load,
            wfc_config: Arc::new(RwLock::new(WfcConfig::default())),
            rx_fragment_instantiate: rx_inst_frag,
            rx_fragment_destroy: rx_dest_frag,
            rt: Arc::new(runtime::Runtime::new().unwrap()),
        };

        {
            let (fragment_table, chunk_table, channels) = (
                world.fragment_table.clone(),
                world.chunk_table.clone(),
                world.channels.clone(),
            );
            world.rt.spawn(async move {
                tokio_transform_chunk_loads(rx_chunk_load, fragment_table, chunk_table, channels)
                    .await;
            });
        }

        {
            let rt = world.rt.clone();

            let fragment_table = world.fragment_table.clone();

            let rx_generate_fragment = world.channels.tx_generate_fragment.subscribe();
            let tx_generate_fragment = world.channels.tx_generate_fragment.clone();

            let tx_fragment_instantiate_event = world.channels.tx_instantiate_fragment.clone();
            let wfc_config = world.wfc_config.clone();

            world.rt.spawn(async move {
                generate_fragments(
                    rt,
                    fragment_table,
                    wfc_config,
                    rx_generate_fragment,
                    tx_generate_fragment,
                    tx_fragment_instantiate_event,
                )
                .await;
            });
        }

        world
    }
}

pub fn async_world_system(
    async_world: ResMut<AsyncWorld>,
    mut ev_chunk_load: EventReader<ChunkLoadEvent>,
    fragment_settings: Res<FragmentSettings>,
    mut wfc_config_needs_updating: Local<(bool,)>,
) {
    // Insert new chunk load events
    let events = ev_chunk_load.iter().copied().collect_vec();
    {
        let tx_chunk_load = async_world.tx_chunk_load.clone();
        async_world.rt.spawn(async move {
            for event in events.into_iter() {
                tx_chunk_load.send(event).await.unwrap();
            }
        });
    }

    // Update wfc config
    if fragment_settings.is_changed() {
        wfc_config_needs_updating.0 = true;
    }
    if wfc_config_needs_updating.0 {
        if let Ok(mut wfc_config) = async_world.wfc_config.try_write() {
            wfc_config.fragment_settings = fragment_settings.clone();
        }
    }
}

/// Transforms chunk load events into fragments which are registered for generation in the fragment table
pub async fn tokio_transform_chunk_loads(
    mut rx_chunk_load_events: mpsc::Receiver<ChunkLoadEvent>,
    fragment_table: Arc<RwLock<FragmentTable>>,
    chunk_table: Arc<RwLock<ChunkTable>>,
    channels: Arc<AsyncTaskChannels>,
) {
    loop {
        let load_chunk = rx_chunk_load_events.recv().await.unwrap();
        let tx_generate_fragment = channels.tx_generate_fragment.clone();

        match load_chunk {
            ChunkLoadEvent::Reset => {
                {
                    let mut chunk_table = chunk_table.write().await;
                    *chunk_table = ChunkTable::default();
                }
                {
                    let mut fragment_table = fragment_table.write().await;
                    *fragment_table = FragmentTable::default();
                }
            }
            ChunkLoadEvent::Unload(chunk_pos) => {
                let mut chunk_table = chunk_table.write().await;
                let mut fragment_table = fragment_table.write().await;

                if let Some(chunk) = chunk_table.loaded.remove(&chunk_pos) {
                    match chunk {
                        ChunkEntry::Waiting {
                            associated_fragments,
                        } => {
                            for fragment in associated_fragments {
                                match fragment {
                                    FragmentLocation::Node(node_pos) => {
                                        if let Some(node) =
                                            fragment_table.loaded_nodes.get_mut(&node_pos)
                                        {
                                            node.chunks.remove(&chunk_pos);
                                            if node.chunks.is_empty() {
                                                fragment_table.loaded_nodes.remove(&node_pos);
                                                fragment_table
                                                    .edges_waiting_by_node
                                                    .remove(&node_pos);
                                                channels
                                                    .tx_destroy_fragment
                                                    .send(FragmentDestroyEvent {
                                                        fragment_location: FragmentLocation::Node(
                                                            node_pos,
                                                        ),
                                                    })
                                                    .unwrap();
                                            }
                                        }
                                    }
                                    FragmentLocation::Edge(edge_pos) => {
                                        if let Some(edge) =
                                            fragment_table.loaded_edges.get_mut(&edge_pos)
                                        {
                                            edge.chunks.remove(&chunk_pos);
                                            if edge.chunks.is_empty() {
                                                if let Some(EdgeFragmentEntry {
                                                    status: EdgeFragmentStatus::Waiting(nodes),
                                                    ..
                                                }) =
                                                    fragment_table.loaded_edges.remove(&edge_pos)
                                                {
                                                    for node in nodes.into_iter() {
                                                        if let Some(edges) = fragment_table
                                                            .edges_waiting_by_node
                                                            .get_mut(&node)
                                                        {
                                                            edges.remove(&edge_pos);
                                                        }
                                                    }
                                                }

                                                fragment_table
                                                    .faces_waiting_by_edge
                                                    .remove(&edge_pos);
                                                channels
                                                    .tx_destroy_fragment
                                                    .send(FragmentDestroyEvent {
                                                        fragment_location: FragmentLocation::Edge(
                                                            edge_pos,
                                                        ),
                                                    })
                                                    .unwrap();
                                            }
                                        }
                                    }
                                    FragmentLocation::Face(face_pos) => {
                                        if let Some(face) =
                                            fragment_table.loaded_faces.get_mut(&face_pos)
                                        {
                                            face.chunks.remove(&chunk_pos);
                                            if face.chunks.is_empty() {
                                                if let Some(FaceFragmentEntry {
                                                    status: FaceFragmentStatus::Waiting(edges),
                                                    ..
                                                }) =
                                                    fragment_table.loaded_faces.remove(&face_pos)
                                                {
                                                    for edge in edges.into_iter() {
                                                        if let Some(faces) = fragment_table
                                                            .faces_waiting_by_edge
                                                            .get_mut(&edge)
                                                        {
                                                            faces.remove(&face_pos);
                                                        }
                                                    }
                                                }

                                                channels
                                                    .tx_destroy_fragment
                                                    .send(FragmentDestroyEvent {
                                                        fragment_location: FragmentLocation::Face(
                                                            face_pos,
                                                        ),
                                                    })
                                                    .unwrap();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ChunkLoadEvent::Load(chunk_pos) => {
                // Checkout new chunk
                let mut chunk_table = chunk_table.write().await;
                let mut fragment_table = fragment_table.write().await;

                if let Some(..) = chunk_table.loaded.get(&chunk_pos) {
                    continue;
                }

                // Get write lock on fragment table

                // Positions of chunks component fragments
                let faces_pos = [4 * chunk_pos + 2 * IVec3::X + 2 * IVec3::Z];
                let edges_pos = [
                    2 * chunk_pos + IVec3::Z,
                    2 * chunk_pos + IVec3::X,
                    2 * chunk_pos + 2 * IVec3::X + IVec3::Z,
                    2 * chunk_pos + IVec3::X + 2 * IVec3::Z,
                ];
                let nodes_pos = [
                    chunk_pos,
                    chunk_pos + IVec3::X,
                    chunk_pos + IVec3::X + IVec3::Z,
                    chunk_pos + IVec3::Z,
                ];

                chunk_table.loaded.insert(
                    chunk_pos,
                    ChunkEntry::Waiting {
                        associated_fragments: faces_pos
                            .iter()
                            .map(|pos| FragmentLocation::Face(*pos))
                            .chain(edges_pos.iter().map(|pos| FragmentLocation::Edge(*pos)))
                            .chain(nodes_pos.iter().map(|pos| FragmentLocation::Node(*pos)))
                            .collect_vec(),
                    },
                );

                drop(chunk_table);

                let edge_loaded = edges_pos.map(|pos| {
                    matches!(
                        fragment_table.loaded_edges.get(&pos),
                        Some(EdgeFragmentEntry {
                            status: EdgeFragmentStatus::Generated(..),
                            ..
                        })
                    )
                });
                let node_loaded = nodes_pos.map(|pos| {
                    matches!(
                        fragment_table.loaded_nodes.get(&pos),
                        Some(NodeFragmentEntry {
                            status: NodeFragmentStatus::Generated(..),
                            ..
                        })
                    )
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

                    if let Some(node_entry) = fragment_table.loaded_nodes.get_mut(&node) {
                        node_entry.chunks.insert(chunk_pos);
                    } else {
                        // Announce new node to generate
                        fragment_table.loaded_nodes.insert(
                            node,
                            NodeFragmentEntry {
                                status: NodeFragmentStatus::Generating,
                                chunks: HashSet::from_iter(Some(chunk_pos)),
                            },
                        );
                        tx_generate_fragment
                            .send(FragmentGenerateEvent::Node(node))
                            .unwrap();
                    }

                    if let Some(edge_entry) = fragment_table.loaded_edges.get_mut(&edge) {
                        edge_entry.chunks.insert(chunk_pos);
                    } else {
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
                                .edges_waiting_by_node
                                .entry(node)
                                .or_insert(HashSet::new());
                            waiting_on_node.insert(edge);
                        }

                        // Check if dependencies have already been satisfied.
                        if !waiting_for.is_empty() {
                            fragment_table.loaded_edges.insert(
                                edge,
                                EdgeFragmentEntry {
                                    status: EdgeFragmentStatus::Waiting(HashSet::from_iter(
                                        waiting_for,
                                    )),
                                    chunks: HashSet::from_iter(Some(chunk_pos)),
                                },
                            );
                        } else {
                            tx_generate_fragment
                                .send(FragmentGenerateEvent::Edge(edge))
                                .unwrap();
                        }
                    }
                }

                if let Some(face_entry) = fragment_table.loaded_faces.get_mut(&face) {
                    face_entry.chunks.insert(chunk_pos);
                } else {
                    // Keep track of fragments the face is waiting for
                    let waiting_for = edges_pos
                        .into_iter()
                        .zip(edge_loaded.into_iter())
                        .filter_map(|(pos, loaded)| match loaded {
                            true => None,
                            false => Some(pos),
                        })
                        .collect_vec();

                    for edge in waiting_for.clone() {
                        let faces_awaiting_edge = fragment_table
                            .faces_waiting_by_edge
                            .entry(edge)
                            .or_insert(HashSet::new());
                        faces_awaiting_edge.insert(face);
                    }

                    // Check if dependencies have already been satisfied.
                    if !waiting_for.is_empty() {
                        fragment_table.loaded_faces.insert(
                            face,
                            FaceFragmentEntry {
                                status: FaceFragmentStatus::Waiting(HashSet::from_iter(
                                    waiting_for,
                                )),
                                chunks: HashSet::from_iter(Some(chunk_pos)),
                            },
                        );
                    } else {
                        tx_generate_fragment
                            .send(FragmentGenerateEvent::Face(face))
                            .unwrap();
                    }
                }
            }
        }
    }
}
