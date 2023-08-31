use crate::{
    basic_tileset::BasicTileset,
    carcassonne_tileset::CarcassonneTileset,
    graph_grid::GridGraphSettings,
    world::{GenerateEvent, World},
};
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
use hierarchical_wfc::TileSet;
use std::sync::Arc;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_plugins(DefaultInspectorConfigPlugin)
            .add_event::<RenderUpdateEvent>()
            .init_resource::<UiState>()
            .register_type::<UiState>()
            .register_type::<TileSetUi>()
            .register_type::<GridGraphSettings>()
            .add_systems(Update, (ui, render_world).chain());
    }
}

#[derive(Resource, Reflect)]
struct UiState {
    seed: u64,
    random_seed: bool,
    picked_tileset: TileSetUi,
    timeout: Option<f64>,
    chunk_size: usize,
    #[reflect(ignore)]
    weights: Vec<u32>,
    #[reflect(ignore)]
    image_handles: Vec<(TextureId, Handle<Image>)>,
    #[reflect(ignore)]
    tile_entities: Vec<Vec<Entity>>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            seed: 0,
            random_seed: true,
            picked_tileset: TileSetUi::default(),
            timeout: Some(0.05),
            chunk_size: 4,
            weights: Vec::new(),
            image_handles: Vec::new(),
            tile_entities: Vec::new(),
        }
    }
}

#[derive(Reflect)]
enum TileSetUi {
    Carcassonne(GridGraphSettings),
    BasicTileset(GridGraphSettings),
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
    let tileset = match &ui_state.picked_tileset {
        TileSetUi::BasicTileset(_) => {
            Box::new(BasicTileset::default()) as Box<dyn TileSet<GraphSettings = GridGraphSettings>>
        }
        TileSetUi::Carcassonne(_) => Box::new(CarcassonneTileset::default())
            as Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
    };

    if ui_state.weights.len() != tileset.tile_count() {
        ui_state.weights = tileset.get_weights();

        for handle in ui_state.image_handles.drain(..) {
            contexts.remove_image(&handle.1);
        }
        for path in tileset.get_tile_paths() {
            let bevy_handle = asset_server.load(path);
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
                        ui_for_value(ui_state.as_mut(), ui, &type_registry.read());

                        if ui.button("Generate Single").clicked() {
                            let settings = match &ui_state.picked_tileset {
                                TileSetUi::BasicTileset(settings) => settings,
                                TileSetUi::Carcassonne(settings) => settings,
                            };
                            let seed = if !ui_state.random_seed {
                                ui_state.seed
                            } else {
                                rand::random()
                            };

                            generate_events.send(GenerateEvent::Single {
                                tileset: tileset.clone(),
                                settings: settings.clone(),
                                weights: Arc::new(ui_state.weights.clone()),
                                seed,
                            });
                        }
                        if ui.button("Generate Chunked").clicked() {
                            let settings = match &ui_state.picked_tileset {
                                TileSetUi::BasicTileset(settings) => settings,
                                TileSetUi::Carcassonne(settings) => settings,
                            };
                            let seed = if !ui_state.random_seed {
                                ui_state.seed
                            } else {
                                rand::random()
                            };

                            generate_events.send(GenerateEvent::Chunked {
                                tileset: tileset.clone(),
                                settings: settings.clone(),
                                weights: Arc::new(ui_state.weights.clone()),
                                seed,
                                chunk_size: ui_state.chunk_size,
                            });
                        }
                    });

                CollapsingHeader::new("Tileset Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("some_unique_id").show(ui, |ui| {
                            for i in 0..ui_state.weights.len() {
                                ui.vertical_centered(|ui| {
                                    ui.image(ui_state.image_handles[i % 30].0, [64.0, 64.0]);
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

    // if let Some(peasant) = ui_state.guild.output.pop() {
    //     ui_state.graph = Some(peasant.graph);
    //     ui_state.graph_dirty = true;
    //     ui_state.render_dirty = RenderState::Dirty;
    // }
}

#[derive(Event)]
pub struct RenderUpdateEvent;

/// disgusting
fn render_world(
    mut commands: Commands,
    mut ui_state: ResMut<UiState>,
    asset_server: Res<AssetServer>,
    mut tile_entity_query: Query<(&mut Transform, &mut Handle<Image>), With<TileSprite>>,
    world: Res<World>,
    mut render_world_event: EventReader<RenderUpdateEvent>,
    mut current_size: Local<IVec2>,
) {
    for _ in render_world_event.iter() {
        let tileset = match &ui_state.picked_tileset {
            TileSetUi::BasicTileset(_) => Box::new(BasicTileset::default())
                as Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
            TileSetUi::Carcassonne(_) => Box::new(CarcassonneTileset::default())
                as Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
        };

        // tileset
        let bad_tile = asset_server.load("fail.png");
        let mut tile_handles: Vec<Handle<Image>> = Vec::new();
        for tile in tileset.get_tile_paths() {
            tile_handles.push(asset_server.load(tile));
        }

        let world_size = IVec2::new(world.world.len() as i32, world.world[0].len() as i32);
        if world_size != *current_size {
            *current_size = world_size;

            // prepare tile rendering
            let mut tile_entities = Vec::new();
            for x in 0..world_size.x as usize {
                tile_entities.push(Vec::new());
                for y in 0..world_size.y as usize {
                    let pos = Vec2::new(x as f32, y as f32);
                    let mut transform = Transform::from_translation(
                        ((pos + 0.5) / world_size.y as f32 - 0.5).extend(-0.5),
                    );
                    let mut texture = Default::default();

                    if let Some(mut tile_index) = world.world[x][y].collapse() {
                        let mut tile_rotation = 0;
                        if tileset.tile_count() > 100 {
                            tile_rotation = tile_index / (tileset.tile_count() / 4);
                            tile_index = tile_index % (tileset.tile_count() / 4);
                        }

                        transform = transform.with_rotation(Quat::from_rotation_z(
                            -std::f32::consts::PI * tile_rotation as f32 / 2.0,
                        ));
                        texture = tile_handles[tile_index].clone();
                    }
                    if world.world[x][y].count_bits() == 0 {
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

                    if let Some(mut tile_index) = world.world[x][y].collapse() {
                        let mut tile_rotation = 0;
                        if tileset.tile_count() > 100 {
                            tile_rotation = tile_index / (tileset.tile_count() / 4);
                            tile_index = tile_index % (tileset.tile_count() / 4);
                        }

                        transform.rotation = Quat::from_rotation_z(
                            -std::f32::consts::PI * tile_rotation as f32 / 2.0,
                        );
                        *sprite = tile_handles[tile_index].clone();
                    } else {
                        if world.world[x][y].count_bits() == 0 {
                            *sprite = bad_tile.clone();
                        } else {
                            *sprite = Default::default();
                        }
                    }
                }
            }
        }
    }
}

impl Default for TileSetUi {
    fn default() -> Self {
        Self::Carcassonne(GridGraphSettings::default())
    }
}
