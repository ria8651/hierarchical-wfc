use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    core_pipeline::clear_color::ClearColorConfig,
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    render::{
        camera::{RenderTarget, ScalingMode},
        render_resource::{
            AddressMode, Extent3d, FilterMode, SamplerDescriptor, TextureDescriptor,
            TextureDimension, TextureFormat, TextureUsages,
        },
        texture::ImageSampler,
        view::RenderLayers,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, mouse_scroll)
        .add_systems(Update, image_nearest)
        .run();
}

#[derive(Component)]
struct TileSprite;

#[derive(Component)]
struct Viewport;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    // fill image.data with zeroes
    image.resize(size);
    let image_handle = images.add(image);
    let ui_layer = RenderLayers::layer(0);
    let scene_layer = RenderLayers::layer(1);

    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::AutoMin {
                    min_width: 1.0,
                    min_height: 1.0,
                },
                ..Default::default()
            },

            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
            camera_2d: Camera2d {
                //2d2a2e
                clear_color: ClearColorConfig::Custom(Color::hex("ff00ff").unwrap()),

                ..Default::default()
            },
            camera: Camera {
                order: -1,
                target: RenderTarget::Image(image_handle.clone()),
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(0.5, 0.5, 2.0)),
            ..Default::default()
        },
        scene_layer,
    ));

    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::AutoMin {
                    min_width: 1.0,
                    min_height: 1.0,
                },
                ..Default::default()
            },

            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
            camera_2d: Camera2d {
                //2d2a2e
                clear_color: ClearColorConfig::Custom(Color::hex("2d2a2e").unwrap()),

                ..Default::default()
            },
            camera: Camera {
                order: 0,
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(0.5, 0.5, 2.0)),
            ..Default::default()
        },
        ui_layer,
    ));

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                ..default()
            },
            ui_layer,
        ))
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

            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),

                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    // a `NodeBundle` is transparent by default, so to see the image we have to its color to `WHITE`
                    background_color: Color::WHITE.into(),
                    border_color: BorderColor {
                        0: Color::hex("adadad").unwrap(),
                    },
                    ..default()
                },
                UiImage::new(image_handle),
                Viewport,
                Label,
                AccessibilityNode(NodeBuilder::new(Role::ListItem)),
            ));
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
                            "Available Tiles",
                            TextStyle {
                                font: asset_server.load("fonts/FiraCode-Regular.ttf"),
                                font_size: 16.,
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
                                            flex_direction: FlexDirection::Row,
                                            flex_wrap: FlexWrap::Wrap,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        ..default()
                                    },
                                    ScrollingList::default(),
                                    Interaction::default(),
                                    AccessibilityNode(NodeBuilder::new(Role::List)),
                                ))
                                .with_children(|parent| {
                                    // List items
                                    for i in 1..=16 {
                                        parent.spawn((
                                            NodeBundle {
                                                style: Style {
                                                    width: Val::Px(64.0),
                                                    height: Val::Px(64.0),

                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                // a `NodeBundle` is transparent by default, so to see the image we have to its color to `WHITE`
                                                background_color: Color::WHITE.into(),
                                                border_color: BorderColor {
                                                    0: Color::hex("adadad").unwrap(),
                                                },
                                                ..default()
                                            },
                                            UiImage::new(asset_server.load(format!(
                                                // "Graphics/cliffside/cliffside_{:03}.png",
                                                "tileset/{}.png",
                                                i
                                            ))),
                                            Label,
                                            AccessibilityNode(NodeBuilder::new(Role::ListItem)),
                                        ));
                                    }
                                });
                        });
                });
        })
        .insert(Interaction::default());
}

#[derive(Component, Default)]
struct ScrollingList {
    position: f32,
    target: f32,
}

fn mouse_scroll(
    time: Res<Time>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Parent, &Node, &Interaction)>,
    query_node: Query<&Node>,
) {
    const SHARPNESS: f32 = 5.0;
    for (mut scrolling_list, mut style, parent, list_node, _interaction) in &mut query_list {
        let items_height = list_node.size().y;
        let container_height = query_node.get(parent.get()).unwrap().size().y;
        let max_scroll = (items_height - container_height).max(0.);

        // Adapted from https://www.reddit.com/r/Unity3D/comments/5qla41/comment/dd0jp6o/?utm_source=share&utm_medium=web2x&context=3
        let t = f32::exp(-SHARPNESS * time.delta_seconds());

        scrolling_list.position = scrolling_list.position * t + (1. - t) * scrolling_list.target;
        scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
        style.top = Val::Px(scrolling_list.position);
    }
    for mouse_wheel_event in mouse_wheel_events.iter() {
        for (mut scrolling_list, _style, parent, list_node, interaction) in &mut query_list {
            dbg!(interaction);
            if interaction != &Interaction::Hovered {
                continue;
            }

            let items_height = list_node.size().y;
            let container_height = query_node.get(parent.get()).unwrap().size().y;
            let max_scroll = (items_height - container_height).max(0.);

            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 64.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };

            scrolling_list.target += dy;
            scrolling_list.target = scrolling_list.target.clamp(-max_scroll, 0.);
        }
    }
}

fn image_nearest(mut ev_asset: EventReader<AssetEvent<Image>>, mut assets: ResMut<Assets<Image>>) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
                if let Some(texture) = assets.get_mut(&handle) {
                    texture.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
                        address_mode_u: AddressMode::ClampToEdge,
                        address_mode_v: AddressMode::ClampToEdge,
                        mag_filter: FilterMode::Nearest,
                        min_filter: FilterMode::Nearest,
                        ..Default::default()
                    });
                };
            }

            AssetEvent::Modified { .. } => {}
            AssetEvent::Removed { .. } => {}
        }
    }
}
