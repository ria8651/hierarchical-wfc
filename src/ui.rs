use crate::{
    basic_tileset::BasicTileset,
    carcassonne_tileset::CarcassonneTileset,
    graph::{Cell, Graph},
    graph_grid::GridGraphSettings,
    hierarchical_tileset::HierarchicalTileset,
    tileset::TileSet,
    wfc::GraphWfc,
};
use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::{EguiContexts, EguiPlugin},
    egui::{
        self, panel::Side, CollapsingHeader, Color32, DragValue, Frame, Id, SidePanel, TextureId,
    },
    reflect_inspector::ui_for_value,
    DefaultInspectorConfigPlugin,
};
use rand::{rngs::StdRng, SeedableRng};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_plugins(DefaultInspectorConfigPlugin)
            .init_resource::<UiState>()
            .register_type::<UiState>()
            .register_type::<TileSetUi>()
            .register_type::<GridGraphSettings>()
            .add_systems(Update, (ui, propagate, render_grid_graph).chain());
    }
}

#[derive(Resource, Reflect, Default)]
struct UiState {
    seed: u64,
    random_seed: bool,
    picked_tileset: TileSetUi,
    timeout: Option<f64>,
    #[reflect(ignore)]
    weights: Vec<u32>,
    #[reflect(ignore)]
    image_handles: Vec<(TextureId, Handle<Image>)>,
    #[reflect(ignore)]
    graph: Option<Graph<Cell>>,
    #[reflect(ignore)]
    graph_dirty: bool,
    #[reflect(ignore)]
    render_dirty: RenderState,
    #[reflect(ignore)]
    tile_entities: Vec<Entity>,
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
    HierarchicalTileset(GridGraphSettings),
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
    let tileset = get_tileset(&ui_state.picked_tileset);

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
            CollapsingHeader::new("WFC Settings")
                .default_open(true)
                .show(ui, |ui| {
                    ui_for_value(ui_state.as_mut(), ui, &type_registry.read());

                    if ui.button("Generate").clicked() {
                        let create_graph_span = info_span!("wfc_create_graph").entered();
                        let settings = match &ui_state.picked_tileset {
                            TileSetUi::BasicTileset(settings) => settings,
                            TileSetUi::Carcassonne(settings) => settings,
                            TileSetUi::HierarchicalTileset(settings) => settings,
                        };
                        let graph = tileset.create_graph(settings);
                        create_graph_span.exit();

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
                                                    .extend(0.0),
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
}

fn get_tileset(picked_tileset: &TileSetUi) -> Box<dyn TileSet<GraphSettings = GridGraphSettings>> {
    match picked_tileset {
        TileSetUi::BasicTileset(_) => Box::new(BasicTileset::default()),
        TileSetUi::Carcassonne(_) => Box::new(CarcassonneTileset::default()),
        TileSetUi::HierarchicalTileset(_) => Box::new(HierarchicalTileset::default()),
    }
}

fn propagate(mut ui_state: ResMut<UiState>) {
    if ui_state.graph_dirty {
        let tileset = get_tileset(&ui_state.picked_tileset);
        let setup_constraints_span = info_span!("wfc_setup_constraints").entered();
        let constraints = tileset.get_constraints();
        let mut rng = if !ui_state.random_seed {
            StdRng::seed_from_u64(ui_state.seed)
        } else {
            StdRng::from_entropy()
        };
        setup_constraints_span.exit();

        let collapse_span = info_span!("wfc_collapse").entered();
        let mut graph = ui_state.graph.as_ref().unwrap().clone();
        ui_state.graph_dirty = GraphWfc::collapse(
            &mut graph,
            &constraints,
            &ui_state.weights,
            &mut rng,
            ui_state.timeout,
        );
        ui_state.graph = Some(graph);
        collapse_span.exit();

        if ui_state.render_dirty != RenderState::Init {
            ui_state.render_dirty = RenderState::Dirty;
        }
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
            let tileset = get_tileset(&ui_state.picked_tileset);

            // tileset
            let mut tile_handles: Vec<Handle<Image>> = Vec::new();
            for tile in tileset.get_tile_paths() {
                tile_handles.push(asset_server.load(tile));
            }

            // result
            println!("graph.tiles.len(): {}", graph.tiles.len());
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
