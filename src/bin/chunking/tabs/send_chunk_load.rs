use bevy::{ecs::system::SystemState, prelude::*};
use bevy_inspector_egui::reflect_inspector;
use bevy_render::camera::Projection;
use hierarchical_wfc::{
    camera_plugin::{
        cam_switcher::{CameraController, SwitchingCameraController},
        fps::FpsCameraSettings,
    },
    ui_plugin::{EcsTab, EcsUiTab},
};

use crate::fragments::generate::ChunkLoadEvent;

pub struct EcsUiSendChunkLoads {
    system_state: SystemState<(EventWriter<'static, ChunkLoadEvent>, Local<'static, IVec3>)>,
}

impl EcsUiSendChunkLoads {
    pub fn tab_from_world(world: &mut World) -> EcsUiTab {
        EcsUiTab::Ecs(Box::new(Self {
            system_state: SystemState::new(world),
        }))
    }
}

impl std::fmt::Debug for EcsUiSendChunkLoads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Load Chunks").finish()
    }
}

impl EcsTab for EcsUiSendChunkLoads {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let (mut ev_chunk_load, mut chunk_location) = self.system_state.get_mut(world);

        ui.label("Chunk Load Event");
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut chunk_location.x));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut chunk_location.y));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut chunk_location.z));
        });

        if ui.button("Send Event").clicked() {
            ev_chunk_load.send(ChunkLoadEvent::Load(chunk_location.clone()))
        }

        self.system_state.apply(world);
    }
}
