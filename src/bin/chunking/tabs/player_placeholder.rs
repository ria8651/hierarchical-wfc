use bevy::{ecs::system::SystemState, prelude::*};

use bevy_egui::EguiContexts;
use hierarchical_wfc::ui_plugin::{EcsTab, EcsUiTab};

use crate::fragments::plugin::ChunkLoadEvent;

pub struct EcsUiPlayerPlaceholder {
    system_state: SystemState<(
        EguiContexts<'static, 'static>,
        Query<'static, 'static, (&'static Camera, &'static GlobalTransform)>,
        EventWriter<'static, ChunkLoadEvent>,
        Local<'static, Vec3>,
    )>,
}

impl EcsUiPlayerPlaceholder {
    pub fn tab_from_world(world: &mut World) -> EcsUiTab {
        EcsUiTab::Ecs(Box::new(Self {
            system_state: SystemState::new(world),
        }))
    }
}

impl std::fmt::Debug for EcsUiPlayerPlaceholder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player Placeholder").finish()
    }
}

impl EcsTab for EcsUiPlayerPlaceholder {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        _type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let (mut contexts, q_camera, mut ev_chunk_load, mut player_location) =
            self.system_state.get_mut(world);

        let (camera, camera_transform) = q_camera.get_single().unwrap();
        let viewport = if let Some(viewport) = &camera.viewport {
            Some(egui::Rect {
                min: egui::Pos2::from(viewport.physical_position.as_vec2().to_array()),
                max: egui::Pos2::from(
                    (viewport.physical_position + viewport.physical_size)
                        .as_vec2()
                        .to_array(),
                ),
            })
        } else {
            None
        };
        let viewport = if let Some(viewport) = viewport {
            viewport
        } else {
            return;
        };
        // if ui.button("Send Event").clicked() {
        //     for (z, y, x) in iproduct!(z_0..=z_1, y_0..=y_1, x_0..=x_1) {
        //         ev_chunk_load.send(ChunkLoadEvent::Load(ivec3(x, y, z)))
        //     }
        // }

        // egui::Area::new("Viewport")
        //     .fixed_pos((0.0, 0.0))
        //     .show(&contexts.ctx_mut(), |ui| {
        //         ui.with_layer_id(egui::LayerId::background(), |ui| {
        //             let painter = ui.painter();

        //             let padding =
        //                 egui::vec2(16.0, 16.0 + egui_dock::style::TabBarStyle::default().height);
        //             let radius: f32 = 24.0f32;
        //             // let center =
        //             //     (0.5 * viewport.min.to_vec2() + 0.5 * viewport.max.to_vec2()).to_pos2();

        //             let center = egui::pos2(
        //                 viewport.max.x - radius - padding.x,
        //                 viewport.min.y + radius + padding.y,
        //             );

        //             if let Some(loc_viewport) =
        //                 camera.world_to_viewport(camera_transform, *player_location)
        //             {
        //                 let loc_screen: egui::Pos2 =
        //                     viewport.min + egui::vec2(loc_viewport.x, loc_viewport.y);
        //                 painter.circle(
        //                     loc_screen,
        //                     32.0,
        //                     egui::Color32::RED,
        //                     (1.0, egui::Color32::WHITE),
        //                 );
        //             }

        //             painter.circle(
        //                 center,
        //                 32.0,
        //                 egui::Color32::RED,
        //                 (1.0, egui::Color32::WHITE),
        //             );
        //         })
        //     });

        self.system_state.apply(world);
    }
}
