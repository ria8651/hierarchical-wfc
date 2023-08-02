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
        .run();
}

#[derive(Component)]
struct TileSprite;

fn setup(mut commands: Commands) {
    // let tileset_span = info_span!("create_tileset", name = "create_tileset").entered();
    // let tileset = CarcassonneTileset::new();
    // drop(tileset_span);

    // let initialize_span = info_span!("initialize_wfc", name = "initialize_wfc").entered();
    // let size = UVec2::new(100, 100);
    // let mut graph_wfc = GraphWfc::new();
    // drop(initialize_span);

    // let collapse_span = info_span!("collapse_wfc", name = "collapse_wfc").entered();
    // graph_wfc.collapse(&tileset, 0);
    // drop(collapse_span);

    // // for y in (0..size.y as usize).rev() {
    // //     for x in 0..size.x as usize {
    // //         print!("[{:?}]", graph_wfc.tiles[x * size.y as usize + y]);
    // //     }
    // //     println!();
    // // }

    // // for now uses the assumed known ordering of tiles
    // let nodes = match graph_wfc.validate() {
    //     Ok(nodes) => nodes,
    //     Err(e) => {
    //         error!("Error: {}", e);
    //         return;
    //     }
    // };
    // let mut tiles = Vec::new();
    // for x in 0..size.x as usize {
    //     let mut row = Vec::new();
    //     for y in 0..size.y as usize {
    //         row.push(nodes[x * size.y as usize + y]);
    //     }
    //     tiles.push(row);
    // }

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
