use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::camera_plugin::pan_orbit::AlignViewEvent;

use egui::Rgba;

const EGUI_X_COLOR: Rgba = egui::Rgba::from_rgb(0.8, 0.2, 0.2);
const EGUI_Y_COLOR: Rgba = egui::Rgba::from_rgb(0.2, 0.8, 0.2);
const EGUI_Z_COLOR: Rgba = egui::Rgba::from_rgb(0.2, 0.2, 0.8);

pub struct OrientationGizmoPlugin;

impl Plugin for OrientationGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, orientation_gizmo_system);
    }
}

fn orientation_gizmo_system(
    mut contexts: EguiContexts,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut ev_align_view: EventWriter<AlignViewEvent>,
) {
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

    egui::Area::new("Viewport")
        .fixed_pos((0.0, 0.0))
        .show(&contexts.ctx_mut(), |ui| {
            ui.with_layer_id(egui::LayerId::background(), |ui| {
                let painter = ui.painter();

                let padding =
                    egui::vec2(16.0, 16.0 + egui_dock::style::TabBarStyle::default().height);
                let radius = 24.0f32;
                // let center =
                //     (0.5 * viewport.min.to_vec2() + 0.5 * viewport.max.to_vec2()).to_pos2();

                let center = egui::pos2(
                    viewport.max.x - radius - padding.x,
                    viewport.min.y + radius + padding.y,
                );

                if let Some(pos) = ui.input(|input| input.pointer.hover_pos()) {
                    if (center - pos).length_sq() < (radius + 12.0) * (radius + 12.0) {
                        painter.circle_filled(
                            center,
                            radius + 12.0,
                            egui::Rgba::from_luminance_alpha(1.0, 0.2),
                        );
                    }
                };

                let inv_view_matrix = camera_transform.compute_matrix().inverse();

                let mut axis = [
                    (
                        "X",
                        Vec3::X,
                        EGUI_X_COLOR,
                        egui::Rgba::from_rgb(0.6, 0.3, 0.3),
                        true,
                    ),
                    (
                        "Y",
                        Vec3::Y,
                        EGUI_Y_COLOR,
                        egui::Rgba::from_rgb(0.3, 0.6, 0.3),
                        true,
                    ),
                    (
                        "Z",
                        Vec3::Z,
                        EGUI_Z_COLOR,
                        egui::Rgba::from_rgb(0.3, 0.3, 0.6),
                        true,
                    ),
                    (
                        "-X",
                        Vec3::NEG_X,
                        EGUI_X_COLOR,
                        egui::Rgba::from_rgb(0.6, 0.3, 0.3),
                        false,
                    ),
                    (
                        "-Y",
                        Vec3::NEG_Y,
                        EGUI_Y_COLOR,
                        egui::Rgba::from_rgb(0.3, 0.6, 0.3),
                        false,
                    ),
                    (
                        "-Z",
                        Vec3::NEG_Z,
                        EGUI_Z_COLOR,
                        egui::Rgba::from_rgb(0.3, 0.3, 0.6),
                        false,
                    ),
                ]
                .map(|data| {
                    (
                        data.0,
                        data.1,
                        inv_view_matrix * data.1.extend(0.0),
                        data.2,
                        data.3,
                        data.4,
                    )
                });
                axis.sort_by(|a, b| PartialOrd::partial_cmp(&a.2.z, &b.2.z).unwrap());

                for (letter, axis, screen_space_axis, color, secondary_color, primary) in axis {
                    let screen_space_axis =
                        egui::vec2(screen_space_axis.x, -screen_space_axis.y) * radius;
                    let screen_space_axis = center + screen_space_axis;

                    if primary {
                        painter.line_segment([center, screen_space_axis], (3.0, color));
                    }

                    let mut hovered = false;
                    if let (Some(pos), clicked) = ui
                        .input(|input| (input.pointer.hover_pos(), input.pointer.primary_clicked()))
                    {
                        if (screen_space_axis - pos).length_sq() < 6.0 * 6.0 {
                            if clicked {
                                ev_align_view.send(AlignViewEvent(-axis));
                            }
                            hovered = true;
                            painter.circle(screen_space_axis, 8.0, secondary_color, (2.0, color));
                            painter.text(
                                screen_space_axis + egui::vec2(1.0, 1.0),
                                egui::Align2::CENTER_CENTER,
                                letter,
                                egui::FontId {
                                    size: 10.,
                                    family: egui::FontFamily::Monospace,
                                },
                                egui::Color32::WHITE,
                            );
                        }
                    }

                    if !hovered {
                        if primary {
                            painter.circle_filled(screen_space_axis, 6.0, color);
                            painter.text(
                                screen_space_axis + egui::vec2(0.5, 0.5),
                                egui::Align2::CENTER_CENTER,
                                letter,
                                egui::FontId {
                                    size: 10.,
                                    family: egui::FontFamily::Monospace,
                                },
                                egui::Color32::BLACK,
                            );
                        } else {
                            painter.circle(screen_space_axis, 6.0, secondary_color, (2.0, color));
                        }
                    }
                }
            });
        });
}
