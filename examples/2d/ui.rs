use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::{EguiContexts, EguiPlugin},
    egui::{
        self, panel::Side, CollapsingHeader, Color32, DragValue, Frame, Id, ScrollArea, SidePanel,
        TextureId,
    },
    reflect_inspector::ui_for_value,
    DefaultInspectorConfigPlugin,
};
use grid_wfc::{
    basic_tileset::BasicTileset, carcassonne_tileset::CarcassonneTileset,
    graph_grid::GridGraphSettings, mxgmn_tileset::MxgmnTileset,
};
use hierarchical_wfc::TileSet;
use std::sync::Arc;

use crate::world::{GenerateEvent, MaybeWorld};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_plugins(DefaultInspectorConfigPlugin)
            .add_event::<RenderUpdateEvent>()
            .init_resource::<UiState>()
            .register_type::<UiState>()
            .register_type::<GridGraphSettings>()
            .add_systems(Update, (ui, render_world).chain());
    }
}

#[derive(Resource, Reflect)]
struct UiState {
    seed: u64,
    random_seed: bool,
    grid_graph_settings: GridGraphSettings,
    deterministic: bool,
    multithreaded: bool,
    chunk_size: usize,
    overlap: usize,
    #[reflect(ignore)]
    picked_tileset: usize,
    #[reflect(ignore)]
    tile_sets: Vec<(Arc<dyn TileSet>, String)>,
    #[reflect(ignore)]
    weights: Vec<f32>,
    #[reflect(ignore)]
    image_handles: Vec<(TextureId, Handle<Image>)>,
    #[reflect(ignore)]
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

        Self {
            seed: 0,
            random_seed: true,
            grid_graph_settings: GridGraphSettings::default(),
            deterministic: true,
            multithreaded: true,
            chunk_size: 4,
            overlap: 1,
            picked_tileset: 0,
            tile_sets,
            weights: Vec::new(),
            image_handles: Vec::new(),
            tile_entities: Vec::new(),
        }
    }
}

#[derive(Component)]
struct TileSprite;

fn ui(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    type_registry: Res<AppTypeRegistry>,
    asset_server: Res<AssetServer>,
    mut generate_events: EventWriter<GenerateEvent>,
) {
    let mut tileset = ui_state.tile_sets[ui_state.picked_tileset].0.clone();

    if ui_state.weights.len() != tileset.tile_count() {
        ui_state.weights = tileset.get_weights().as_ref().clone();

        for handle in ui_state.image_handles.drain(..) {
            contexts.remove_image(&handle.1);
        }
        for path in tileset.get_tile_paths() {
            let bevy_handle = asset_server.load(path.0);
            let handle = contexts.add_image(bevy_handle.clone_weak());
            ui_state.image_handles.push((handle, bevy_handle));
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

                        ui_for_value(ui_state.as_mut(), ui, &type_registry.read());

                        let seed = if !ui_state.random_seed {
                            ui_state.seed
                        } else {
                            rand::random()
                        };

                        dyn_clone::arc_make_mut(&mut tileset).set_weights(ui_state.weights.clone());
                        if ui.button("Generate Single").clicked() {
                            generate_events.send(GenerateEvent::Single {
                                tileset,
                                settings: ui_state.grid_graph_settings.clone(),
                                seed,
                            });
                        } else if ui.button("Generate Chunked").clicked() {
                            generate_events.send(GenerateEvent::Chunked {
                                tileset,
                                settings: ui_state.grid_graph_settings.clone(),
                                multithreaded: ui_state.multithreaded,
                                deterministic: ui_state.deterministic,
                                seed,
                                chunk_size: ui_state.chunk_size,
                                overlap: ui_state.overlap,
                            });
                        }
                    });

                CollapsingHeader::new("Tileset Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("some_unique_id").show(ui, |ui| {
                            for i in 0..ui_state.weights.len() {
                                ui.vertical_centered(|ui| {
                                    ui.image(ui_state.image_handles[i].0, [64.0, 64.0]);
                                    ui.add(DragValue::new(&mut ui_state.weights[i]));
                                });

                                if i % 4 == 3 {
                                    ui.end_row();
                                }
                            }
                        });
                    });
            });
        });
}

#[derive(Event)]
pub struct RenderUpdateEvent;

/// disgusting
fn render_world(
    mut commands: Commands,
    mut ui_state: ResMut<UiState>,
    asset_server: Res<AssetServer>,
    mut tile_entity_query: Query<(&mut Transform, &mut Handle<Image>), With<TileSprite>>,
    world: Res<MaybeWorld>,
    mut render_world_event: EventReader<RenderUpdateEvent>,
    mut current_size: Local<IVec2>,
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
        let mut tile_handles: Vec<(Handle<Image>, Transform)> = Vec::new();
        for tile in tileset.get_tile_paths() {
            tile_handles.push((asset_server.load(tile.0), tile.1));
        }

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

                    if let Some(tile_index) = world[x][y].collapse() {
                        texture = tile_handles[tile_index].0.clone();
                        transform.rotation = tile_handles[tile_index].1.rotation;
                        transform.scale = tile_handles[tile_index].1.scale;
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
                    let (mut transform, mut sprite) = tile_entity_query
                        .get_mut(ui_state.tile_entities[x][y])
                        .unwrap();

                    if let Some(tile_index) = world[x][y].collapse() {
                        let new_transform = &tile_handles[tile_index].1;
                        transform.rotation = new_transform.rotation;
                        transform.scale = new_transform.scale;
                        *sprite = tile_handles[tile_index].0.clone();
                    } else if world[x][y].count_bits() == 0 {
                        *sprite = bad_tile.clone();
                    } else {
                        *sprite = Default::default();
                    }
                }
            }
        }
    }
}
