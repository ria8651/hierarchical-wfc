use bevy::{asset::ChangeWatcher, window::PresentMode};
use bevy_render::texture::ImageSampler;
use std::time::Duration;

use bevy::{
    math::vec3,
    prelude::{AssetPlugin, PluginGroup, *},
    render::render_resource::{AddressMode, FilterMode, SamplerDescriptor},
};

use bevy::log::LogPlugin;
use bevy_inspector_egui::{bevy_egui, DefaultInspectorConfigPlugin};
use bevy_mod_billboard::prelude::*;
use bevy_mod_debugdump;
use bevy_rapier3d::prelude::{
    Collider, ComputedColliderShape, NoUserData, RapierPhysicsPlugin, RigidBody,
};
use hierarchical_wfc::{
    camera_plugin::cam_switcher::SwitchingCameraPlugin,
    materials::{debug_arc_material::DebugLineMaterial, tile_pbr_material::TilePbrMaterial},
    tools::MeshBuilder,
    ui_plugin::{EcsTab, EcsUiPlugin, EcsUiState, EcsUiTab},
    village::{
        facade_graph::{FacadePassData, FacadePassSettings, FacadeTileset},
        layout_graph::LayoutGraphSettings,
        layout_pass::LayoutTileset,
    },
    wfc::{
        bevy_passes::{
            wfc_collapse_system, wfc_ready_system, WfcEntityMarker, WfcFCollapsedData,
            WfcInitialData, WfcInvalidatedMarker, WfcParentPasses, WfcPassReadyMarker,
        },
        TileSet, WfcGraph,
    },
};
use rand::{rngs::StdRng, SeedableRng};

#[derive(Component)]
pub struct GenerateDebugMarker;
