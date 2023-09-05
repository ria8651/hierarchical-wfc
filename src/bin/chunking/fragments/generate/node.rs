use super::{
    super::{
        plugin::{CollapsedData, FragmentGenerateEvent, GenerationDebugSettings, LayoutSettings},
        table::FragmentTable,
    },
    FragmentQueue, FRAGMENT_EDGE_PADDING, FRAGMENT_FACE_SIZE, FRAGMENT_NODE_PADDING,
};
use crate::fragments::{
    plugin::{FragmentMarker, GenerateDebugMarker},
    table::{EdgeFragmentEntry, NodeFragmentEntry},
};
use bevy::{
    math::{uvec3, vec3},
    prelude::*,
};
use hierarchical_wfc::{
    graphs::regular_grid_3d,
    wfc::{Superposition, TileSet, WaveFunctionCollapse},
};
use rand::{rngs::StdRng, SeedableRng};

pub(crate) fn generate_node(
    layout_settings: &Res<'_, LayoutSettings>,
    node_pos: IVec3,
    constraints: &Box<[Box<[Superposition]>]>,
    weights: &Vec<u32>,
    commands: &mut Commands<'_, '_>,
    debug_settings: &ResMut<'_, GenerationDebugSettings>,
    fragment_table: &mut ResMut<'_, FragmentTable>,
    generate_fragment_queue: &mut Local<'_, FragmentQueue>,
) {
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
        constraints,
        weights,
        &mut StdRng::from_seed(seed),
    );
    if let Ok(result) = graph.validate() {
        let mut fragment_commands = commands.spawn((
            FragmentMarker,
            Transform::from_translation(
                node_pos.as_vec3() * FRAGMENT_FACE_SIZE as f32 * layout_settings.settings.spacing
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
