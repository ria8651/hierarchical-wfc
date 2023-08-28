// Adapted from https://bevy-cheatbook.github.io/cookbook/pan-orbit-camera.html

use std::time::{Duration, Instant};

use bevy::{
    self,
    input::mouse::MouseWheel,
    prelude::*,
    window::{PrimaryWindow, Window},
};
use bevy_rapier3d::prelude::{RapierContext, Real};

use super::cam_switcher::MainCamera;

pub struct PanOrbitCameraPlugin;

impl Plugin for PanOrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_systems(Startup, spawn_camera)
            // .insert_resource(Msaa::Off)
            // .add_plugins(TemporalAntiAliasPlugin)
            .add_systems(Update, pan_orbit_camera);
    }
}

/// Tags an entity as capable of panning and orbiting.
#[derive(Component, Clone, Copy)]
pub struct PanOrbitCamera {
    /// The "focus point" to orbit around. It is automatically updated when panning the camera
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
    pub initialised: bool,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: 5.0,
            upside_down: false,
            initialised: false,
        }
    }
}
struct DoubleClickTime(Instant);
impl Default for DoubleClickTime {
    fn default() -> Self {
        Self { 0: Instant::now() }
    }
}

fn viewport_rect(window: &Window, camera: &Camera) -> Rect {
    if let Some(viewport) = &camera.viewport {
        let x_0 = viewport.physical_position.x as f32;
        let y_0 = viewport.physical_position.y as f32;
        let x_1 = x_0 + viewport.physical_size.x as f32;
        let y_1 = y_0 + viewport.physical_size.y as f32;
        Rect::new(x_0, y_0, x_1, y_1)
    } else {
        Rect::new(0.0, 0.0, window.width(), window.height())
    }
}

#[derive(Default)]
struct PreviousCursor {
    position: Option<Vec2>,
    dragging: bool,
}

/// Pan the camera with middle mouse click, zoom with scroll wheel, orbit with right mouse click.
fn pan_orbit_camera(
    mut q_primary_window: Query<&mut Window, With<PrimaryWindow>>,
    // mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    mut double_click_time: Local<DoubleClickTime>,
    mut q_camera: Query<
        (
            &mut PanOrbitCamera,
            &mut Transform,
            &Projection,
            &Camera,
            &GlobalTransform,
        ),
        With<MainCamera>,
    >,
    rapier_context: Res<RapierContext>,
    mut previous_cursor: Local<PreviousCursor>,
) {
    let orbit_button = MouseButton::Right;
    let pan_button = MouseButton::Middle;

    let mut primary_window = q_primary_window.get_single_mut().unwrap();
    let Ok((mut pan_orbit, mut transform, projection, camera, global_transform)) =
        q_camera.get_single_mut()
    else {
        return;
    };

    let viewport_rect = viewport_rect(&primary_window, camera);

    let mut pan = Vec2::ZERO;
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let mut orbit_button_changed = false;

    if let Some(cursor_pos) = primary_window.cursor_position() {
        let mut dragging = false;
        if let Some(last_pos) = previous_cursor.position.take() {
            if previous_cursor.dragging || viewport_rect.contains(cursor_pos) {
                if input_mouse.pressed(orbit_button) {
                    rotation_move += cursor_pos - last_pos;
                    dragging = true;
                } else if input_mouse.pressed(pan_button) {
                    // Pan only if we're not rotating at the moment
                    pan += cursor_pos - last_pos;
                    dragging = true;
                } else {
                }
                for ev in ev_scroll.iter() {
                    scroll += ev.y;
                }
            }
        }

        previous_cursor.dragging = dragging;
        if previous_cursor.dragging || viewport_rect.contains(cursor_pos) {
            previous_cursor.position = Some(cursor_pos);
        }

        if dragging {
            let viewport_size = viewport_rect.size();
            let wrapped_pos = ((cursor_pos - viewport_rect.min) / viewport_size).fract()
                * viewport_size
                + viewport_rect.min;

            primary_window.set_cursor_position(Some(wrapped_pos));
            previous_cursor.position = Some(wrapped_pos);
        }
    }

    if input_mouse.just_released(orbit_button) || input_mouse.just_pressed(orbit_button) {
        orbit_button_changed = true;
    }

    {
        let mut update_transform = false;
        if pan_orbit.initialised {
            if orbit_button_changed {
                // only check for upside down when orbiting started or ended this frame
                // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
                let up = transform.rotation * Vec3::Y;
                pan_orbit.upside_down = up.y <= 0.0;
            }

            if input_mouse.just_pressed(MouseButton::Left) {
                let now = Instant::now();
                if now.duration_since(double_click_time.0) < Duration::from_millis(250) {
                    if let Some(cursor_pos) = primary_window.cursor_position() {
                        let cursor_correction = viewport_rect.min;
                        if viewport_rect.contains(cursor_pos) {
                            if let Some(view_ray) = camera
                                .viewport_to_world(global_transform, cursor_pos - cursor_correction)
                            {
                                if let Some(hit) = rapier_context.cast_ray(
                                    view_ray.origin,
                                    view_ray.direction,
                                    Real::MAX,
                                    false,
                                    bevy_rapier3d::prelude::QueryFilter::only_fixed(),
                                ) {
                                    let new_focus = view_ray.origin + view_ray.direction * hit.1;

                                    let cam = global_transform.translation();
                                    let look = (pan_orbit.focus - cam).normalize();
                                    let delta = new_focus - pan_orbit.focus;

                                    pan_orbit.radius += look.dot(delta); // Keep the camera on the same plane
                                    pan_orbit.focus = new_focus;
                                    update_transform = true;
                                }
                            }
                        }
                    }
                } else {
                    *double_click_time = DoubleClickTime(Instant::now());
                }
            }

            if rotation_move.length_squared() > 0.0 {
                update_transform = true;
                let window = get_primary_window_size(&q_primary_window);
                let delta_x = {
                    let delta = rotation_move.x / window.x * std::f32::consts::PI * 2.0;
                    if pan_orbit.upside_down {
                        -delta
                    } else {
                        delta
                    }
                };
                let delta_y = rotation_move.y / window.y * std::f32::consts::PI;
                let yaw = Quat::from_rotation_y(-delta_x);
                let pitch = Quat::from_rotation_x(-delta_y);
                transform.rotation = yaw * transform.rotation; // rotate around global y axis
                transform.rotation = transform.rotation * pitch; // rotate around local x axis
            } else if pan.length_squared() > 0.0 {
                update_transform = true;
                // make panning distance independent of resolution and FOV,
                let window = get_primary_window_size(&q_primary_window);
                if let Projection::Perspective(projection) = projection {
                    pan *= Vec2::new(projection.fov * projection.aspect_ratio, projection.fov)
                        / window;
                }
                // translate by local axes
                let right = transform.rotation * Vec3::X * -pan.x;
                let up = transform.rotation * Vec3::Y * pan.y;
                // make panning proportional to distance away from focus point
                let translation = (right + up) * pan_orbit.radius;
                pan_orbit.focus += translation;
            } else if scroll.abs() > 0.0 {
                update_transform = true;
                pan_orbit.radius -= scroll * pan_orbit.radius * 0.2;
                // dont allow zoom to reach zero or you get stuck
                pan_orbit.radius = f32::max(pan_orbit.radius, 0.05);
            }
        } else {
            pan_orbit.initialised = true;

            let ray_origin = global_transform.translation();
            let ray_dir = global_transform.forward();
            let hit = rapier_context.cast_ray(
                ray_origin,
                ray_dir,
                Real::MAX,
                false,
                bevy_rapier3d::prelude::QueryFilter::only_fixed(),
            );
            let distance = match hit {
                Some((_, t)) => t,
                None => 10.0,
            };

            let new_focus = ray_origin + ray_dir * distance;
            pan_orbit.radius = (ray_origin - new_focus).length();
            pan_orbit.focus = new_focus;
            update_transform = true;
        }

        if update_transform {
            // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
            // parent = x and y rotation
            // child = z-offset
            let rot_matrix = Mat3::from_quat(transform.rotation);
            transform.translation =
                pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
        }
    }

    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    // ev_motion.clear();
}

fn get_primary_window_size(windows: &Query<&mut Window, With<PrimaryWindow>>) -> Vec2 {
    let window = windows.get_single().unwrap();
    let window = Vec2::new(window.width() as f32, window.height() as f32);
    window
}

// Spawn a camera like this
// fn spawn_camera(mut commands: Commands) {
//     let translation = Vec3::new(-2.0, 2.5, 5.0);
//     let radius = translation.length();

//     commands
//         .spawn((
//             Camera3dBundle {
//                 camera: Camera {
//                     hdr: true,
//                     ..Default::default()
//                 },
//                 tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
//                 transform: Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
//                 ..Default::default()
//             },
//             PanOrbitCamera {
//                 radius,
//                 ..Default::default()
//             },
//             ContrastAdaptiveSharpeningSettings {
//                 enabled: false,
//                 ..default()
//             },
//         ))
//         .insert(ScreenSpaceAmbientOcclusionBundle {
//             settings: ScreenSpaceAmbientOcclusionSettings {
//                 quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
//                 ..Default::default()
//             },
//             ..Default::default()
//         })
//         .insert(TemporalAntiAliasBundle {
//             ..Default::default()
//         });
// }
