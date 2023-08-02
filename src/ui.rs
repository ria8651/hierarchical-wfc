use crate::{
    basic_tileset::BasicTileset, carcassonne_tileset::CarcassonneTileset,
    graph_grid::GridGraphSettings, tileset::TileSet, wfc::GraphWfc,
};
use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::{EguiContexts, EguiPlugin},
    egui,
    reflect_inspector::ui_for_value,
    DefaultInspectorConfigPlugin,
};

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
    picked_tileset: TileSetUi,
}

#[derive(Reflect)]
enum TileSetUi {
    Carcassonne(GridGraphSettings),
    BasicTileset(GridGraphSettings),
}

fn ui(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    type_registry: Res<AppTypeRegistry>,
    _asset_server: Res<AssetServer>,
) {
    egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
        ui_for_value(ui_state.as_mut(), ui, &type_registry.read());

        if ui.button("Generate").clicked() {
            let graph = match &ui_state.picked_tileset {
                TileSetUi::BasicTileset(settings) => {
                    let tileset = BasicTileset::new();
                    let mut graph_wfc = GraphWfc::new();
                    let mut graph = tileset.create_graph(&settings);
                    graph_wfc.collapse(&mut graph, &tileset, 0);
                    graph
                }
                TileSetUi::Carcassonne(settings) => {
                    let tileset = CarcassonneTileset::new();
                    let mut graph_wfc = GraphWfc::new();
                    let mut graph = tileset.create_graph(&settings);
                    graph_wfc.collapse(&mut graph, &tileset, 0);
                    graph
                }
            };
            let result = graph.validate();
            println!("{:?}", result);

            // // tileset
            // let mut tile_handles: Vec<Handle<Image>> = Vec::new();
            // for tile in tileset.get_tile_paths() {
            //     tile_handles.push(asset_server.load(tile));
            // }

            // // result
            // for x in 0..tiles.len() {
            //     for y in 0..tiles[0].len() {
            //         let mut tile_index = tiles[x][y] as usize;
            //         let mut tile_rotation = 0;
            //         if tileset.tile_count() > 100 {
            //             tile_rotation = tile_index / (tileset.tile_count() / 4);
            //             tile_index = tile_index % (tileset.tile_count() / 4);
            //         }
            //         let pos = Vec2::new(x as f32, y as f32);
            //         commands.spawn((
            //             SpriteBundle {
            //                 texture: tile_handles[tile_index].clone(),
            //                 transform: Transform::from_translation(
            //                     ((pos + 0.5) / tiles.len() as f32 - 0.5).extend(0.0),
            //                 )
            //                 .with_rotation(Quat::from_rotation_z(
            //                     -std::f32::consts::PI * tile_rotation as f32 / 2.0,
            //                 )),
            //                 sprite: Sprite {
            //                     custom_size: Some(Vec2::splat(1.0 / tiles.len() as f32)),
            //                     ..default()
            //                 },
            //                 ..default()
            //             },
            //             TileSprite,
            //         ));
            //     }
            // }
        }
    });
}

impl Default for TileSetUi {
    fn default() -> Self {
        Self::Carcassonne(GridGraphSettings::default())
    }
}
