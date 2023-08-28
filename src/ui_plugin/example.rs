use std::any::TypeId;

use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_asset::{HandleId, ReflectAsset};
use bevy_egui::EguiContext;
use bevy_inspector_egui::{
    bevy_inspector::{
        self,
        hierarchy::{hierarchy_ui, SelectedEntities},
        ui_for_entities_shared_components, ui_for_entity_with_children,
    },
    reflect_inspector, DefaultInspectorConfigPlugin,
};
// use bevy_mod_picking::backends::egui::EguiPointer;
// use bevy_mod_picking::prelude::*;
use bevy_egui::EguiSet;
use bevy_rapier3d::prelude::CollisionEvent;
use bevy_reflect::TypeRegistry;
use bevy_render::camera::Viewport;
use egui_dock::{DockArea, NodeIndex, Style, Tree};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugin(bevy_framepace::FramepacePlugin) // reduces input lag
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(bevy_egui::EguiPlugin)
        .init_resource::<UiState>()
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            show_ui_system
                .before(EguiSet::ProcessOutput)
                .before(bevy::transform::TransformSystem::TransformPropagate),
        )
        .add_systems(PostUpdate, set_camera_viewport.after(show_ui_system))
        .register_type::<Option<Handle<Image>>>()
        .register_type::<AlphaMode>()
        .run();
}

/*
fn auto_add_raycast_target(
    mut commands: Commands,
    query: Query<Entity, (Without<PickRaycastTarget>, With<Handle<Mesh>>)>,
) {
    for entity in &query {
        commands
            .entity(entity)
            .insert((PickRaycastTarget::default(), PickableBundle::default()));
    }
}

fn handle_pick_events(
    mut ui_state: ResMut<UiState>,
    mut click_events: EventReader<PointerClick>,
    mut egui: ResMut<EguiContext>,
    egui_entity: Query<&EguiPointer>,
) {
    let egui_context = egui.ctx_mut();

    for click in click_events.iter() {
        if egui_entity.get(click.target()).is_ok() {
            continue;
        };

        let modifiers = egui_context.input().modifiers;
        let add = modifiers.ctrl || modifiers.shift;

        ui_state
            .selected_entities
            .select_maybe_add(click.target(), add);
    }
}
*/

#[derive(Component)]
struct MainCamera;

fn show_ui_system(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<UiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

// make camera only render to view not obstructed by UI
fn set_camera_viewport(
    ui_state: Res<UiState>,
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

    cam.viewport = Some(Viewport {
        physical_position: UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32),
        physical_size: UVec2::new(viewport_size.x as u32, viewport_size.y as u32),
        depth: 0.0..1.0,
    });
}

#[derive(Resource)]
struct UiState {
    tree: Tree<EguiWindow>,
    viewport_rect: egui::Rect,
}

impl FromWorld for UiState {
    fn from_world(world: &mut World) -> Self {
        Self::new(world)
    }
}

impl UiState {
    pub fn new(world: &mut World) -> Self {
        let mut tree = Tree::new(vec![EguiWindow::GameView]);
        let [game, inspector] = tree.split_below(
            NodeIndex::root(),
            0.75,
            vec![
                EguiWindow::ECS(Box::new(EcsTransformUi::new(world))),
                EguiWindow::ECS(Box::new(EcsCameraUi::new(world))),
            ],
        );
        Self {
            tree,
            viewport_rect: egui::Rect::NOTHING,
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &mut egui::Context) {
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
enum EguiWindow {
    GameView,
    ECS(Box<dyn EcsUiNode + Send + Sync>),
}

struct TabViewer<'a> {
    world: &'a mut World,
    viewport_rect: &'a mut egui::Rect,
}

trait EcsUiNode: std::fmt::Debug {
    fn ui(&mut self, world: &mut World, ui: &mut egui::Ui, type_registry: &TypeRegistry);
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = EguiWindow;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        let type_registry = self.world.resource::<AppTypeRegistry>().0.clone();
        let type_registry = type_registry.read();

        match window {
            EguiWindow::GameView => {
                *self.viewport_rect = ui.clip_rect();
            }
            EguiWindow::ECS(node) => {
                node.ui(self.world, ui, &type_registry);
            }
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        format!("{window:?}").into()
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, EguiWindow::GameView)
    }
}

pub struct EcsTransformUi {
    system_state: SystemState<Query<'static, 'static, &'static mut Transform>>,
}

impl EcsTransformUi {
    fn new(world: &mut World) -> Self {
        Self {
            system_state: SystemState::new(world),
        }
    }
}

impl std::fmt::Debug for EcsTransformUi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsTransformUi").finish()
    }
}
impl EcsUiNode for EcsTransformUi {
    fn ui(&mut self, world: &mut World, ui: &mut egui::Ui, _type_registry: &TypeRegistry) {
        let mut transform_q = self.system_state.get_mut(world);
        for mut transform in transform_q.iter_mut() {
            ui.add(egui::DragValue::new(&mut transform.translation.x));
        }
    }
}

pub struct EcsCameraUi {
    system_state: SystemState<Query<'static, 'static, &'static mut Projection>>,
}

impl EcsCameraUi {
    fn new(world: &mut World) -> Self {
        Self {
            system_state: SystemState::new(world),
        }
    }
}

impl std::fmt::Debug for EcsCameraUi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsCameraUi").finish()
    }
}
impl EcsUiNode for EcsCameraUi {
    fn ui(&mut self, world: &mut World, ui: &mut egui::Ui, type_registry: &TypeRegistry) {
        let mut camera_q = self.system_state.get_mut(world);
        for mut camera in camera_q.iter_mut() {
            reflect_inspector::ui_for_value(camera.into_inner(), ui, type_registry);
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let box_size = 2.0;
    let box_thickness = 0.15;
    let box_offset = (box_size + box_thickness) / 2.0;

    // left - red
    let mut transform = Transform::from_xyz(-box_offset, box_offset, 0.0);
    transform.rotate(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size,
            box_thickness,
            box_size,
        ))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.63, 0.065, 0.05),
            ..Default::default()
        }),
        ..Default::default()
    });
    // right - green
    let mut transform = Transform::from_xyz(box_offset, box_offset, 0.0);
    transform.rotate(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size,
            box_thickness,
            box_size,
        ))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.14, 0.45, 0.091),
            ..Default::default()
        }),
        ..Default::default()
    });
    // bottom - white
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size + 2.0 * box_thickness,
            box_thickness,
            box_size,
        ))),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.725, 0.71, 0.68),
            ..Default::default()
        }),
        ..Default::default()
    });
    // top - white
    let transform = Transform::from_xyz(0.0, 2.0 * box_offset, 0.0);
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size + 2.0 * box_thickness,
            box_thickness,
            box_size,
        ))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.725, 0.71, 0.68),
            ..Default::default()
        }),
        ..Default::default()
    });
    // back - white
    let mut transform = Transform::from_xyz(0.0, box_offset, -box_offset);
    transform.rotate(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size + 2.0 * box_thickness,
            box_thickness,
            box_size + 2.0 * box_thickness,
        ))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.725, 0.71, 0.68),
            ..Default::default()
        }),
        ..Default::default()
    });

    // ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.02,
    });
    // top light
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane::from_size(0.4))),
            transform: Transform::from_matrix(Mat4::from_scale_rotation_translation(
                Vec3::ONE,
                Quat::from_rotation_x(std::f32::consts::PI),
                Vec3::new(0.0, box_size + 0.5 * box_thickness, 0.0),
            )),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE * 100.0,
                ..Default::default()
            }),
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn(PointLightBundle {
                point_light: PointLight {
                    color: Color::WHITE,
                    intensity: 25.0,
                    ..Default::default()
                },
                transform: Transform::from_translation((box_thickness + 0.05) * Vec3::Y),
                ..Default::default()
            });
        });
    // directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0)),
        ..Default::default()
    });

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, box_offset, 4.0)
                .looking_at(Vec3::new(0.0, box_offset, 0.0), Vec3::Y),
            ..Default::default()
        },
        MainCamera,
        // PickRaycastSource,
    ));
}
