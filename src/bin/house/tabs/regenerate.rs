use bevy::{ecs::system::SystemState, math::vec3, prelude::*};
use hierarchical_wfc::ui_plugin::{EcsTab, EcsUiTab};

use crate::{passes::HousePassMarker, regenerate::RegenerateSettings};

type LayoutPassQuery = Query<'static, 'static, Entity, With<HousePassMarker>>;

type LayoutSystemParams = (
    Commands<'static, 'static>,
    LayoutPassQuery,
    Local<'static, RegenerateSettings>,
    Gizmos<'static>,
    ResMut<'static, GizmoConfig>,
);
pub struct EcsUiRegenerate {
    system_state: SystemState<LayoutSystemParams>,
}

impl EcsUiRegenerate {
    pub fn tab_from_world(world: &mut World) -> EcsUiTab {
        EcsUiTab::Ecs(Box::new(Self {
            system_state: SystemState::new(world),
        }))
    }
}

impl std::fmt::Debug for EcsUiRegenerate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Regenerate").finish()
    }
}

impl EcsTab for EcsUiRegenerate {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        _type_registry: &bevy_reflect::TypeRegistry,
        _active: bool,
    ) {
        let (mut commands, q_layout_pass_settings, mut regenerate_settings, mut gizmos, mut config) =
            self.system_state.get_mut(world);

        {
            config.line_width = 2.0;
            config.depth_bias = -1.0;

            let scale = vec3(2.0, 3.0, 2.0) * (regenerate_settings.max - regenerate_settings.min);
            let translation =
                0.5 * vec3(2.0, 3.0, 2.0) * (regenerate_settings.max + regenerate_settings.min);
            let bound_color = Color::rgb(2.0, 1.0, 0.5);
            gizmos.cuboid(
                Transform::from_scale(scale).with_translation(translation),
                bound_color,
            );
            config.depth_bias = 0.0;
        }

        ui.label("Regenerate Region");
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut regenerate_settings.min.x));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut regenerate_settings.min.y));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut regenerate_settings.min.z));
            ui.label("min")
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut regenerate_settings.max.x));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut regenerate_settings.max.y));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut regenerate_settings.max.z));
            ui.label("max")
        });

        ui.add_space(12.0);
        if let Ok(existing_pass) = q_layout_pass_settings.get_single() {
            if ui.button("Regenerate").clicked() {
                commands
                    .entity(existing_pass)
                    .insert(regenerate_settings.clone());
            }
        }
        self.system_state.apply(world);
    }
}
