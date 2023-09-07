use crate::fragments::{
    plugin::ChunkEntry,
    table::{EdgeFragmentEntry, EdgeKey, FaceFragmentEntry, FaceKey, NodeFragmentEntry, NodeKey},
};

use bevy::{prelude::*, utils::HashSet};
use std::sync::Arc;
use tokio::runtime;

use itertools::Itertools;

use super::{
    generate::{generate_fragments, node::generate_node, FragmentInstantiatedEvent, WfcConfig},
    plugin::{ChunkLoadEvent, ChunkTable, FragmentGenerateEvent},
    table::FragmentTable,
};
use tokio::sync::{broadcast, mpsc, RwLock};

pub struct AsyncTaskChannels {
    pub tx_generate_fragment: broadcast::Sender<FragmentGenerateEvent>,
    pub tx_instantiate_fragment: broadcast::Sender<FragmentInstantiatedEvent>,
}

#[derive(Resource)]
pub struct AsyncWorld {
    channels: Arc<AsyncTaskChannels>,
    pub tx_chunk_load: mpsc::Sender<ChunkLoadEvent>,
    pub rx_fragment_instantiate: broadcast::Receiver<FragmentInstantiatedEvent>,
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
        let world = AsyncWorld {
            channels: Arc::new(AsyncTaskChannels {
                tx_generate_fragment: broadcast::channel(100).0,
                tx_instantiate_fragment: tx_inst_frag,
            }),
            chunk_table: Arc::new(chunk_table),
            fragment_table: Arc::new(fragment_table),
            tx_chunk_load,
            wfc_config: Arc::new(RwLock::new(WfcConfig::default())),
            rx_fragment_instantiate: rx_inst_frag,
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
) {
    // Insert new chunk load events
    for ev in ev_chunk_load.iter() {
        async_world.tx_chunk_load.blocking_send(*ev).unwrap();
        dbg!("Forwarded chunk load event!");
    }

    // Read back new entities
}

/// Transforms chunk load events into fragments which are registered for generation in the fragment table
pub async fn tokio_transform_chunk_loads(
    mut rx_chunk_load_events: mpsc::Receiver<ChunkLoadEvent>,
    fragment_table: Arc<RwLock<FragmentTable>>,
    chunk_table: Arc<RwLock<ChunkTable>>,
    channels: Arc<AsyncTaskChannels>,
) {
    loop {
        dbg!("Waiting for event");
        let load_chunk = rx_chunk_load_events.recv().await.unwrap();
        let tx_generate_fragment = channels.tx_generate_fragment.clone();

        match load_chunk {
            ChunkLoadEvent::Load(chunk_pos) => {
                dbg!("Got event {}", chunk_pos);

                // Checkout new chunk
                {
                    let mut chunk_table = chunk_table.write().await;
                    if let Some(chunk) = chunk_table.loaded.get(&chunk_pos) {
                        match chunk {
                            ChunkEntry::Waiting => continue,
                        }
                    }
                    chunk_table.loaded.insert(chunk_pos, ChunkEntry::Waiting);
                }

                // Get write lock on fragment table
                let mut fragment_table = fragment_table.write().await;

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

                let edge_loaded = edges_pos.map(|pos| {
                    matches!(
                        fragment_table.loaded_edges.get(&pos),
                        Some(EdgeFragmentEntry::Generated(..))
                    )
                });
                let node_loaded = nodes_pos.map(|pos| {
                    matches!(
                        fragment_table.loaded_nodes.get(&pos),
                        Some(NodeFragmentEntry::Generated(..))
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

                    if !fragment_table.loaded_nodes.contains_key(&node) {
                        // Announce new node to generate
                        fragment_table
                            .loaded_nodes
                            .insert(node, NodeFragmentEntry::Generating);
                        tx_generate_fragment
                            .send(FragmentGenerateEvent::Node(node))
                            .unwrap();
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
                            tx_generate_fragment
                                .send(FragmentGenerateEvent::Edge(edge))
                                .unwrap();
                        }
                    }
                }

                if !fragment_table.loaded_faces.contains_key(&face) {
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
                        tx_generate_fragment
                            .send(FragmentGenerateEvent::Face(face))
                            .unwrap();
                    }
                }
            }
        }
    }
}
