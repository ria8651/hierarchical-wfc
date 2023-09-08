use std::sync::Arc;

use bevy::{math::vec3, prelude::*, utils::HashMap};
use hierarchical_wfc::{castle::LayoutTileset, graphs::regular_grid_3d, wfc::WfcGraph};

use super::{
    generate::FragmentSettings,
    systems::{async_world_system, AsyncWorld},
};

#[derive(Event, Clone, Copy)]
pub enum ChunkLoadEvent {
    Load(IVec3),
    Unload(IVec3),
    Reset,
}

pub enum ChunkEntry {
    Waiting,
}

#[derive(Resource, Default)]
pub struct ChunkTable {
    pub loaded: HashMap<IVec3, ChunkEntry>,
}

#[derive(Debug, Event, Clone, Copy)]
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

#[derive(Debug, Component, Clone)]
pub struct CollapsedData {
    pub graph: Arc<WfcGraph<usize>>,
}

#[derive(Event)]
pub struct ChunkGenerateEvent(IVec3);

#[derive(Component)]
pub struct GenerateDebugMarker;

#[derive(Resource)]
pub struct GenerationDebugSettings {
    pub create_fragment_nodes: bool,
    pub create_fragment_edges: bool,
    pub create_fragment_faces: bool,
    pub show_fragment_nodes: bool,
    pub show_fragment_edges: bool,
    pub show_fragment_faces: bool,
    pub debug_chunks: bool,
}
impl Default for GenerationDebugSettings {
    fn default() -> Self {
        GenerationDebugSettings {
            create_fragment_nodes: true,
            create_fragment_edges: true,
            create_fragment_faces: true,
            show_fragment_nodes: false,
            show_fragment_edges: false,
            show_fragment_faces: true,
            debug_chunks: false,
        }
    }
}
#[derive(Component)]
pub struct ChunkMarker;

#[derive(Component)]
pub enum FragmentMarker {
    Node,
    Edge,
    Face,
}

pub struct GenerationPlugin;
impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, async_world_system)
            .init_resource::<FragmentSettings>()
            .init_resource::<GenerationDebugSettings>()
            .init_resource::<AsyncWorld>()
            .add_event::<ChunkLoadEvent>()
            .add_event::<ChunkGenerateEvent>();
    }
}
