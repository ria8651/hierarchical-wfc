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
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<CustomMaterial>::default(),
        ))
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
    mut custom_materials: ResMut<Assets<CustomMaterial>>,
) {
    // let mut grid_wfc: GridWfc<BasicTileset> = GridWfc::new(UVec2::new(100, 100));
    // grid_wfc.collapse(1);

    // let tiles = match grid_wfc.validate() {
    //     Ok(tiles) => tiles,
    //     Err(e) => {
    //         error!("Error: {}", e);
    //         return;
    //     }
    // };

    // for y in (0..tiles[0].len()).rev() {
    //     for x in 0..tiles.len() {
    //         print!("{}", &tiles[x][y]);
    //     }
    //     println!();
    // }
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
            material: custom_materials.add(CustomMaterial {
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

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // left vertical fill (border)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(200.),
                        border: UiRect::all(Val::Px(2.)),
                        ..default()
                    },
                    background_color: Color::rgb(0.65, 0.65, 0.65).into(),
                    ..default()
                })
                .with_children(|parent| {
                    // left vertical fill (content)
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.),
                                ..default()
                            },
                            background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            // text
                            parent.spawn((
                                TextBundle::from_section(
                                    "Text Example",
                                    TextStyle {
                                        font: asset_server.load("fonts/FiraCode-Regular.ttf"),
                                        font_size: 30.0,
                                        color: Color::WHITE,
                                    },
                                )
                                .with_style(Style {
                                    margin: UiRect::all(Val::Px(5.)),
                                    ..default()
                                }),
                                // Because this is a distinct label widget and
                                // not button/list item text, this is necessary
                                // for accessibility to treat the text accordingly.
                                Label,
                            ));
                        });
                });
            // right vertical fill
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        width: Val::Px(200.),
                        ..default()
                    },
                    background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                    ..default()
                })
                .with_children(|parent| {
                    // Title
                    parent.spawn((
                        TextBundle::from_section(
                            "Scrolling list",
                            TextStyle {
                                font: asset_server.load("fonts/FiraCode-Bold.ttf"),
                                font_size: 25.,
                                color: Color::WHITE,
                            },
                        ),
                        Label,
                    ));
                    // List with hidden overflow
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                align_self: AlignSelf::Stretch,
                                height: Val::Percent(50.),
                                overflow: Overflow::clip_y(),
                                ..default()
                            },
                            background_color: Color::rgb(0.10, 0.10, 0.10).into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            // Moving panel
                            parent
                                .spawn((
                                    NodeBundle {
                                        style: Style {
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        ..default()
                                    },
                                    ScrollingList::default(),
                                    AccessibilityNode(NodeBuilder::new(Role::List)),
                                ))
                                .with_children(|parent| {
                                    // List items
                                    for i in 0..30 {
                                        parent.spawn((
                                            TextBundle::from_section(
                                                format!("Item {i}"),
                                                TextStyle {
                                                    font: asset_server
                                                        .load("fonts/FiraCode-Bold.ttf"),
                                                    font_size: 20.,
                                                    color: Color::WHITE,
                                                },
                                            ),
                                            Label,
                                            AccessibilityNode(NodeBuilder::new(Role::ListItem)),
                                        ));
                                    }
                                });
                        });
                });
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        height: Val::Px(200.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(210.),
                        bottom: Val::Px(10.),
                        border: UiRect::all(Val::Px(20.)),
                        ..default()
                    },
                    border_color: Color::GREEN.into(),
                    background_color: Color::rgb(0.4, 0.4, 1.).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: Color::rgb(0.8, 0.8, 1.).into(),
                        ..default()
                    });
                });
            // render order test: reddest in the back, whitest in the front (flex center)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.0),
                                height: Val::Px(100.0),
                                ..default()
                            },
                            background_color: Color::rgb(1.0, 0.0, 0.).into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn(NodeBundle {
                                style: Style {
                                    // Take the size of the parent node.
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(20.),
                                    bottom: Val::Px(20.),
                                    ..default()
                                },
                                background_color: Color::rgb(1.0, 0.3, 0.3).into(),
                                ..default()
                            });
                            parent.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(40.),
                                    bottom: Val::Px(40.),
                                    ..default()
                                },
                                background_color: Color::rgb(1.0, 0.5, 0.5).into(),
                                ..default()
                            });
                            parent.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(60.),
                                    bottom: Val::Px(60.),
                                    ..default()
                                },
                                background_color: Color::rgb(1.0, 0.7, 0.7).into(),
                                ..default()
                            });
                            // alpha test
                            parent.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(80.),
                                    bottom: Val::Px(80.),
                                    ..default()
                                },
                                background_color: Color::rgba(1.0, 0.9, 0.9, 0.4).into(),
                                ..default()
                            });
                        });
                });
            // bevy logo (flex center)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // bevy logo (image)
                    // A `NodeBundle` is used to display the logo the image as an `ImageBundle` can't automatically
                    // size itself with a child node present.
                    parent
                        .spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(500.0),
                                    height: Val::Px(125.0),
                                    margin: UiRect::top(Val::VMin(5.)),
                                    ..default()
                                },
                                // a `NodeBundle` is transparent by default, so to see the image we have to its color to `WHITE`
                                background_color: Color::WHITE.into(),
                                ..default()
                            },
                            UiImage::new(asset_server.load("branding/bevy_logo_dark_big.png")),
                        ))
                        .with_children(|parent| {
                            // alt text
                            // This UI node takes up no space in the layout and the `Text` component is used by the accessibility module
                            // and is not rendered.
                            parent.spawn((
                                NodeBundle {
                                    style: Style {
                                        display: Display::None,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                Text::from_section("Bevy logo", TextStyle::default()),
                            ));
                        });
                });
        });
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material2d for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/tri_point.wgsl".into()
    }
}

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct CustomMaterial {
    #[uniform(0)]
    color: Color,
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
