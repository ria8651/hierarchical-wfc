use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::{EguiContexts, EguiPlugin},
    egui::{
        self, panel::Side, CollapsingHeader, Color32, DragValue, Frame, Id, SidePanel, TextureId,
    },
    reflect_inspector::ui_for_value,
    DefaultInspectorConfigPlugin,
};
use hierarchical_wfc::{
    graphs::regular_grid_2d,
    wfc::{Superposition, TileSet, WaveFunctionCollapse},
};
use rand::{rngs::StdRng, SeedableRng};

use crate::{basic_tileset::BasicTileset, carcassonne_tileset::CarcassonneTileset};
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_plugins(DefaultInspectorConfigPlugin)
            .init_resource::<UiState>()
            .register_type::<UiState>()
            .register_type::<TileSetUi>()
            .register_type::<regular_grid_2d::GraphSettings>()
            .add_systems(Update, ui);
    }
}

#[derive(Resource, Reflect, Default)]
struct UiState {
    seed: u64,
    random_seed: bool,
    picked_tileset: TileSetUi,
    #[reflect(ignore)]
    weights: Vec<u32>,
    #[reflect(ignore)]
    image_handles: Vec<(TextureId, Handle<Image>)>,
}

#[derive(Reflect)]
enum TileSetUi {
    Carcassonne(regular_grid_2d::GraphSettings),
    BasicTileset(regular_grid_2d::GraphSettings),
}

#[derive(Component)]
struct TileSprite;

fn ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    mut tile_sprites: Query<Entity, With<TileSprite>>,
    type_registry: Res<AppTypeRegistry>,
    asset_server: Res<AssetServer>,
) {
    let tileset = match &ui_state.picked_tileset {
        TileSetUi::BasicTileset(_) => Box::new(BasicTileset) as Box<dyn TileSet>,
        TileSetUi::Carcassonne(_) => Box::new(CarcassonneTileset) as Box<dyn TileSet>,
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
            CollapsingHeader::new("WFC Settings")
                .default_open(true)
                .show(ui, |ui| {
                    ui_for_value(ui_state.as_mut(), ui, &type_registry.read());

                    if ui.button("Generate").clicked() {
                        let settings = match &ui_state.picked_tileset {
                            TileSetUi::BasicTileset(settings) => settings,
                            TileSetUi::Carcassonne(settings) => settings,
                        };

                        let create_graph_span = info_span!("wfc_create_graph").entered();
                        let mut graph = regular_grid_2d::create_grid_graph(
                            settings,
                            Superposition::filled(tileset.tile_count()),
                        );
                        create_graph_span.exit();

                        let setup_constraints_span = info_span!("wfc_setup_constraints").entered();
                        let constraints = tileset.get_constraints();
                        let mut rng = if !ui_state.random_seed {
                            StdRng::seed_from_u64(ui_state.seed)
                        } else {
                            StdRng::from_entropy()
                        };
                        setup_constraints_span.exit();

                        let collapse_span = info_span!("wfc_collapse").entered();
                        WaveFunctionCollapse::collapse(
                            &mut graph,
                            &constraints,
                            &ui_state.weights,
                            &mut rng,
                        );
                        collapse_span.exit();

                        // for y in (0..settings.height).rev() {
                        //     for x in 0..settings.width {
                        //         print!("[{:?}]", graph.nodes[x * settings.height + y]);
                        //     }
                        //     println!();
                        // }

                        let render_span = info_span!("wfc_render").entered();
                        let result = match graph.validate() {
                            Ok(graph) => graph,
                            Err(e) => {
                                println!("{}", e);
                                return;
                            }
                        };

                        // cleanup
                        for entity in tile_sprites.iter_mut() {
                            commands.entity(entity).despawn();
                        }

                        // tileset
                        let mut tile_handles: Vec<Handle<Image>> = Vec::new();
                        for tile in tileset.get_tile_paths() {
                            dbg!(&tile);
                            tile_handles.push(asset_server.load(tile));
                        }

                        // result
                        for i in 0..result.nodes.len() {
                            let mut tile_index = result.nodes[i] as usize;
                            let mut tile_rotation = 0;
                            if tileset.tile_count() > 100 {
                                tile_rotation = tile_index / (tileset.tile_count() / 4);
                                tile_index %= tileset.tile_count() / 4;
                            }
                            let pos = Vec2::new(
                                (i / settings.height) as f32,
                                (i % settings.height) as f32,
                            );
                            let transform = Transform::from_translation(
                                ((pos + 0.5) / settings.width as f32 - 0.5).extend(0.0),
                            )
                            .with_rotation(Quat::from_rotation_z(
                                -std::f32::consts::PI * tile_rotation as f32 / 2.0,
                            ));
                            dbg!(&tile_index, &transform);
                            commands.spawn((
                                SpriteBundle {
                                    texture: tile_handles[tile_index].clone(),
                                    transform,
                                    sprite: Sprite {
                                        custom_size: Some(Vec2::ONE), //Some(Vec2::splat(1.0 / settings.width as f32)),
                                        ..default()
                                    },
                                    ..default()
                                },
                                TileSprite,
                            ));
                        }
                        render_span.exit();
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

impl Default for TileSetUi {
    fn default() -> Self {
        Self::Carcassonne(regular_grid_2d::GraphSettings::default())
    }
}
