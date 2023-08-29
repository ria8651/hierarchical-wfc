use std::f32::consts::PI;

use bevy::{
    self,
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_rapier3d::prelude::{
    Ccd, CharacterLength, Collider, GravityScale, KinematicCharacterController,
    KinematicCharacterControllerOutput, LockedAxes, RigidBody, Sleeping, Velocity,
};

pub struct FpsCameraPlugin;

impl Plugin for FpsCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                fps_gravity,
                fps_character_keyboard,
                fps_manage_mouse,
                fps_camera_mouse,
                fps_camera_track_character
                    .after(fps_gravity)
                    .after(fps_character_keyboard),
            ),
        );
    }
}

#[derive(Component, Clone, Copy, PartialEq, Debug, Reflect)]
pub struct FpsCameraSettings {
    pub flying: bool,
    pub speed: f32,
    pub gravity: f32,
    pub jump_velocity: f32,
    pub sensitivity: f32,
}
impl Default for FpsCameraSettings {
    fn default() -> Self {
        Self {
            flying: true,
            speed: 10.0,
            jump_velocity: 5.0,
            gravity: 2.0,
            sensitivity: 1.0,
        }
    }
}

#[derive(Component, Clone, Copy, Default)]
pub struct FpsCamera;

#[derive(Component)]
pub struct FpsCharacter(Entity);
#[derive(Component)]
pub struct FpsCameraAttachedCharacter(Entity);

#[derive(Bundle)]
pub struct FpsCameraBundle {
    pub camera: FpsCamera,
    pub settings: FpsCameraSettings,
    pub attached_character: FpsCameraAttachedCharacter,
}
impl FpsCameraBundle {
    pub fn new(character: Entity) -> Self {
        Self {
            camera: FpsCamera,
            settings: FpsCameraSettings::default(),
            attached_character: FpsCameraAttachedCharacter(character),
        }
    }
}

#[derive(Bundle)]
pub struct FpsCharacterBundle {
    pub character: FpsCharacter,

    pub velocity: Velocity,

    pub controller: KinematicCharacterController,

    pub collider: Collider,
    pub rigid_body: RigidBody,

    pub ccd: Ccd,
    pub gravity: GravityScale,
    pub sleeping: Sleeping,
    pub locked_axis: LockedAxes,
}
impl FpsCharacterBundle {
    pub fn new(camera: Entity) -> Self {
        Self {
            character: FpsCharacter(camera),
            velocity: Velocity::default(),
            controller: KinematicCharacterController {
                snap_to_ground: Some(CharacterLength::Relative(0.1)),
                ..Default::default()
            },
            collider: Collider::capsule_y(1.0, 0.5),
            rigid_body: RigidBody::Dynamic,
            ccd: Ccd::enabled(),
            gravity: GravityScale(0.0),
            sleeping: Sleeping::disabled(),
            locked_axis: LockedAxes::ROTATION_LOCKED_X.union(LockedAxes::ROTATION_LOCKED_Z),
        }
    }
}

fn fps_camera_track_character(
    mut commands: Commands,
    mut q_camera: Query<&mut Transform, (With<FpsCamera>, Without<FpsCharacter>)>,
    q_character: Query<(Entity, &Transform, &FpsCharacter), Without<FpsCamera>>,
) {
    for (character_entity, transform, FpsCharacter(camera_entity)) in q_character.iter() {
        if let Ok(mut camera) = q_camera.get_mut(*camera_entity) {
            camera.translation = transform.translation;
        } else {
            commands.entity(character_entity).despawn();
        }
    }
}

fn fps_manage_mouse(
    mut q_primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mouse: Res<Input<MouseButton>>,
    q_camera: Query<With<FpsCamera>>,
) {
    let mut primary_window = q_primary_window.get_single_mut().unwrap();
    if q_camera.get_single().is_ok() {
        if mouse.pressed(MouseButton::Left) {
            primary_window.cursor.grab_mode = CursorGrabMode::Locked;
            primary_window.cursor.visible = false;
        } else {
            primary_window.cursor.grab_mode = CursorGrabMode::None;
            primary_window.cursor.visible = true;
        }
    }
}

fn fps_camera_mouse(
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut q_camera: Query<(&FpsCameraSettings, &mut Transform), With<FpsCamera>>,
) {
    if primary_window.get_single().unwrap().cursor.visible {
        return;
    }

    for MouseMotion { delta } in ev_motion.iter() {
        for (settings, mut transform) in q_camera.iter_mut() {
            let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

            pitch -= delta.y * settings.sensitivity * 0.01;
            let pitch = pitch.clamp(-0.5 * PI + 0.001, 0.5 * PI - 0.001);
            yaw -= delta.x * settings.sensitivity * 0.01;
            transform.rotation =
                Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
        }
    }
}
fn fps_character_keyboard(
    keyboard: Res<Input<KeyCode>>,
    q_camera: Query<(&FpsCameraSettings, &Transform, &FpsCameraAttachedCharacter), With<FpsCamera>>,
    mut q_character: Query<(
        &mut KinematicCharacterController,
        &FpsCharacter,
        &mut Velocity,
        Option<&KinematicCharacterControllerOutput>,
    )>,
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
    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
        input += Vec3::NEG_Y
    }
    for (mut controller, FpsCharacter(camera_entity), mut velocity, output) in
        q_character.iter_mut()
    {
        if let Ok((camera_settings, camera_transform, _)) = q_camera.get(*camera_entity) {
            if camera_settings.flying {
                let local_x = camera_transform.local_x();
                let local_y = Vec3::Y;
                let local_z = camera_transform.local_z();
                let movement = local_x * input.x + local_y * input.y + local_z * input.z;
                controller.translation =
                    Some(time.delta_seconds() * movement * camera_settings.speed);
            } else {
                let local_x = (camera_transform.local_x()
                    - camera_transform.local_x().project_onto_normalized(Vec3::Y))
                .normalize();
                let local_z = (camera_transform.local_z()
                    - camera_transform.local_z().project_onto_normalized(Vec3::Y))
                .normalize();
                let movement = local_x * input.x + local_z * input.z;

                let snap = Vec3::NEG_Y
                    * match output.map(|output| output.grounded)
                        .unwrap_or(false)
                    {
                        true => 0.1,
                        false => 2e-5,
                    };

                controller.translation =
                    Some(time.delta_seconds() * movement * camera_settings.speed + snap);

                if let Some(output) = output {
                    dbg!(output.grounded);
                    if output.grounded && keyboard.pressed(KeyCode::Space) {
                        velocity.linvel += Vec3::Y * camera_settings.jump_velocity;
                    }
                }
            }
        }
    }
}

fn fps_gravity(
    q_camera: Query<&FpsCameraSettings, With<FpsCamera>>,
    mut q_character: Query<(&FpsCharacter, &mut GravityScale), Without<FpsCamera>>,
) {
    for (FpsCharacter(camera_entity), mut gravity) in q_character.iter_mut() {
        if let Ok(settings) = q_camera.get(*camera_entity) {
            if settings.flying {
                gravity.0 = 0.0;
            } else {
                gravity.0 = settings.gravity;
            }
        }
    }
}
