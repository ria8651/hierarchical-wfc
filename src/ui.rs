use crate::{
    basic_tileset::BasicTileset, carcassonne_tileset::CarcassonneTileset,
    graph_grid::GridGraphSettings, tileset::TileSet, wfc::GraphWfc,
};
use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::{EguiContexts, EguiPlugin},
    egui::{self, DragValue, TextureId},
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
    Carcassonne(GridGraphSettings),
    BasicTileset(GridGraphSettings),
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

    egui::Window::new("Hello")
        .resizable(true)
        .scroll2([false, true])
        .show(contexts.ctx_mut(), |ui| {
            ui_for_value(ui_state.as_mut(), ui, &type_registry.read());

            if ui.button("Generate").clicked() {
                let settings = match &ui_state.picked_tileset {
                    TileSetUi::BasicTileset(settings) => settings,
                    TileSetUi::Carcassonne(settings) => settings,
                };

                let create_graph_span = info_span!("wfc_create_graph").entered();
                let mut graph = tileset.create_graph(settings);
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
                GraphWfc::collapse(&mut graph, &constraints, &ui_state.weights, &mut rng);
                collapse_span.exit();

                // for y in (0..settings.height as usize).rev() {
                //     for x in 0..settings.width as usize {
                //         print!("[{:?}]", graph.tiles[x * settings.height as usize + y]);
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
                    tile_handles.push(asset_server.load(tile));
                }

                // result
                for i in 0..result.tiles.len() {
                    let mut tile_index = result.tiles[i] as usize;
                    let mut tile_rotation = 0;
                    if tileset.tile_count() > 100 {
                        tile_rotation = tile_index / (tileset.tile_count() / 4);
                        tile_index = tile_index % (tileset.tile_count() / 4);
                    }
                    let pos = Vec2::new((i / settings.height) as f32, (i % settings.height) as f32);
                    commands.spawn((
                        SpriteBundle {
                            texture: tile_handles[tile_index].clone(),
                            transform: Transform::from_translation(
                                ((pos + 0.5) / settings.width as f32 - 0.5).extend(0.0),
                            )
                            .with_rotation(Quat::from_rotation_z(
                                -std::f32::consts::PI * tile_rotation as f32 / 2.0,
                            )),
                            sprite: Sprite {
                                custom_size: Some(Vec2::splat(1.0 / settings.width as f32)),
                                ..default()
                            },
                            ..default()
                        },
                        TileSprite,
                    ));
                }
                render_span.exit();
            }

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
}

impl Default for TileSetUi {
    fn default() -> Self {
        Self::Carcassonne(GridGraphSettings::default())
    }
}
