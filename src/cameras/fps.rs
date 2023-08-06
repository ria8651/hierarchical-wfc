use std::f32::consts::PI;

use bevy::{
    self,
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_rapier3d::prelude::{Collider, KinematicCharacterController, RigidBody};

pub struct FpsCameraPlugin;

impl Plugin for FpsCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (fps_camera_mouse, fps_camera_keyboard, fps_manage_mouse),
        );
    }
}
#[derive(Component, Clone, Copy, PartialEq, Debug, Reflect)]
pub struct FpsCameraSettings {
    pub speed: f32,
    pub sensitivity: f32,
}
impl Default for FpsCameraSettings {
    fn default() -> Self {
        Self {
            speed: 10.0,
            sensitivity: 1.0,
        }
    }
}

/// Tags an entity as capable of panning and orbiting.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default, Reflect)]
pub struct FpsCamera {
    pub settings: FpsCameraSettings,
}

#[derive(Bundle)]
pub struct FpsCameraBundle {
    pub camera: FpsCamera,
    pub controller: KinematicCharacterController,
    pub collider: Collider,
    pub rigid_body: RigidBody,
}
impl Default for FpsCameraBundle {
    fn default() -> Self {
        Self {
            camera: FpsCamera::default(),
            controller: KinematicCharacterController {
                ..Default::default()
            },
            collider: Collider::capsule_y(1.0, 0.5),
            rigid_body: RigidBody::KinematicPositionBased,
        }
    }
}

fn fps_manage_mouse(
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    query: Query<With<FpsCamera>>,
) {
    let mut window = primary_window.get_single_mut().unwrap();
    if let Ok(_) = query.get_single() {
        if mouse.pressed(MouseButton::Left) {
            window.cursor.grab_mode = CursorGrabMode::Locked;
            window.cursor.visible = false;
        } else {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        }
        // if keyboard.just_pressed(KeyCode::Escape) {
        //     window.cursor.grab_mode = CursorGrabMode::None;
        //     window.cursor.visible = true;
        // }
    } else {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
}

/// Pan the camera with middle mouse click, zoom with scroll wheel, orbit with right mouse click.
fn fps_camera_mouse(
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut query: Query<(
        &FpsCamera,
        &mut Transform,
        &RigidBody,
        &mut KinematicCharacterController,
        &Camera,
        &GlobalTransform,
    )>,
) {
    if primary_window.get_single().unwrap().cursor.visible {
        return;
    }

    for MouseMotion { delta } in ev_motion.iter() {
        for (fps_camera, mut transform, rb, mut controller, camera, global_transform) in
            query.iter_mut()
        {
            let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

            pitch -= delta.y * fps_camera.settings.sensitivity * 0.01;
            yaw -= delta.x * fps_camera.settings.sensitivity * 0.01;
            let yay = yaw.clamp(-0.5 * PI, 0.5 * PI);
            transform.rotation =
                Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
        }
    }
}
fn fps_camera_keyboard(
    keyboard: Res<Input<KeyCode>>,
    mut query: Query<(&FpsCamera, &Transform, &mut KinematicCharacterController)>,
    time: Res<Time>,
) {
    let mut input = Vec3::ZERO;
    if keyboard.pressed(KeyCode::W) {
        input += Vec3::NEG_Z
    }
    if keyboard.pressed(KeyCode::A) {
        input += Vec3::NEG_X
    }
    if keyboard.pressed(KeyCode::S) {
        input += Vec3::Z
    }
    if keyboard.pressed(KeyCode::D) {
        input += Vec3::X
    }
    if keyboard.pressed(KeyCode::Space) {
        input += Vec3::Y
    }
    if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        input += Vec3::NEG_Y
    }
    for (fps_camera, transform, mut controller) in query.iter_mut() {
        let local_x = (transform.local_x() - transform.local_x().project_onto_normalized(Vec3::Y))
            .normalize_or_zero();
        let local_y = Vec3::Y;
        let local_z = (transform.local_z() - transform.local_z().project_onto_normalized(Vec3::Y))
            .normalize_or_zero();
        let movement = local_x * input.x + local_y * input.y + local_z * input.z;
        controller.translation = Some(time.delta_seconds() * movement * fps_camera.settings.speed);
    }
}

fn get_primary_window_size(windows: &Query<&mut Window, With<PrimaryWindow>>) -> Vec2 {
    let window = windows.get_single().unwrap();
    let window = Vec2::new(window.width() as f32, window.height() as f32);
    window
}
