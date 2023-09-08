use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::EguiContext;
use bevy_reflect::TypeRegistry;
use bevy_render::camera::Viewport;
use egui_dock::{DockArea, Style, Tree};

use crate::camera_plugin::cam_switcher::MainCamera;

pub struct EcsUiPlugin;
impl Plugin for EcsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show_ui_system)
            .add_systems(PostUpdate, set_camera_viewport)
            .register_type::<Option<Handle<Image>>>()
            .register_type::<AlphaMode>();
    }
}

fn show_ui_system(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<EcsUiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

// make camera only render to view not obstructed by UI
fn set_camera_viewport(
    ui_state: Res<EcsUiState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    egui_settings: Res<bevy_egui::EguiSettings>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
) {
    let mut cam = cameras.single_mut();

    let Ok(window) = primary_window.get_single() else {
        return;
    };

    let scale_factor = window.scale_factor() * egui_settings.scale_factor;

    let viewport_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor as f32;
    let viewport_size = ui_state.viewport_rect.size() * scale_factor as f32;

    if viewport_size.x as u32 > 0 && viewport_size.y as u32 > 0 {
        cam.viewport = Some(Viewport {
            physical_position: UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32),
            physical_size: UVec2::new(viewport_size.x as u32, viewport_size.y as u32),
            depth: 0.0..1.0,
        });
    }
}

#[derive(Resource)]
pub struct EcsUiState {
    pub tree: Tree<EcsUiTab>,
    viewport_rect: egui::Rect,
}

impl EcsUiState {
    pub fn new(tree: Tree<EcsUiTab>) -> Self {
        // let mut tree = Tree::new(vec![EguiWindow::GameView]);

        Self {
            tree,
            viewport_rect: egui::Rect::NOTHING,
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let mut tab_viewer = TabViewer {
            world,
            viewport_rect: &mut self.viewport_rect,
            // selected_entities: &mut self.selected_entities,
            // selection: &mut self.selection,
            // gizmo_mode: self.gizmo_mode,
        };
        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

#[derive(Debug)]
pub enum EcsUiTab {
    Viewport,
    Ecs(Box<dyn EcsTab + Send + Sync>),
}

struct TabViewer<'a> {
    world: &'a mut World,
    viewport_rect: &'a mut egui::Rect,
}

pub trait EcsTab: std::fmt::Debug {
    fn ui(&mut self, world: &mut World, ui: &mut egui::Ui, type_registry: &TypeRegistry);
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = EcsUiTab;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        let type_registry = self.world.resource::<AppTypeRegistry>().0.clone();
        let type_registry = type_registry.read();

        match window {
            EcsUiTab::Viewport => {
                *self.viewport_rect = ui.clip_rect();
            }
            EcsUiTab::Ecs(node) => {
                node.ui(self.world, ui, &type_registry);
            }
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        match window {
            Self::Tab::Viewport => "Viewport".into(),
            Self::Tab::Ecs(node) => format!("{:?}", node).into(),
        }
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, EcsUiTab::Viewport)
    }
}
