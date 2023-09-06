use std::sync::Arc;

use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, render::camera::ScalingMode};
use hierarchical_wfc::{CpuExecutor, Peasant, TileSet, WaveFunction};
use ui::UiPlugin;
use utilities::{
    graph_grid::{self, GridGraphSettings},
    mxgmn_tileset::MxgmnTileset,
};
use world::WorldPlugin;

mod ui;
mod world;

fn main() {
    // let tileset = MxgmnTileset::new(
    //     "/Users/brian/Documents/Code/Rust/hierarchical-wfc/assets/mxgmn/Circuit.xml".to_string(),
    // );
    // let settings = GridGraphSettings {
    //     width: 2,
    //     height: 2,
    //     periodic: false,
    // };
    // let graph = graph_grid::create(&settings, WaveFunction::filled(tileset.tile_count()));
    // let mut peasant = Peasant {
    //     graph,
    //     constraints: Arc::new(tileset.get_constraints()),
    //     weights: Arc::new(tileset.get_weights()),
    //     seed: 0,
    //     user_data: None,
    // };
    // CpuExecutor::execute(&mut peasant);

    // for y in 0..2 {
    //     for x in 0..2 {
    //         let tile = peasant.graph.tiles[x as usize * 2 as usize + y as usize].clone();
    //         print!("{:?} ", tile);
    //     }
    //     println!();
    // }

    // panic!();

    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((UiPlugin, WorldPlugin))
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
