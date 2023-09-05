use bevy::{
    math::{uvec3, vec3},
    prelude::*,
    utils::HashMap,
};
use hierarchical_wfc::{castle::LayoutTileset, graphs::regular_grid_3d, wfc::WfcGraph};

use super::{generate::generate_fragments, systems::transform_chunk_loads, table::FragmentTable};

#[derive(Event)]
pub enum ChunkLoadEvent {
    Load(IVec3),
}

pub enum ChunkEntry {
    Waiting,
}

#[derive(Resource, Default)]
pub struct ChunkTable {
    pub loaded: HashMap<IVec3, ChunkEntry>,
}

#[derive(Event, Clone, Copy)]
pub enum FragmentGenerateEvent {
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

pub struct GenerationPlugin;
impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (generate_fragments, transform_chunk_loads))
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
            .add_event::<ChunkLoadEvent>()
            .add_event::<ChunkGenerateEvent>();
    }
}
