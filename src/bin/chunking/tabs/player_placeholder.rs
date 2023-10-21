use bevy::{
    ecs::system::SystemState,
    math::{ivec3, vec3},
    prelude::*,
};

use bevy_egui::EguiContexts;
use hierarchical_wfc::ui_plugin::{EcsTab, EcsUiTab};

use crate::fragments::{generate::FragmentSettings, plugin::ChunkLoadEvent};
use egui_gizmo::Gizmo;

pub struct PlayerData {
    position: Vec3,
    chunk: IVec3,
    view_distance: u32,
    generate: bool,
    destroy: bool,
}
impl Default for PlayerData {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            chunk: IVec3::ZERO,
            view_distance: 0,
            generate: false,
            destroy: false,
        }
    }
}

pub struct EcsUiPlayerPlaceholder {
    system_state: SystemState<(
        EguiContexts<'static, 'static>,
        Query<'static, 'static, (&'static Camera, &'static GlobalTransform)>,
        EventWriter<'static, ChunkLoadEvent>,
        Local<'static, PlayerData>,
        Res<'static, FragmentSettings>,
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
        active: bool,
    ) {
        let (mut contexts, q_camera, mut ev_chunk_load, mut player_data, fragment_settings) =
            self.system_state.get_mut(world);

        let (camera, camera_transform) = q_camera.get_single().unwrap();
        let viewport = camera.logical_viewport_rect().map(|viewport| egui::Rect {
            min: egui::Pos2::from(viewport.min.to_array()),
            max: egui::Pos2::from(viewport.max.to_array()),
        });

        if viewport.is_none() {
            return;
        }
        let viewport = viewport.unwrap();

        player_data.chunk = chunk_from_position(player_data.position, &fragment_settings);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut player_data.chunk.x).speed(0.0));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut player_data.chunk.y).speed(0.0));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut player_data.chunk.z).speed(0.0));
        });
        ui.add(egui::DragValue::new(&mut player_data.view_distance));
        ui.checkbox(&mut player_data.generate, "Generate");
        ui.checkbox(&mut player_data.destroy, "Destroy");

        let transform_gizmo = Gizmo::new("id_source")
            .projection_matrix(camera.projection_matrix().to_cols_array_2d())
            .view_matrix(
                camera_transform
                    .compute_matrix()
                    .inverse()
                    .to_cols_array_2d(),
            )
            .model_matrix(
                Transform::from_translation(player_data.position)
                    .compute_matrix()
                    .to_cols_array_2d(),
            )
            .viewport(viewport)
            .mode(egui_gizmo::GizmoMode::Translate);

        egui::Area::new("Viewport")
            .fixed_pos((0.0, 0.0))
            .show(contexts.ctx_mut(), |ui| {
                ui.with_layer_id(egui::LayerId::background(), |ui| {
                    if active {
                        if let Some(response) = transform_gizmo.interact(ui) {
                            let new_position = response.translation * vec3(1.0, 0.0, 1.0);

                            let new_chunk = chunk_from_position(new_position, &fragment_settings);
                            let view_dist = player_data.view_distance as i32;

                            if player_data.chunk != new_chunk {
                                if player_data.generate {
                                    for dx in -view_dist..=view_dist {
                                        for dz in -view_dist..=view_dist {
                                            let loading_chunk = new_chunk + ivec3(dx, 0, dz);
                                            let distance = (loading_chunk - player_data.chunk)
                                                .abs()
                                                .max_element()
                                                as u32;
                                            if distance >= player_data.view_distance {
                                                ev_chunk_load
                                                    .send(ChunkLoadEvent::Load(loading_chunk));
                                            }
                                            dbg!(loading_chunk);
                                        }
                                    }
                                }
                                if player_data.destroy {
                                    for dx in -view_dist..=view_dist {
                                        for dz in -view_dist..=view_dist {
                                            let unloading_chunk =
                                                player_data.chunk + ivec3(dx, 0, dz);
                                            let distance =
                                                (unloading_chunk - new_chunk).abs().max_element()
                                                    as u32;
                                            if distance > player_data.view_distance {
                                                dbg!(new_chunk);
                                                ev_chunk_load
                                                    .send(ChunkLoadEvent::Unload(unloading_chunk));
                                                dbg!(unloading_chunk);
                                            }
                                        }
                                    }
                                }
                            }
                            player_data.position = new_position;
                        };
                    }
                })
            });

        self.system_state.apply(world);
    }
}

fn chunk_from_position(position: Vec3, fragment_settings: &FragmentSettings) -> IVec3 {
    (position / fragment_settings.spacing / fragment_settings.face_size as f32)
        .floor()
        .as_ivec3()
        * ivec3(1, 0, 1)
}
