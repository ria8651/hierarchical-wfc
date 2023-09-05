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

use crate::fragments::plugin::{ChunkLoadEvent, GenerationDebugSettings, LayoutSettings};

pub struct EcsUiDebugSettings {
    system_state: SystemState<(
        ResMut<'static, GenerationDebugSettings>,
        ResMut<'static, LayoutSettings>,
    )>,
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
        let (mut settings, mut layout_settings) = self.system_state.get_mut(world);

        ui.label("Debug Chunks");
        ui.checkbox(&mut settings.debug_chunks, "Chunks");

        ui.spacing();
        ui.label("Debug Fragments");
        ui.checkbox(&mut settings.debug_fragment_nodes, "Nodes");
        ui.checkbox(&mut settings.debug_fragment_edges, "Edges");
        ui.checkbox(&mut settings.debug_fragment_faces, "Faces");

        ui.label("Layout Settings");
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.settings.size.x));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.settings.size.y));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.settings.size.z));
        });
        self.system_state.apply(world);
    }
}
