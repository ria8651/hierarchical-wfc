use bevy::{ecs::system::SystemState, prelude::*};
use bevy_inspector_egui::{bevy_inspector::ui_for_value, reflect_inspector};
use bevy_render::camera::Projection;
use hierarchical_wfc::{
    camera_plugin::{
        cam_switcher::{CameraController, SwitchingCameraController},
        fps::FpsCameraSettings,
    },
    ui_plugin::{EcsTab, EcsUiTab},
};

use crate::fragments::generate::{ChunkLoadEvent, GenerationDebugSettings};

pub struct EcsUiDebugSettings {
    system_state: SystemState<(ResMut<'static, GenerationDebugSettings>,)>,
}

impl EcsUiDebugSettings {
    pub fn tab_from_world(world: &mut World) -> EcsUiTab {
        EcsUiTab::Ecs(Box::new(Self {
            system_state: SystemState::new(world),
        }))
    }
}

impl std::fmt::Debug for EcsUiDebugSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Debug Settings").finish()
    }
}

impl EcsTab for EcsUiDebugSettings {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let mut settings = self.system_state.get_mut(world);

        ui.label("Spawn Debug Meshes");
        ui.checkbox(&mut settings.0.debug_chunks, "Chunks");
        ui.checkbox(&mut settings.0.debug_fragments, "Fragments");

        self.system_state.apply(world);
    }
}
