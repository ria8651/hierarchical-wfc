#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, render::camera::ScalingMode};
use ui::UiPlugin;

mod basic_tileset;
mod carcassonne_tileset;
mod graph;
mod graph_grid;
mod graph_grid_8D;
mod hierarchical_tileset;
mod tileset;
mod ui;
mod wfc;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(UiPlugin)
        .add_systems(Startup, setup)
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
            ..default()
        },
        camera_2d: Camera2d {
            clear_color: ClearColorConfig::Custom(Color::rgb(0.15, 0.15, 0.15)),
        },
        ..default()
    });
}
