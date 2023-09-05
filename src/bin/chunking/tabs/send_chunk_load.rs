use bevy::{ecs::system::SystemState, math::ivec3, prelude::*};

use hierarchical_wfc::ui_plugin::{EcsTab, EcsUiTab};
use itertools::iproduct;

use crate::fragments::plugin::ChunkLoadEvent;

pub struct EcsUiSendChunkLoads {
    system_state: SystemState<(
        EventWriter<'static, ChunkLoadEvent>,
        Local<'static, IVec3>,
        Local<'static, (IVec3, IVec3)>,
    )>,
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
        _type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let (mut ev_chunk_load, mut chunk_location, mut chunk_area) =
            self.system_state.get_mut(world);

        ui.label("Single Load Event");
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
            ev_chunk_load.send(ChunkLoadEvent::Load(*chunk_location))
        }

        ui.label("Multiple Load Events");
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut chunk_area.0.x));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut chunk_area.0.y));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut chunk_area.0.z));
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut chunk_area.1.x));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut chunk_area.1.y));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut chunk_area.1.z));
        });

        let (
            IVec3 {
                x: x_0,
                y: y_0,
                z: z_0,
            },
            IVec3 {
                x: x_1,
                y: y_1,
                z: z_1,
            },
        ) = (chunk_area.0, chunk_area.1);

        if ui.button("Send Event").clicked() {
            for (z, y, x) in iproduct!(z_0..=z_1, y_0..=y_1, x_0..=x_1) {
                ev_chunk_load.send(ChunkLoadEvent::Load(ivec3(x, y, z)))
            }
        }

        self.system_state.apply(world);
    }
}
