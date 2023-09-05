use bevy::{self, prelude::*};
use hierarchical_wfc::{
    graphs::regular_grid_3d::{self},
    wfc::{Superposition, TileSet},
};

use super::{
    plugin::{CollapsedData, FragmentGenerateEvent, GenerationDebugSettings, LayoutSettings},
    table::FragmentTable,
};

#[derive(Default)]
pub struct FragmentQueue {
    queue: Vec<FragmentGenerateEvent>,
}

const FRAGMENT_NODE_PADDING: u32 = 4;
const FRAGMENT_EDGE_PADDING: u32 = 4;
const FRAGMENT_FACE_SIZE: u32 = 32;
const NODE_RADIUS: i32 = FRAGMENT_EDGE_PADDING as i32 + FRAGMENT_NODE_PADDING as i32;

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
            FragmentGenerateEvent::Node(node_pos) => node::generate_node(
                &layout_settings,
                node_pos,
                &constraints,
                &weights,
                &mut commands,
                &debug_settings,
                &mut fragment_table,
                &mut generate_fragment_queue,
            ),
            FragmentGenerateEvent::Edge(edge_pos) => edge::generate_edge(
                edge_pos,
                &mut fragment_table,
                &layout_settings,
                &q_fragments,
                fill_with,
                &constraints,
                &weights,
                &mut commands,
                &debug_settings,
                &mut generate_fragment_queue,
            ),
            FragmentGenerateEvent::Face(face_pos) => face::generate_face(
                face_pos,
                &mut fragment_table,
                &q_fragments,
                &layout_settings,
                fill_with,
                &constraints,
                &weights,
                &mut commands,
                &debug_settings,
            ),
        }
    }
}

mod edge;
mod face;
mod node;
