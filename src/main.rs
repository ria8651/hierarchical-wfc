use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    core_pipeline::clear_color::ClearColorConfig,
    input::mouse::{MouseScrollUnit, MouseWheel},
    pbr::wireframe::{Wireframe, WireframePipeline, WireframePlugin},
    prelude::*,
    reflect::{erased_serde::__private::serde::__private::de, TypePath, TypeUuid},
    render::{
        camera::ScalingMode,
        render_resource::{AsBindGroup, ShaderRef},
        settings::{WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
};

use wfc_lib::{
    basic_tileset::BasicTileset,
    grid_wfc::GridWfc,
    planar_graph_wfc::{PlanarGraph, Wfc},
    point_material::PointMaterial,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Material2dPlugin::<PointMaterial>::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, mouse_scroll)
        .run();
}

#[derive(Component)]
struct TileSprite;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<ColorMaterial>>,
    mut custom_materials: ResMut<Assets<PointMaterial>>,
) {
    let mut graph = PlanarGraph::new_voronoi(32, 32, 1.0);

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

#[derive(Component, Default)]
struct ScrollingList {
    position: f32,
}

fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Parent, &Node)>,
    query_node: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.iter() {
        for (mut scrolling_list, mut style, parent, list_node) in &mut query_list {
            let items_height = list_node.size().y;
            let container_height = query_node.get(parent.get()).unwrap().size().y;

            let max_scroll = (items_height - container_height).max(0.);

            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };

            scrolling_list.position += dy;
            scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
            style.top = Val::Px(scrolling_list.position);
        }
    }
}
