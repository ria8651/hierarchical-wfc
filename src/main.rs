use basic_tileset::BasicTileset;
use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    pbr::wireframe::{Wireframe, WireframePipeline, WireframePlugin},
    prelude::*,
    reflect::erased_serde::__private::serde::__private::de,
    render::{
        camera::ScalingMode,
        settings::{WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
    sprite::MaterialMesh2dBundle,
};
use generate_grid::Voronoi;
use grid_wfc::GridWfc;

mod basic_tileset;
mod generate_grid;
mod grid_wfc;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct TileSprite;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut grid_wfc: GridWfc<BasicTileset> = GridWfc::new(UVec2::new(100, 100));
    grid_wfc.collapse(1);

    let tiles = match grid_wfc.validate() {
        Ok(tiles) => tiles,
        Err(e) => {
            error!("Error: {}", e);
            return;
        }
    };

    // for y in (0..tiles[0].len()).rev() {
    //     for x in 0..tiles.len() {
    //         print!("{}", &tiles[x][y]);
    //     }
    //     println!();
    // }

    // camera

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(Voronoi::voronoi(32, 32, 1.0)).into(),
            material: materials.add(ColorMaterial {
                color: Color::RED,
                ..Default::default()
            }),
            ..Default::default()
        },
        Wireframe,
    ));

    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 1.0,
                min_height: 1.0,
            },
            ..Default::default()
        },

        camera_2d: Camera2d {
            clear_color: ClearColorConfig::Custom(Color::rgb(0.15, 0.15, 0.15)),

            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0.5, 0.5, 2.0)),
        ..Default::default()
    });

    // tileset
    // let mut tile_handles: Vec<Handle<Image>> = Vec::new();
    // for tile in 1..=16 {
    //     tile_handles.push(asset_server.load(format!("tileset/{}.png", tile).as_str()));
    // }

    // // result
    // for x in 0..tiles.len() {
    //     for y in 0..tiles[0].len() {
    //         let tile = tiles[x][y];
    //         if tile > 0 {
    //             let pos = Vec2::new(x as f32, y as f32);
    //             commands.spawn((
    //                 SpriteBundle {
    //                     texture: tile_handles[tile as usize - 1].clone(),
    //                     transform: Transform::from_translation(
    //                         ((pos + 0.5) / tiles.len() as f32 - 0.5).extend(0.0),
    //                     ),
    //                     sprite: Sprite {
    //                         custom_size: Some(Vec2::splat(1.0 / tiles.len() as f32)),
    //                         ..default()
    //                     },
    //                     ..default()
    //                 },
    //                 TileSprite,
    //             ));
    //         }
    //     }
    // }
}
