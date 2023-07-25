use std::fmt::{self, Debug};
use std::time::Duration;

use bevy::asset::ChangeWatcher;
use bevy::core_pipeline::clear_color::ClearColorConfig;

use bevy::prelude::{AssetPlugin, PluginGroup};

use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;

use bevy_inspector_egui::bevy_egui::{self, EguiContexts};

use bevy::render::camera::{CameraRenderGraph, ScalingMode};
use bevy::window::{PresentMode, Window, WindowPlugin};

use bevy_inspector_egui::egui::{self, Ui};
use wfc_lib::editor_ui::MainCamera;
use wfc_lib::point_material::PointMaterial;
use wfc_lib::render_pipeline::MainPassSettings;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(AssetPlugin {
                watch_for_changes: Some(ChangeWatcher {
                    delay: Duration::from_millis(200),
                }),
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Immediate,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        Material2dPlugin::<PointMaterial>::default(),
    ))
    .add_plugins(bevy_egui::EguiPlugin)
    .add_systems(Update, (ui_system, elapsed_widget_system))
    .add_systems(Startup, setup);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            camera_render_graph: CameraRenderGraph::new("core_2d"),

            projection: OrthographicProjection {
                scaling_mode: ScalingMode::AutoMin {
                    min_width: 64.0,
                    min_height: 64.0,
                },
                ..Default::default()
            },
            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::hex("2d2a2e").unwrap()),
                ..Default::default()
            },

            transform: Transform::from_translation(Vec3::new(0.5, 0.5, 2.0)),
            ..Default::default()
        },
        MainCamera,
        MainPassSettings {},
    ));

    commands.spawn(Widget {
        ui: Box::new(|ui: &mut Ui| {
            ui.label("foo");
        }),
    });
    commands.spawn((
        Widget {
            ui: Box::new(|ui: &mut Ui| {
                ui.label("bar");
            }),
        },
        ElapsedWidget,
    ));
}

fn ui_system(mut contexts: EguiContexts, widgets_q: Query<&Widget>) {
    egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
        ui.label("world");

        for Widget { ui: widget } in widgets_q.iter() {
            widget(ui);
        }
    });
}

fn elapsed_widget_system(time: Res<Time>, mut widgets_q: Query<&mut Widget, With<ElapsedWidget>>) {
    let time = time.elapsed_seconds();
    for mut widget in widgets_q.iter_mut() {
        widget.as_mut().ui = Box::new(move |ui: &mut Ui| {
            ui.label(format!("Elapsed: {:.2}", time));
        });
    }
}

#[derive(Component)]
struct Widget {
    ui: Box<dyn Fn(&mut Ui) -> () + Send + Sync + 'static>,
}

#[derive(Component)]
struct ElapsedWidget;
