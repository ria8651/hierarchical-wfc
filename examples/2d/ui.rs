use crate::world::{GenerateEvent, MaybeWorld};
use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::{EguiContexts, EguiPlugin},
    egui::{
        self, panel::Side, CollapsingHeader, Color32, Frame, Id, ScrollArea, SidePanel, TextureId,
    },
    reflect_inspector::ui_for_value,
    DefaultInspectorConfigPlugin,
};
use grid_wfc::{
    basic_tileset::BasicTileset,
    carcassonne_tileset::CarcassonneTileset,
    grid_graph::GridGraphSettings,
    mxgmn_tileset::MxgmnTileset,
    overlapping_tileset::OverlappingTileset,
    world::{ChunkSettings, ChunkState},
};
use hierarchical_wfc::{wfc_task::WfcSettings, TileRender, TileSet};
use serde::Deserialize;
use std::sync::Arc;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_plugins(DefaultInspectorConfigPlugin)
            .add_event::<RenderUpdateEvent>()
            .init_resource::<UiState>()
            .init_resource::<UiSettings>()
            .register_type::<UiSettings>()
            .register_type::<GridGraphSettings>()
            .add_systems(Update, (ui, render_world, debug_gizmos).chain());
    }
}

#[derive(Resource, Reflect)]
struct UiSettings {
    seed: u64,
    random_seed: bool,
    graph_settings: GridGraphSettings,
    deterministic: bool,
    multithreaded: bool,
    chunk_settings: ChunkSettings,
    wfc_settings: WfcSettings,
    draw_gizmos: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            random_seed: true,
            graph_settings: Default::default(),
            deterministic: false,
            multithreaded: true,
            chunk_settings: Default::default(),
            wfc_settings: Default::default(),
            draw_gizmos: false,
        }
    }
}

#[derive(Resource)]
struct UiState {
    picked_tileset: usize,
    tile_sets: Vec<(Arc<dyn TileSet>, String)>,
    weights: Vec<f32>,
    tile_render_assets: Vec<Bleh>,
    tile_entities: Vec<Vec<Entity>>,
}

impl Default for UiState {
    fn default() -> Self {
        let mut tile_sets: Vec<(Arc<dyn TileSet>, String)> = vec![
            (
                Arc::new(CarcassonneTileset::default()),
                "CarcassonneTileset".to_string(),
            ),
            (
                Arc::new(BasicTileset::default()),
                "BasicTileset".to_string(),
            ),
        ];

        tile_sets.push((
            Arc::new(OverlappingTileset::from_image(
                "assets/dungeon.png".to_string(),
                1,
                8,
            )),
            "Dungeon".to_string(),
        ));

        let paths = std::fs::read_dir("assets/mxgmn").unwrap();
        for path in paths {
            let path = path.unwrap().path();
            if let Some(ext) = path.extension() {
                if ext == "xml" {
                    tile_sets.push((
                        Arc::new(MxgmnTileset::new(&path, None).unwrap()),
                        path.file_stem().unwrap().to_str().unwrap().to_string(),
                    ));
                }
            }
        }

        // tile_sets.push((
        //     Arc::new(
        //         MxgmnTileset::new(
        //             Path::new("assets/mxgmn/Circuit.xml"),
        //             Some("Turnless".to_string()),
        //         )
        //         .unwrap(),
        //     ),
        //     "Circuit 2".to_string(),
        // ));

        // let paths = std::fs::read_dir("assets/samples").unwrap();
        // for path in paths {
        //     let path = path.unwrap().path();
        //     if let Some(ext) = path.extension() {
        //         if ext == "png" {
        //             tile_sets.push((
        //                 Arc::new(OverlappingTileset::from_image(path.to_str().unwrap(), 1)),
        //                 path.file_stem().unwrap().to_str().unwrap().to_string(),
        //             ));
        //         }
        //     }
        // }
        let xml = std::fs::read_to_string("assets/samples.xml").unwrap();
        let samples: Samples = serde_xml_rs::from_str(&xml).unwrap();
        for sample in samples.overlapping.into_iter() {
            let overlap = sample.n / 2;
            println!("overlap: {}", overlap);
            tile_sets.push((
                Arc::new(OverlappingTileset::from_image(
                    format!("assets/samples/{}.png", sample.name),
                    overlap,
                    sample.symmetry,
                )),
                format!("{} {} {}", sample.name, sample.n, sample.symmetry),
            ));
        }

        Self {
            picked_tileset: 4,
            tile_sets,
            weights: Vec::new(),
            tile_render_assets: Vec::new(),
            tile_entities: Vec::new(),
        }
    }
}

#[derive(Component)]
struct TileSprite;

fn ui(
    mut contexts: EguiContexts,
    mut ui_settings: ResMut<UiSettings>,
    mut ui_state: ResMut<UiState>,
    type_registry: Res<AppTypeRegistry>,
    asset_server: Res<AssetServer>,
    mut generate_events: EventWriter<GenerateEvent>,
    // mut world: ResMut<MaybeWorld>,
    // mut render_world_event: EventWriter<RenderUpdateEvent>,
) {
    let mut tileset = ui_state.tile_sets[ui_state.picked_tileset].0.clone();

    if ui_state.weights.len() != tileset.tile_count() {
        ui_state.weights = tileset.get_weights().as_ref().clone();

        for handle in ui_state.tile_render_assets.drain(..) {
            if let Bleh::Image { bevy_handle, .. } = &handle {
                contexts.remove_image(bevy_handle);
            }
        }
        for (tile_render_assets, transform) in tileset.get_render_tile_assets() {
            ui_state.tile_render_assets.push(match tile_render_assets {
                TileRender::Sprite(path) => {
                    let bevy_handle = asset_server.load(path);
                    let egui_handle = contexts.add_image(bevy_handle.clone_weak());
                    Bleh::Image {
                        transform,
                        bevy_handle,
                        egui_handle,
                    }
                }
                TileRender::Color(color) => Bleh::Color(color),
            });
        }
    }

    SidePanel::new(Side::Left, Id::new("left_panel"))
        .resizable(true)
        .width_range(104.0..=1000.0)
        .frame(
            Frame::default()
                .inner_margin(20.0)
                .fill(Color32::from_rgb(38, 38, 38)),
        )
        .show(contexts.ctx_mut(), |ui| {
            ScrollArea::new([false, true]).show(ui, |ui| {
                CollapsingHeader::new("WFC Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        let selected = ui_state.tile_sets[ui_state.picked_tileset].1.clone();
                        egui::ComboBox::from_label("Tileset")
                            .selected_text(selected.to_string())
                            .show_ui(ui, |ui| {
                                for (i, tileset) in ui_state.tile_sets.clone().iter().enumerate() {
                                    ui.selectable_value(
                                        &mut ui_state.picked_tileset,
                                        i,
                                        tileset.1.clone(),
                                    );
                                }
                            });

                        ui.add_space(10.0);
                        ui_for_value(ui_settings.as_mut(), ui, &type_registry.read());

                        let seed = if !ui_settings.random_seed {
                            ui_settings.seed
                        } else {
                            rand::random()
                        };

                        dyn_clone::arc_make_mut(&mut tileset).set_weights(ui_state.weights.clone());
                        if ui.button("Generate Single").clicked() {
                            generate_events.send(GenerateEvent::Single {
                                tileset,
                                settings: ui_settings.graph_settings.clone(),
                                wfc_settings: ui_settings.wfc_settings.clone(),
                                seed,
                            });
                        } else if ui.button("Generate Chunked").clicked() {
                            generate_events.send(GenerateEvent::Chunked {
                                tileset,
                                settings: ui_settings.graph_settings.clone(),
                                wfc_settings: ui_settings.wfc_settings.clone(),
                                chunk_settings: ui_settings.chunk_settings,
                                multithreaded: ui_settings.multithreaded,
                                deterministic: ui_settings.deterministic,
                                seed,
                            });

                            // good for debugging
                            // let (new_world, _) = grid_wfc::single_shot::generate_world(
                            //     tileset,
                            //     &mut hierarchical_wfc::wfc_backend::MultiThreaded::new(8),
                            //     ui_state.grid_graph_settings.clone(),
                            //     seed,
                            //     match ui_state.deterministic {
                            //         true => grid_wfc::world::GenerationMode::Deterministic,
                            //         false => grid_wfc::world::GenerationMode::NonDeterministic,
                            //     },
                            //     ui_state.chunk_size,
                            //     ui_state.overlap,
                            //     ui_state.backtracking.clone(),
                            // );
                            // world.as_mut().replace(new_world);
                            // render_world_event.send(RenderUpdateEvent);
                        }
                    });

                // CollapsingHeader::new("Tileset Settings")
                //     .default_open(true)
                //     .show(ui, |ui| {
                //         egui::Grid::new("some_unique_id").show(ui, |ui| {
                //             for i in 0..ui_state.weights.len() {
                //                 ui.vertical_centered(|ui| {
                //                     let handle = &ui_state.tile_render_assets[i];
                //                     // if let Some(handle) = handle {
                //                     //     ui.image(handle.0, [64.0, 64.0]);
                //                     // } else {
                //                     //     ui.colored_label(Color32::from_rgb(255, 0, 0), "No image");
                //                     // }
                //                     match handle {
                //                         Bleh::Image { egui_handle, .. } => {
                //                             ui.image(*egui_handle, [64.0, 64.0]);
                //                         }
                //                         Bleh::Color(_) => {
                //                             ui.colored_label(
                //                                 Color32::from_rgb(255, 0, 0),
                //                                 "No image",
                //                             );
                //                         }
                //                     }
                //                     ui.add(DragValue::new(&mut ui_state.weights[i]));
                //                 });

                //                 if i % 4 == 3 {
                //                     ui.end_row();
                //                 }
                //             }
                //         });
                //     });
            });
        });
}

fn debug_gizmos(mut gizmos: Gizmos, world: Res<MaybeWorld>, ui_settings: Res<UiSettings>) {
    if ui_settings.draw_gizmos {
        let world = match world.as_ref().as_ref() {
            Some(world) => world,
            None => return,
        };

        let height = world.world[0].len();

        for (chunk, state) in world.generated_chunks.iter() {
            let color = match state {
                ChunkState::Scheduled => Color::rgb(0.0, 0.0, 1.0),
                ChunkState::Done => Color::rgb(0.0, 1.0, 0.0),
                ChunkState::Failed => Color::rgb(1.0, 0.0, 0.0),
            };

            let (bottom_left, top_right) = world.chunk_bounds(*chunk, world.chunk_settings.overlap);
            let (bottom_left, top_right) = (
                bottom_left.as_vec2() / height as f32 - 0.5,
                top_right.as_vec2() / height as f32 - 0.5,
            );
            let center = (bottom_left + top_right) / 2.0;
            let size = top_right - bottom_left;
            gizmos.rect_2d(center, 0.0, size, color);
        }
    }
}

#[derive(Event)]
pub struct RenderUpdateEvent;

pub enum Bleh {
    Image {
        transform: Transform,
        bevy_handle: Handle<Image>,
        egui_handle: TextureId,
    },
    Color(Color),
}

/// disgusting
fn render_world(
    mut commands: Commands,
    mut ui_state: ResMut<UiState>,
    asset_server: Res<AssetServer>,
    mut tile_entity_query: Query<
        (&mut Transform, &mut Handle<Image>, &mut Sprite),
        With<TileSprite>,
    >,
    world: Res<MaybeWorld>,
    mut render_world_event: EventReader<RenderUpdateEvent>,
    mut current_size: Local<IVec2>,
    _ui_settings: Res<UiSettings>,
) {
    let mut do_thing = false;
    for _ in render_world_event.iter() {
        do_thing = true;
    }

    // lol
    if do_thing {
        let tileset = ui_state.tile_sets[ui_state.picked_tileset].0.clone();

        // tileset
        let bad_tile = asset_server.load("fail.png");
        let white_tile = asset_server.load("white.png");

        let world = &world.as_ref().as_ref().unwrap().world;
        let world_size = IVec2::new(world.len() as i32, world[0].len() as i32);
        if world_size != *current_size {
            *current_size = world_size;

            // prepare tile rendering
            let mut tile_entities = Vec::new();
            for x in 0..world_size.x as usize {
                tile_entities.push(Vec::new());
                for y in 0..world_size.y as usize {
                    let pos = Vec2::new(x as f32, y as f32);
                    let mut texture = Default::default();
                    let mut transform = Transform::from_translation(
                        ((pos + 0.5) / world_size.y as f32 - 0.5).extend(-0.5),
                    );
                    let mut color = Color::WHITE;

                    if let Some(pattern_index) = world[x][y].collapse() {
                        let tile_index = tileset.get_render_tile(pattern_index);
                        match &ui_state.tile_render_assets[tile_index] {
                            Bleh::Image {
                                transform: new_transform,
                                bevy_handle,
                                ..
                            } => {
                                transform.rotation = new_transform.rotation;
                                transform.scale = new_transform.scale;
                                texture = bevy_handle.clone();
                            }
                            Bleh::Color(new_color) => {
                                color = *new_color;
                                texture = white_tile.clone();
                            }
                        }
                    }
                    if world[x][y].count_bits() == 0 {
                        texture = bad_tile.clone();
                    }

                    tile_entities[x].push(
                        commands
                            .spawn((
                                SpriteBundle {
                                    transform,
                                    sprite: Sprite {
                                        custom_size: Some(Vec2::splat(1.0 / world_size.y as f32)),
                                        color,
                                        ..default()
                                    },
                                    texture,
                                    ..default()
                                },
                                TileSprite,
                            ))
                            .id(),
                    );
                }
            }
            for tile_entitys in ui_state.tile_entities.iter() {
                for tile_entity in tile_entitys.iter() {
                    commands.entity(*tile_entity).despawn();
                }
            }
            ui_state.tile_entities = tile_entities;
        } else {
            for x in 0..current_size.x as usize {
                for y in 0..current_size.y as usize {
                    let (mut transform, mut image_handle, mut sprite) = tile_entity_query
                        .get_mut(ui_state.tile_entities[x][y])
                        .unwrap();

                    if let Some(pattern_index) = world[x][y].collapse() {
                        let tile_index = tileset.get_render_tile(pattern_index);
                        match &ui_state.tile_render_assets[tile_index] {
                            Bleh::Image {
                                transform: new_transform,
                                bevy_handle,
                                ..
                            } => {
                                transform.rotation = new_transform.rotation;
                                transform.scale = new_transform.scale;
                                *image_handle = bevy_handle.clone();
                                sprite.color = Color::WHITE;
                            }
                            Bleh::Color(new_color) => {
                                *image_handle = white_tile.clone();
                                sprite.color = *new_color;
                            }
                        }
                    } else if world[x][y].count_bits() == 0 {
                        *image_handle = bad_tile.clone();
                        sprite.color = Color::WHITE;
                    } else {
                        *image_handle = Default::default();
                        sprite.color = Color::WHITE;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename = "samples")]
struct Samples {
    overlapping: Vec<Overlapping>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Overlapping {
    name: String,
    #[serde(rename = "N")]
    n: usize,
    #[serde(default)]
    symmetry: usize,
}
