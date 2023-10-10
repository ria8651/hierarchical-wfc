use std::sync::Arc;

use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, render::camera::ScalingMode};
use bevy_pancam::{PanCam, PanCamPlugin};
use grid_wfc::{
    carcassonne_tileset::CarcassonneTileset,
    overlapping_graph::{self, OverlappingGraphSettings},
};
use hierarchical_wfc::{
    wfc_backend::SingleThreaded, wfc_task::WfcSettings, TileSet, WaveFunction, WfcTask,
};
use ui::UiPlugin;
use world::WorldPlugin;

mod ui;
mod world;

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

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

    let tileset = Arc::new(CarcassonneTileset::default());
    let settings = OverlappingGraphSettings {
        width: 16,
        height: 16,
        overlap: 1,
        periodic: false,
    };
    let graph = overlapping_graph::create(&settings, WaveFunction::filled(tileset.tile_count()));
    let mut task = WfcTask {
        graph,
        tileset,
        seed: 0,
        metadata: None,
        settings: WfcSettings::default(),
    };

    SingleThreaded::execute(&mut task).unwrap();

    for y in (0..settings.height).rev() {
        for x in 0..settings.width {
            print!(
                "{:>5}",
                task.graph.tiles[x as usize * settings.height as usize + y as usize].count_bits()
            );
        }
        println!();
    }
    println!();
}
