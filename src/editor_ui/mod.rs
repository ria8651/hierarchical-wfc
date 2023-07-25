use std::any::TypeId;
use std::ops::{Add, Div, Mul};
use std::time::Duration;

use bevy::asset::{ChangeWatcher, HandleId, ReflectAsset};
use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::ecs::component::TableStorage;
use bevy::ecs::storage::Table;
use bevy::ecs::system::SystemState;

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::{
    AssetEvent, AssetPlugin, Event, EventReader, EventWriter, GlobalTransform, IVec3,
    IntoSystemConfigs, Local, MouseButton, PluginGroup, Resource,
};
use bevy::render::texture::ImageSampler;
use bevy::render::view::NoFrustumCulling;
use bevy::sprite::{
    Material2dPlugin, MaterialMesh2dBundle, SpriteSheetBundle, TextureAtlas, TextureAtlasSprite,
};
use bevy::{
    math::{UVec2, Vec2},
    prelude::*,
};

use bevy_inspector_egui::bevy_egui::{self, egui, EguiContext, EguiUserTextures};
use bevy_inspector_egui::bevy_inspector::hierarchy::{hierarchy_ui, SelectedEntities};
use bevy_inspector_egui::bevy_inspector::{
    self, ui_for_entities_shared_components, ui_for_entity_with_children,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use bevy::reflect::TypeRegistry;
use bevy::render::camera::{CameraRenderGraph, ScalingMode, Viewport};
use bevy::window::{CursorMoved, PresentMode, PrimaryWindow, Window, WindowPlugin};
use bevy_inspector_egui::bevy_egui::EguiSet;
use bevy_simple_tilemap::prelude::{SimpleTileMapPlugin, TileMapBundle};
use bevy_simple_tilemap::{Tile, TileMap};
use egui_dock::{DockArea, NodeIndex, Style, Tree};
use egui_gizmo::GizmoMode;

pub trait EditorTab: Sync + Send {
    fn update(&self, world: &mut World);
    fn is_game(&self) -> bool {
        false
    }
    fn ui(&mut self, ui: &mut egui_dock::egui::Ui);
    fn title(&self) -> String;
}

type Tab = Box<dyn EditorTab>;
#[derive(Resource)]
pub struct EditorState {
    tree: Tree<Tab>,
    pub viewport_rect: egui::Rect,
    pub selected_entities: SelectedEntities,
    pub selection: InspectorSelection,
    pub gizmo_mode: GizmoMode,
    pub tile_map_bundle: TileMapBundle,
    pub tile_size: u32,
    pub active_tile: Option<u32>,
}

impl EditorState {
    pub fn new() -> Self {
        let selected_entities = SelectedEntities::default();
        let selection = InspectorSelection::Entities;
        let mut viewport_rect = egui::Rect::NOTHING;
        let gizmo_mode = GizmoMode::Translate;
        let tile_size = 64;
        let active_tile = None;
        let tile_map_bundle = TileMapBundle::default();

        let mut tree: Tree<Tab> = Tree::new(vec![Box::new(EguiWindow::GameView {
            viewport_rect: viewport_rect,
        })]);
        let [game, _inspector] = tree.split_right(
            NodeIndex::root(),
            0.75,
            vec![Box::new(EguiWindow::Inspector {
                world: World::new(),
            })],
        );
        // let [game, _hierarchy] = tree.split_left(game, 0.2, vec![Box::new(EguiWindow::Hierarchy)]);
        // let [_game, _bottom] = tree.split_below(
        //     game,
        //     0.8,
        //     vec![
        //         Box::new(EguiWindow::Resources),
        //         Box::new(EguiWindow::Assets),
        //     ],
        // );

        Self {
            tree,
            selected_entities,
            selection,
            viewport_rect,
            gizmo_mode,
            tile_size,
            active_tile,
            tile_map_bundle,
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &mut egui::Context) {
        let mut tab_viewer = TabViewer {
            world,
            // viewport_rect: &mut self.viewport_rect,
            // selected_entities: &mut self.selected_entities,
            // selection: &mut self.selection,
            // gizmo_mode: self.gizmo_mode,
            // tile_map_bundle: &self.tile_map_bundle,
            // tile_size: &mut self.tile_size,
            // active_tile: &mut self.active_tile,
        };
        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

#[derive(Component)]
pub enum EguiWindow<'a> {
    GameView {
        viewport_rect: egui::Rect,
    },
    Hierarchy {
        world: &'a mut World,
        selected_entities: &'a mut SelectedEntities,
        selection: InspectorSelection,
    },
    Resources {
        selection: &'a mut InspectorSelection,
        type_registry: &'a mut TypeRegistry,
    },
    Assets {
        world: &'a mut World,
        selection: InspectorSelection,
        type_registry: &'a mut TypeRegistry,
    },
    Inspector {
        world: World,
        // viewport_rect: egui::Rect,
        // selected_entities: SelectedEntities,
        // selection: InspectorSelection,
        // gizmo_mode: GizmoMode,
        // tile_map_bundle: TileMapBundle,
        // tile_size: u32,
        // active_tile: Option<u32>,
    },
}

impl EditorTab for EguiWindow<'_> {
    fn update(&self, world: &mut World) {}
    fn title(&self) -> String {
        format!("self:?")
    }
    fn ui(&mut self, ui: &mut egui_dock::egui::Ui) {
        match self {
            EguiWindow::GameView { mut viewport_rect } => {
                viewport_rect = ui.clip_rect();

                // draw_gizmo(ui, self.world, self.selected_entities, self.gizmo_mode);
            }
            EguiWindow::Hierarchy {
                world,
                selected_entities,
                selection,
            } => {
                let selected = hierarchy_ui(world, ui, selected_entities);
                if selected {
                    *selection = InspectorSelection::Entities;
                }
            }
            EguiWindow::Resources {
                selection,
                type_registry,
            } => select_resource(ui, type_registry, selection),
            EguiWindow::Assets {
                world,
                selection,
                type_registry,
            } => select_asset(ui, type_registry, world, selection),
            EguiWindow::Inspector {
                world,
                // selected_entities,
                // selection,
                // viewport_rect,
                // gizmo_mode,
                // tile_map_bundle,
                // tile_size,
                // active_tile,
            } => {

            }
            // } => match *self.selection {
            //     InspectorSelection::Entities => match self.selected_entities.as_slice() {
            //         &[entity] => ui_for_entity_with_children(self.world, entity, ui),
            //         entities => ui_for_entities_shared_components(self.world, entities, ui),
            //     },
            //     InspectorSelection::Resource(type_id, ref name) => {
            //         ui.label(name);
            //         bevy_inspector::by_type_id::ui_for_resource(
            //             self.world,
            //             type_id,
            //             ui,
            //             name,
            //             &type_registry.read(),
            //         )
            //     }
            //     InspectorSelection::Asset(type_id, ref name, handle) => {
            //         ui.label(name);
            //         bevy_inspector::by_type_id::ui_for_asset(
            //             self.world,
            //             type_id,
            //             handle,
            //             ui,
            //             &type_registry.read(),
            //         );
            //     }
            // },
            // EguiWindow::TileMap => tilemap_ui(self.world, ui, self.tile_size, self.active_tile),
        }
    }
    fn is_game(&self) -> bool {
        matches!(&self, EguiWindow::GameView { viewport_rect })
    }
}

struct TabViewer<'a> {
    world: &'a mut World,
    // selected_entities: &'a mut SelectedEntities,
    // selection: &'a mut InspectorSelection,
    // viewport_rect: &'a mut egui::Rect,
    // gizmo_mode: GizmoMode,
    // tile_map_bundle: &'a TileMapBundle,
    // tile_size: &'a mut u32,
    // active_tile: &'a mut Option<u32>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        let type_registry = self.world.resource::<AppTypeRegistry>().0.clone();
        window.ui(ui);
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        window.title().into()
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        window.is_game()
    }
}

fn draw_gizmo(
    ui: &mut egui::Ui,
    world: &mut World,
    selected_entities: &SelectedEntities,
    gizmo_mode: GizmoMode,
) {
    // let (cam_transform, projection) = world
    //     .query_filtered::<(&GlobalTransform, &Projection), With<MainCamera>>()
    //     .single(world);
    // let view_matrix = Mat4::from(cam_transform.affine().inverse());
    // let projection_matrix = projection.get_projection_matrix();

    // if selected_entities.len() != 1 {
    //     return;
    // }

    // for selected in selected_entities.iter() {
    //     let Some(transform) = world.get::<Transform>(selected) else {
    //         continue;
    //     };
    //     let model_matrix = transform.compute_matrix();

    //     let Some(result) = Gizmo::new(selected)
    //         .model_matrix(model_matrix.to_cols_array_2d())
    //         .view_matrix(view_matrix.to_cols_array_2d())
    //         .projection_matrix(projection_matrix.to_cols_array_2d())
    //         .orientation(GizmoOrientation::Local)
    //         .mode(gizmo_mode)
    //         .interact(ui)
    //     else {
    //         continue;
    //     };

    //     let mut transform = world.get_mut::<Transform>(selected).unwrap();
    //     *transform = Transform {
    //         translation: Vec3::from(<[f32; 3]>::from(result.translation)),
    //         rotation: Quat::from_array(<[f32; 4]>::from(result.rotation)),
    //         scale: Vec3::from(<[f32; 3]>::from(result.scale)),
    //     };
    // }
}

fn select_resource(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    selection: &mut InspectorSelection,
) {
    let mut resources: Vec<_> = type_registry
        .read()
        .iter()
        .filter(|registration| registration.data::<ReflectResource>().is_some())
        .map(|registration| (registration.short_name().to_owned(), registration.type_id()))
        .collect();
    resources.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

    for (resource_name, type_id) in resources {
        let selected = match *selection {
            InspectorSelection::Resource(selected, _) => selected == type_id,
            _ => false,
        };

        if ui.selectable_label(selected, &resource_name).clicked() {
            *selection = InspectorSelection::Resource(type_id, resource_name);
        }
    }
}

fn select_asset(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    world: &World,
    selection: &mut InspectorSelection,
) {
    let type_registry = type_registry.read();
    let mut assets: Vec<_> = type_registry
        .iter()
        .filter_map(|registration| {
            let reflect_asset = registration.data::<ReflectAsset>()?;
            Some((
                registration.short_name().to_owned(),
                registration.type_id(),
                reflect_asset,
            ))
        })
        .collect();
    assets.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));

    for (asset_name, asset_type_id, reflect_asset) in assets {
        let mut handles: Vec<_> = reflect_asset.ids(world).collect();
        handles.sort();

        ui.collapsing(format!("{asset_name} ({})", handles.len()), |ui| {
            for handle in handles {
                let selected = match *selection {
                    InspectorSelection::Asset(_, _, selected_id) => selected_id == handle,
                    _ => false,
                };

                if ui
                    .selectable_label(selected, format!("{:?}", handle))
                    .clicked()
                {
                    *selection =
                        InspectorSelection::Asset(asset_type_id, asset_name.clone(), handle);
                }
            }
        });
    }
}
pub fn show_ui_system(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<EditorState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

pub fn set_gizmo_mode(input: Res<Input<KeyCode>>, mut ui_state: ResMut<EditorState>) {
    for (key, mode) in [
        (KeyCode::R, GizmoMode::Rotate),
        (KeyCode::T, GizmoMode::Translate),
        (KeyCode::S, GizmoMode::Scale),
    ] {
        if input.just_pressed(key) {
            ui_state.gizmo_mode = mode;
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum InspectorSelection {
    Entities,
    Resource(TypeId, String),
    Asset(TypeId, String, HandleId),
}

fn tilemap_ui(
    world: &mut World,
    ui: &mut egui::Ui,
    tile_size: &mut u32,
    active_tile: &mut Option<u32>,
) {
    ui.label("tilemap");
    ui.add(egui::widgets::Slider::new(tile_size, 1..=128));
    let tile_size = tile_size.clone();

    // Definitely really bad
    let mut system_state: SystemState<(
        Res<Assets<TextureAtlas>>,
        ResMut<EguiUserTextures>,
        EventWriter<BrushSelectEvent>,
    )> = SystemState::new(world);

    let (textures, mut user_textures, mut brush_select_event) = system_state.get_mut(world);

    for atlas in textures.iter() {
        let texture_id = user_textures.add_image(atlas.1.texture.clone());
        let width = (ui.available_width() as u32).div_euclid(tile_size + 4);
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(
                width as f32 * (tile_size as f32 + 4.0),
                (tile_size as f32 + 4.0)
                    * (atlas.1.textures.len() as f32 / width as f32).ceil() as f32,
            ),
            egui::Sense {
                click: true,
                drag: true,
                focusable: true,
            },
        );
        let size = atlas.1.size;
        for (i, texture_rect) in atlas.1.textures.iter().enumerate() {
            painter.image(
                texture_id,
                egui::Rect::from_min_max(
                    egui::Pos2::new(2.0, 2.0),
                    egui::Vec2::splat(tile_size as f32 + 2.0).to_pos2(),
                )
                .translate(
                    response.rect.min.to_vec2()
                        + egui::Vec2::new(
                            (tile_size as f32 + 4.0) * (i as u32 % width) as f32,
                            (tile_size as f32 + 4.0) * (i as u32).div_euclid(width) as f32,
                        ),
                ),
                egui::Rect::from_min_max(
                    egui::Pos2::new(texture_rect.min.x / size.x, texture_rect.min.y / size.y),
                    egui::Pos2::new(texture_rect.max.x / size.x, texture_rect.max.y / size.y),
                ),
                egui::Color32::WHITE,
            );
            // ui.add(egui::widgets::Image)
        }
        let tile_outer_size = tile_size as f32 + 4.0;
        if let Some(pos) = response.hover_pos() {
            let pos = (pos - response.rect.min).to_pos2();
            painter.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(
                        pos.x.div_euclid(tile_outer_size) * tile_outer_size + 1.0,
                        pos.y.div_euclid(tile_outer_size) * tile_outer_size + 1.0,
                    ),
                    egui::pos2(
                        pos.x.div_euclid(tile_outer_size).add(1.0) * tile_outer_size - 1.0,
                        pos.y.div_euclid(tile_outer_size).add(1.0) * tile_outer_size - 1.0,
                    ),
                )
                .translate(response.rect.min.to_vec2()),
                egui::Rounding::none(),
                egui::Stroke {
                    width: 2.0,
                    color: egui::Color32::from_rgb(0xad, 0xad, 0xad),
                },
            );

            if response.clicked() {
                let index = (pos.x / tile_outer_size).floor() as u32
                    + (pos.y / tile_outer_size).floor() as u32 * width;
                *active_tile = Some(index);
                brush_select_event.send(BrushSelectEvent { tile: index });
            }
        }
        if let Some(active_tile) = active_tile {
            painter.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(1.0, 1.0),
                    egui::pos2(tile_outer_size - 1.0, tile_outer_size - 1.0),
                )
                .translate(
                    egui::vec2(
                        (*active_tile % width) as f32,
                        active_tile.div_euclid(width) as f32,
                    ) * tile_outer_size
                        + response.rect.min.to_vec2(),
                ),
                egui::Rounding::none(),
                egui::Stroke {
                    width: 2.0,
                    color: egui::Color32::from_rgb(0xff, 0x61, 0x88),
                },
            );
        }
    }
}

#[derive(Component)]
pub struct Brush;

#[derive(Component)]
pub struct BrushTile;

#[derive(Event)]
pub struct BrushSelectEvent {
    pub tile: u32,
}

pub fn brush_system(
    ui_state: Res<EditorState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    egui_settings: Res<bevy_egui::EguiSettings>,

    mut brush_select_event: EventReader<BrushSelectEvent>,
    mut brush_query: Query<(&mut Transform, &mut TextureAtlasSprite), With<Brush>>,
    mut cursor_move: EventReader<CursorMoved>,
    buttons: Res<Input<MouseButton>>,
    mut tile_map_q: Query<&mut TileMap>,

    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let Ok(window) = primary_window.get_single() else {
        return;
    };

    let scale_factor = window.scale_factor() * egui_settings.scale_factor;

    let viewport_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor as f32;
    let viewport_size = ui_state.viewport_rect.size() * scale_factor as f32;

    UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32);
    UVec2::new(viewport_size.x as u32, viewport_size.y as u32);

    let (camera, camera_transform) = camera_q.single();

    let (mut transform, mut sprite) = brush_query.get_single_mut().unwrap();
    for event in brush_select_event.iter() {
        sprite.index = event.tile as usize;
    }
    for event in cursor_move.iter() {
        let cursor = event.position - Vec2::new(viewport_pos.x, viewport_pos.y);
        if let Some(world_space) = camera.viewport_to_world_2d(camera_transform, cursor) {
            transform.translation = world_space.extend(0.0).div(8.0).round().mul(8.0);
        }
    }
    if mouse_in_viewport(window, &ui_state) {
        for button in buttons.get_pressed() {
            if button == &MouseButton::Left {
                let mut tile_map: bevy::prelude::Mut<'_, TileMap> =
                    tile_map_q.get_single_mut().unwrap();
                tile_map.set_tile(
                    transform.translation.div(8.0).as_ivec3(),
                    Some(Tile {
                        sprite_index: sprite.index as u32,
                        ..Default::default()
                    }),
                )
            } else if button == &MouseButton::Right {
                let mut tile_map: bevy::prelude::Mut<'_, TileMap> =
                    tile_map_q.get_single_mut().unwrap();

                tile_map.set_tile(transform.translation.div(8.0).as_ivec3(), None)
            }
        }
    }
}

pub fn mouse_in_viewport(window: &Window, ui_state: &EditorState) -> bool {
    if let Some(Vec2 { x: c_x, y: c_y }) = window.cursor_position() {
        if ui_state.viewport_rect.contains(egui::pos2(c_x, c_y)) {
            return true;
        }
    }
    return false;
}

#[derive(Component)]
pub struct MainCamera;
