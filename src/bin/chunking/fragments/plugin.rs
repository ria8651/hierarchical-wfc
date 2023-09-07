use std::sync::Arc;

use bevy::{
    math::{uvec3, vec3},
    prelude::*,
    utils::HashMap,
};
use hierarchical_wfc::{castle::LayoutTileset, graphs::regular_grid_3d, wfc::WfcGraph};

use super::systems::{async_world_system, AsyncWorld};

#[derive(Event, Clone, Copy)]
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
    pub debug_fragment_nodes: bool,
    pub debug_fragment_edges: bool,
    pub debug_fragment_faces: bool,
    pub debug_chunks: bool,
}
impl Default for GenerationDebugSettings {
    fn default() -> Self {
        GenerationDebugSettings {
            debug_fragment_nodes: false,
            debug_fragment_edges: false,
            debug_fragment_faces: true,
            debug_chunks: false,
        }
    }
}
#[derive(Component)]
pub struct ChunkMarker;

#[derive(Component)]
pub struct FragmentMarker;

pub struct GenerationPlugin;
impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, async_world_system)
            .insert_resource(LayoutSettings {
                settings: regular_grid_3d::GraphSettings {
                    size: uvec3(8, 1, 8),
                    spacing: vec3(2.0, 3.0, 2.0),
                },
                tileset: LayoutTileset,
            })
            // .init_resource::<ChunkTable>()
            // .init_resource::<FragmentTable>()
            .init_resource::<GenerationDebugSettings>()
            .init_resource::<AsyncWorld>()
            // .add_event::<FragmentGenerateEvent>()
            .add_event::<ChunkLoadEvent>()
            .add_event::<ChunkGenerateEvent>();
    }
}
