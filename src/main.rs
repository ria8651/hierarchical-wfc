use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, render::camera::ScalingMode};
use carcassonne_tileset::CarcassonneTileset;
use graph_wfc::GraphWfc;
use tileset::TileSet;

mod basic_tileset;
mod carcassonne_tileset;
mod graph_wfc;
mod tileset;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct TileSprite;

type UseTileSet = CarcassonneTileset;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let initialize_span = info_span!("initialize_wfc", name = "initialize_wfc").entered();
    let size = UVec2::new(100, 100);
    let mut graph_wfc: GraphWfc<UseTileSet> = GraphWfc::new(size);
    drop(initialize_span);

    let collapse_span = info_span!("collapse_wfc", name = "collapse_wfc").entered();
    graph_wfc.collapse(0);
    drop(collapse_span);

    // for now uses the assumed known ordering of tiles
    let nodes = match graph_wfc.validate() {
        Ok(nodes) => nodes,
        Err(e) => {
            error!("Error: {}", e);
            return;
        }
    };
    let mut tiles = Vec::new();
    for x in 0..size.x as usize {
        let mut row = Vec::new();
        for y in 0..size.y as usize {
            row.push(nodes[x * size.y as usize + y]);
        }
        tiles.push(row);
    }

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

    // tileset
    let mut tile_handles: Vec<Handle<Image>> = Vec::new();
    for tile in UseTileSet::get_tile_paths() {
        tile_handles.push(asset_server.load(tile));
    }

    // result
    for x in 0..tiles.len() {
        for y in 0..tiles[0].len() {
            let mut tile_index = tiles[x][y] as usize;
            let mut tile_rotation = 0;
            if 1 == 1 {
                tile_rotation = tile_index / 30;
                tile_index = tile_index % 30;
            }
            let pos = Vec2::new(x as f32, y as f32);
            commands.spawn((
                SpriteBundle {
                    texture: tile_handles[tile_index].clone(),
                    transform: Transform::from_translation(
                        ((pos + 0.5) / tiles.len() as f32 - 0.5).extend(0.0),
                    )
                    .with_rotation(Quat::from_rotation_z(
                        std::f32::consts::PI * tile_rotation as f32 / 2.0,
                    )),
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(1.0 / tiles.len() as f32)),
                        ..default()
                    },
                    ..default()
                },
                TileSprite,
            ));
        }
    }
}
