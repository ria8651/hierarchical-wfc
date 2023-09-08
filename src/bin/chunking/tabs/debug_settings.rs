use bevy::{ecs::system::SystemState, prelude::*};

use bevy_inspector_egui::reflect_inspector;
use egui::Checkbox;
use hierarchical_wfc::ui_plugin::{EcsTab, EcsUiTab};

use crate::fragments::{generate::FragmentSettings, plugin::GenerationDebugSettings};

pub struct EcsUiDebugSettings {
    system_state: SystemState<(
        ResMut<'static, GenerationDebugSettings>,
        ResMut<'static, FragmentSettings>,
    )>,
}

impl EcsUiDebugSettings {
    pub fn tab_from_world(world: &mut World) -> EcsUiTab {
        EcsUiTab::Ecs(Box::new(Self {
            system_state: SystemState::new(world),
        }))
    }
}

impl std::fmt::Debug for EcsUiDebugSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Debug Settings").finish()
    }
}

impl EcsTab for EcsUiDebugSettings {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let (mut debug_settings, fragment_settings) = self.system_state.get_mut(world);

        ui.label("Chunks");
        ui.checkbox(&mut debug_settings.debug_chunks, "Chunks");

        ui.add_space(ui.style().spacing.interact_size.y);

        ui.label("Fragments");
        egui::Grid::new("debug_fragment_settings_matrix")
            .striped(true)
            .show(ui, |ui| {
                ui.label("");
                ui.label("Create");
                ui.label("Show");
                ui.end_row();

                ui.label("Nodes");
                ui.add(Checkbox::without_text(
                    &mut debug_settings.create_fragment_nodes,
                ));
                ui.add(Checkbox::without_text(
                    &mut debug_settings.show_fragment_nodes,
                ));
                ui.end_row();

                ui.label("Edges");
                ui.add(Checkbox::without_text(
                    &mut debug_settings.create_fragment_edges,
                ));
                ui.add(Checkbox::without_text(
                    &mut debug_settings.show_fragment_edges,
                ));
                ui.end_row();

                ui.label("Faces");
                ui.add(Checkbox::without_text(
                    &mut debug_settings.create_fragment_faces,
                ));
                ui.add(Checkbox::without_text(
                    &mut debug_settings.show_fragment_faces,
                ));
                ui.end_row();
            });

        ui.add_space(ui.style().spacing.interact_size.y);

        ui.label("Fragment Settings");
        reflect_inspector::ui_for_value(fragment_settings.into_inner(), ui, type_registry);

        self.system_state.apply(world);
    }
}
