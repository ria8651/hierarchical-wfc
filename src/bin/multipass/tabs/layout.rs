use bevy::{ecs::system::SystemState, math::vec3, prelude::*};
use hierarchical_wfc::{
    ui_plugin::{EcsTab, EcsUiTab},
    village::{facade_graph::FacadePassSettings, LayoutGraphSettings},
    wfc::bevy_passes::{
        WfcEntityMarker, WfcInvalidatedMarker, WfcParentPasses, WfcPassReadyMarker,
        WfcPendingParentMarker,
    },
};

use crate::{
    generation::GenerateDebugMarker,
    passes::{LayoutDebugSettings, LayoutPass},
    GroundPlane,
};

type GroundQuery = Query<'static, 'static, &'static mut Handle<Mesh>, With<GroundPlane>>;
type WfcEntitiesQuery = Query<'static, 'static, Entity, With<WfcEntityMarker>>;
type LayoutSystemParams = (
    Commands<'static, 'static>,
    WfcEntitiesQuery,
    Local<'static, LayoutGraphSettings>,
    GroundQuery,
    ResMut<'static, Assets<Mesh>>,
    Gizmos<'static>,
    ResMut<'static, GizmoConfig>,
);
pub struct EcsUiLayout {
    system_state: SystemState<LayoutSystemParams>,
}

impl EcsUiLayout {
    pub fn tab_from_world(world: &mut World) -> EcsUiTab {
        EcsUiTab::Ecs(Box::new(Self {
            system_state: SystemState::new(world),
        }))
    }
}

impl std::fmt::Debug for EcsUiLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layout").finish()
    }
}

impl EcsTab for EcsUiLayout {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        _type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let (
            mut commands,
            q_wfc_entities,
            mut layout_settings,
            mut q_ground,
            mut meshes,
            mut gizmos,
            mut config,
        ) = self.system_state.get_mut(world);

        {
            config.line_width = 2.0;
            let origin = vec3(-2.0, 0.0, -2.0);
            let max = vec3(2.0, 3.0, 2.0)
                * vec3(
                    layout_settings.x_size as f32 + 2.0,
                    layout_settings.y_size as f32 + 1.0,
                    layout_settings.z_size as f32 + 2.0,
                );
            let e_x = max * Vec3::X;
            let e_y = max * Vec3::Y;
            let e_z = max * Vec3::Z;

            let bound_color = Color::rgb(0.95, 0.95, 0.95);

            gizmos.line(origin, origin + e_x, Color::rgb(0.9, 0.2, 0.2));
            gizmos.line(origin, origin + e_y, Color::rgb(0.2, 0.9, 0.2));
            gizmos.line(origin, origin + e_z, Color::rgb(0.2, 0.2, 0.9));

            gizmos.line(origin + e_x, origin + e_x + e_y, bound_color);
            gizmos.line(origin + e_x, origin + e_x + e_z, bound_color);

            gizmos.line(origin + e_y, origin + e_y + e_z, bound_color);
            gizmos.line(origin + e_y, origin + e_y + e_x, bound_color);

            gizmos.line(origin + e_z, origin + e_z + e_x, bound_color);
            gizmos.line(origin + e_z, origin + e_z + e_y, bound_color);

            gizmos.line(origin + e_x + e_y + e_z, origin + e_x + e_y, bound_color);
            gizmos.line(origin + e_x + e_y + e_z, origin + e_y + e_z, bound_color);
            gizmos.line(origin + e_x + e_y + e_z, origin + e_z + e_x, bound_color);
        }

        ui.label("Layout size");
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.x_size));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.y_size));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.z_size));
        });

        ui.add_space(12.0);
        if ui.button("Generate").clicked() {
            for entity in q_wfc_entities.iter() {
                commands.entity(entity).insert(WfcInvalidatedMarker);
            }

            let layout_entity = commands
                .spawn((
                    WfcEntityMarker,
                    WfcPassReadyMarker,
                    GenerateDebugMarker,
                    LayoutPass {
                        settings: *layout_settings,
                    },
                    LayoutDebugSettings {
                        blocks: true,
                        arcs: true,
                    },
                ))
                .id();

            commands.spawn((
                WfcEntityMarker,
                FacadePassSettings,
                WfcPendingParentMarker,
                WfcParentPasses(vec![layout_entity]),
            ));

            if let Ok(ground) = q_ground.get_single_mut() {
                let padding = vec3(10.0, 0.0, 10.0);
                let start = vec3(-1.5, 0.0, -1.5) - padding;
                let end = vec3(
                    2.0 * layout_settings.x_size as f32,
                    0.0,
                    2.0 * layout_settings.z_size as f32,
                ) + vec3(0.5, 0.0, 0.5)
                    + padding;

                let mut ground_mesh =
                    Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleStrip);
                ground_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 4]);
                ground_mesh.insert_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    vec![
                        [0.5 * start.x, 0.5 * start.z],
                        [0.5 * start.x, 0.5 * end.z],
                        [0.5 * end.x, 0.5 * start.z],
                        [0.5 * end.x, 0.5 * end.z],
                    ],
                );
                ground_mesh.insert_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    vec![
                        vec3(start.x, 0.0, start.z),
                        vec3(start.x, 0.0, end.z),
                        vec3(end.x, 0.0, start.z),
                        vec3(end.x, 0.0, end.z),
                    ],
                );
                let _ = meshes.set(ground.id(), ground_mesh);
            }
        }
        if !q_wfc_entities.is_empty() && ui.button("Reset").clicked() {
            for entity in q_wfc_entities.iter() {
                commands.entity(entity).insert(WfcInvalidatedMarker);
            }
        }
        self.system_state.apply(world);
    }
}
