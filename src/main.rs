use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, render::camera::ScalingMode};
use carcassonne_tileset::CarcassonneTileset;
use grid_wfc::GridWfc;

mod basic_tileset;
mod carcassonne_tileset;
mod grid_wfc;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Material2dPlugin::<PointMaterial>::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, mouse_scroll)
        .run();
}

#[derive(Component)]
struct TileSprite;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut grid_wfc: GridWfc<CarcassonneTileset> = GridWfc::new(UVec2::new(15, 15));
    grid_wfc.collapse(10);

    graph.collapse(0);
    graph.validate();

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(graph.mesh_edges()).into(),
            material: standard_materials.add(ColorMaterial {
                color: Color::hex("727272").unwrap(),
                ..Default::default()
            }),
            ..Default::default()
        },
        Wireframe,
    ));

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(graph.mesh_nodes()).into(),
            material: custom_materials.add(PointMaterial {
                color: Color::WHITE,
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
        tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
        camera_2d: Camera2d {
            clear_color: ClearColorConfig::Custom(Color::hex("2d2a2e").unwrap()),

            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
        ..Default::default()
    });
}

    // tileset
    let mut tile_handles: Vec<Handle<Image>> = Vec::new();
    for tile in 0..=17 {
        tile_handles.push(asset_server.load(format!("carcassonne/{}.png", tile + 1).as_str()));
    }

    // result
    for x in 0..tiles.len() {
        for y in 0..tiles[0].len() {
            let tile = tiles[x][y] as usize;
            let tile_index = tile % 18;
            let tile_rotation = tile / 18;
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
