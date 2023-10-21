use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, render::camera::ScalingMode};
use bevy_pancam::{PanCam, PanCamPlugin};
use ui::UiPlugin;
use world::WorldPlugin;

mod ui;
mod world;

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    
    // use grid_wfc::{
    //     grid_graph::{self, GridGraphSettings},
    //     overlapping_tileset::OverlappingTileset,
    // };
    // use core_wfc::{wfc_task::WfcSettings, TileSet, WaveFunction, WfcTask, wfc_backend::SingleThreaded};
    // use std::sync::Arc;
    
    // let sample = vec![
    //     vec![0, 0, 0, 0, 0, 0],
    //     vec![0, 1, 1, 1, 1, 0],
    //     vec![0, 1, 0, 0, 1, 0],
    //     vec![0, 1, 0, 0, 1, 0],
    //     vec![0, 1, 1, 1, 1, 0],
    //     vec![0, 0, 0, 0, 0, 0],
    // ];

    // let tileset = Arc::new(OverlappingTileset::new(sample, 1));
    // // let tileset = Arc::new(OverlappingTileset::from_image(
    // //     "assets/samples/flowers.png",
    // //     1,
    // // ));

    // let settings = GridGraphSettings {
    //     width: 16,
    //     height: 16,
    //     periodic: false,
    // };
    // let graph = grid_graph::create(&settings, WaveFunction::filled(tileset.tile_count()));
    // let mut task = WfcTask {
    //     graph,
    //     tileset: tileset.clone(),
    //     seed: 0,
    //     metadata: None,
    //     settings: WfcSettings::default(),
    // };

    // SingleThreaded::execute(&mut task).unwrap();

    // // for y in (0..settings.height).rev() {
    // //     for x in 0..settings.width {
    // //         print!(
    // //             "{} ",
    // //             format!(
    // //                 "{:?}",
    // //                 task.graph.tiles[y as usize * settings.width as usize + x as usize]
    // //             )
    // //             .as_str()
    // //             .trim_end_matches("0000")
    // //         );
    // //     }
    // //     println!();
    // // }
    // // println!();

    // for y in (0..settings.height).rev() {
    //     for x in 0..settings.width {
    //         let pattern = task.graph.tiles[y * settings.height + x]
    //             .collapse()
    //             .unwrap();
    //         let tile = tileset.get_center_tile(pattern);
    //         print!("{:>3}", tile.0);
    //     }
    //     println!();
    // }
    // println!();

    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            PanCamPlugin,
            UiPlugin,
            WorldPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // camera
    commands.spawn((
        Camera2dBundle {
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
        },
        PanCam::default(),
    ));
}
