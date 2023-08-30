use crate::{
    basic_tileset::BasicTileset, carcassonne_tileset::CarcassonneTileset,
    graph_grid::GridGraphSettings,
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
use crossbeam::queue::SegQueue;
use hierarchical_wfc::{CpuExecuter, Executer, Graph, Peasant, TileSet, WaveFunction};
use std::sync::Arc;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_plugins(DefaultInspectorConfigPlugin)
            .init_resource::<UiState>()
            .register_type::<UiState>()
            .register_type::<TileSetUi>()
            .register_type::<GridGraphSettings>()
            .add_systems(Update, (ui, render_grid_graph).chain());
    }
}

#[derive(Resource, Reflect)]
struct UiState {
    seed: u64,
    random_seed: bool,
    picked_tileset: TileSetUi,
    timeout: Option<f64>,
    #[reflect(ignore)]
    guild: Guild,
    #[reflect(ignore)]
    weights: Vec<u32>,
    #[reflect(ignore)]
    image_handles: Vec<(TextureId, Handle<Image>)>,
    #[reflect(ignore)]
    graph: Option<Graph<WaveFunction>>,
    #[reflect(ignore)]
    graph_dirty: bool,
    #[reflect(ignore)]
    render_dirty: RenderState,
    #[reflect(ignore)]
    tile_entities: Vec<Entity>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            seed: 0,
            random_seed: true,
            picked_tileset: TileSetUi::default(),
            timeout: Some(0.05),
            guild: Guild::default(),
            weights: Vec::new(),
            image_handles: Vec::new(),
            graph: None,
            graph_dirty: false,
            render_dirty: Default::default(),
            tile_entities: Vec::new(),
        }
    }
}

struct Guild {
    cpu_executer: CpuExecuter,
    output: Arc<SegQueue<Peasant>>,
}

impl Default for Guild {
    fn default() -> Self {
        let output = Arc::new(SegQueue::new());
        let cpu_executer = CpuExecuter::new(output.clone());

        Self {
            cpu_executer,
            output,
        }
    }
}

#[derive(Default, PartialEq, Eq)]
enum RenderState {
    #[default]
    Init,
    Dirty,
    Done,
}

#[derive(Reflect)]
enum TileSetUi {
    Carcassonne(GridGraphSettings),
    BasicTileset(GridGraphSettings),
}

#[derive(Component)]
struct TileSprite;

fn ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    type_registry: Res<AppTypeRegistry>,
    asset_server: Res<AssetServer>,
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

                        if ui.button("Generate").clicked() {
                            let settings = match &ui_state.picked_tileset {
                                TileSetUi::BasicTileset(settings) => settings,
                                TileSetUi::Carcassonne(settings) => settings,
                            };
                            let graph = tileset.create_graph(settings);

                            // prepare tile rendering
                            let mut tile_entities = Vec::new();
                            for i in 0..graph.tiles.len() {
                                let pos = Vec2::new(
                                    (i / settings.height) as f32,
                                    (i % settings.height) as f32,
                                );
                                tile_entities.push(
                                    commands
                                        .spawn((
                                            SpriteBundle {
                                                transform: Transform::from_translation(
                                                    ((pos + 0.5) / settings.width as f32 - 0.5)
                                                        .extend(-0.5),
                                                ),
                                                sprite: Sprite {
                                                    custom_size: Some(Vec2::splat(
                                                        1.0 / settings.width as f32,
                                                    )),
                                                    ..default()
                                                },
                                                ..default()
                                            },
                                            TileSprite,
                                        ))
                                        .id(),
                                );
                            }
                            for tile_entity in ui_state.tile_entities.iter() {
                                commands.entity(*tile_entity).despawn();
                            }

                            ui_state.tile_entities = tile_entities;
                            ui_state.graph = Some(graph);

                            ui_state.graph_dirty = true;
                            ui_state.render_dirty = RenderState::Init;

                            // start generation task
                            let seed = if !ui_state.random_seed {
                                ui_state.seed
                            } else {
                                rand::random()
                            };
                            let constraints = Arc::new(tileset.get_constraints());
                            let graph = ui_state.graph.as_ref().unwrap().clone();
                            let weights = ui_state.weights.clone();
                            let peasant = Peasant {
                                graph,
                                constraints,
                                weights,
                                seed,
                            };
                            ui_state.guild.cpu_executer.queue_peasant(peasant).unwrap();
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

    if let Some(peasant) = ui_state.guild.output.pop() {
        ui_state.graph = Some(peasant.graph);
        ui_state.graph_dirty = true;
        ui_state.render_dirty = RenderState::Dirty;
    }
}

fn render_grid_graph(
    mut ui_state: ResMut<UiState>,
    asset_server: Res<AssetServer>,
    mut tile_entity_query: Query<(&mut Transform, &mut Handle<Image>), With<TileSprite>>,
) {
    let render_span = info_span!("wfc_render").entered();
    if let Some(graph) = &ui_state.graph {
        if ui_state.render_dirty == RenderState::Dirty {
            let tileset = match &ui_state.picked_tileset {
                TileSetUi::BasicTileset(_) => Box::new(BasicTileset::default())
                    as Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
                TileSetUi::Carcassonne(_) => Box::new(CarcassonneTileset::default())
                    as Box<dyn TileSet<GraphSettings = GridGraphSettings>>,
            };

            // tileset
            let mut tile_handles: Vec<Handle<Image>> = Vec::new();
            for tile in tileset.get_tile_paths() {
                tile_handles.push(asset_server.load(tile));
            }

            // result
            for i in 0..graph.tiles.len() {
                if let Some(mut tile_index) = graph.tiles[i].collapse() {
                    let mut tile_rotation = 0;
                    if tileset.tile_count() > 100 {
                        tile_rotation = tile_index / (tileset.tile_count() / 4);
                        tile_index = tile_index % (tileset.tile_count() / 4);
                    }

                    let (mut transform, mut sprite) = tile_entity_query
                        .get_mut(ui_state.tile_entities[i])
                        .unwrap();

                    transform.rotation =
                        Quat::from_rotation_z(-std::f32::consts::PI * tile_rotation as f32 / 2.0);
                    *sprite = tile_handles[tile_index].clone();
                }
            }

            ui_state.render_dirty = RenderState::Done;
        }
        if ui_state.render_dirty == RenderState::Init {
            ui_state.render_dirty = RenderState::Dirty;
        }
    }
    render_span.exit();
}

impl Default for TileSetUi {
    fn default() -> Self {
        Self::Carcassonne(GridGraphSettings::default())
    }
}
