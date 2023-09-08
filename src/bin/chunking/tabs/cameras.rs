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

pub struct EcsUiCameras {
    system_state: SystemState<
        Query<
            'static,
            'static,
            (
                &'static mut SwitchingCameraController,
                &'static mut Projection,
                Option<&'static mut FpsCameraSettings>,
            ),
        >,
    >,
}

impl EcsUiCameras {
    pub fn tab_from_world(world: &mut World) -> EcsUiTab {
        EcsUiTab::Ecs(Box::new(Self {
            system_state: SystemState::new(world),
        }))
    }
}

impl std::fmt::Debug for EcsUiCameras {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cameras").finish()
    }
}

impl EcsTab for EcsUiCameras {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        type_registry: &bevy_reflect::TypeRegistry,
        _active: bool,
    ) {
        let mut q_cameras = self.system_state.get_mut(world);

        for (mut camera_controller, projection, fps_settings) in q_cameras.iter_mut() {
            egui::ComboBox::new("camera_controller_combo_box", "")
                .selected_text(match camera_controller.selected {
                    CameraController::PanOrbit => "Pan Orbit",
                    CameraController::Fps => "First Person",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut camera_controller.selected,
                        CameraController::PanOrbit,
                        "Pan Orbit",
                    );
                    ui.selectable_value(
                        &mut camera_controller.selected,
                        CameraController::Fps,
                        "First Person",
                    );
                });
            match camera_controller.selected {
                CameraController::Fps => {
                    if let Some(mut settings) = fps_settings {
                        reflect_inspector::ui_for_value(settings.as_mut(), ui, type_registry);
                    }
                }
                CameraController::PanOrbit => {}
            }

            reflect_inspector::ui_for_value(projection.into_inner(), ui, type_registry);
        }

        self.system_state.apply(world);
    }
}
