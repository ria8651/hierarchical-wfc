use bevy::{
    core_pipeline::{
        contrast_adaptive_sharpening::ContrastAdaptiveSharpeningSettings,
        experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin},
    },
    pbr::{
        ScreenSpaceAmbientOcclusionBundle, ScreenSpaceAmbientOcclusionQualityLevel,
        ScreenSpaceAmbientOcclusionSettings,
    },
    prelude::*,
};

use crate::camera_controllers::fps::FpsCameraBundle;

use super::{
    fps::{FpsCamera, FpsCameraPlugin, FpsCharacterBundle},
    pan_orbit::{PanOrbitCamera, PanOrbitCameraPlugin},
};

pub struct SwitchingCameraPlugin;

impl Plugin for SwitchingCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .insert_resource(Msaa::Off)
            .add_plugins(TemporalAntiAliasPlugin)
            .add_systems(Update, switching_system)
            .add_plugins(FpsCameraPlugin)
            .add_plugins(PanOrbitCameraPlugin);
    }
}

#[derive(Component, Default)]
pub struct SwitchingCameraController {
    pub selected: CameraController,
    last: CameraController,
    pub fps_cam: FpsCamera,
    pub pan_cam: PanOrbitCamera,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CameraController {
    PanOrbit,
    Fps,
}
impl Default for CameraController {
    fn default() -> Self {
        Self::PanOrbit
    }
}

pub fn spawn_camera(mut commands: Commands) {
    let translation = Vec3::new(-2.0, 2.5, 5.0);
    let radius = translation.length();

    commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    hdr: true,
                    ..Default::default()
                },
                tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
                transform: Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
                ..Default::default()
            },
            SwitchingCameraController::default(),
            PanOrbitCamera {
                radius,
                ..Default::default()
            },
            ContrastAdaptiveSharpeningSettings {
                enabled: false,
                ..default()
            },
        ))
        .insert(ScreenSpaceAmbientOcclusionBundle {
            settings: ScreenSpaceAmbientOcclusionSettings {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(TemporalAntiAliasBundle {
            ..Default::default()
        });
}

fn switching_system(
    mut commands: Commands,
    mut q_camera: Query<(Entity, &mut SwitchingCameraController, &Transform)>,
) {
    match q_camera.get_single_mut() {
        Ok((entity, mut switcher, transform)) => {
            if switcher.last != switcher.selected {
                match switcher.last {
                    CameraController::Fps => {
                        commands.entity(entity).remove::<FpsCameraBundle>();
                    }
                    CameraController::PanOrbit => {
                        commands.entity(entity).remove::<PanOrbitCamera>();
                    }
                }
                match switcher.selected {
                    CameraController::Fps => {
                        let character_entity = commands
                            .spawn(FpsCharacterBundle::new(entity))
                            .insert(TransformBundle {
                                local: Transform::from_translation(transform.translation),
                                ..Default::default()
                            })
                            .id();
                        commands
                            .entity(entity)
                            .insert(FpsCameraBundle::new(character_entity));
                    }
                    CameraController::PanOrbit => {
                        commands.entity(entity).insert(PanOrbitCamera::default());
                    }
                };
                switcher.last = switcher.selected;
            }
        }
        Err(_) => {}
    }
}
