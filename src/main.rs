#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, render::camera::ScalingMode};
use ui::UiPlugin;

mod basic_tileset;
mod carcassonne_tileset;
mod graph;
mod graph_grid;
mod tileset;
mod ui;
mod wfc;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(UiPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, mouse_scroll)
        .run();
}

fn setup(mut commands: Commands) {
    // camera
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 1.0,
                min_height: 1.0,
            },
            ..Default::default()
        },
        tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
        camera_2d: Camera2d {
            clear_color: ClearColorConfig::Custom(Color::hex("2d2a2e").unwrap()),

            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
        ..Default::default()
    });
}
