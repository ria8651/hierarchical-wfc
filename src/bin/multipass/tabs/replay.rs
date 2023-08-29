use bevy::{ecs::system::SystemState, prelude::*};
use hierarchical_wfc::{
    ui_plugin::{EcsTab, EcsUiTab},
    wfc::bevy_passes::WfcFCollapsedData,
};

use crate::replay::ReplayPassProgress;

pub struct EcsUiReplay {
    system_state: SystemState<
        Query<
            'static,
            'static,
            (
                &'static mut ReplayPassProgress,
                &'static WfcFCollapsedData,
                Option<&'static Children>,
            ),
        >,
    >,
}

impl EcsUiReplay {
    pub fn tab_from_world(world: &mut World) -> EcsUiTab {
        EcsUiTab::Ecs(Box::new(Self {
            system_state: SystemState::new(world),
        }))
    }
}

impl std::fmt::Debug for EcsUiReplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Replay").finish()
    }
}

impl EcsTab for EcsUiReplay {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        _type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let mut q_passes = self.system_state.get_mut(world);

        ui.horizontal(|ui| {
            if ui.button("Hide all").clicked() {
                for (_, (mut replay_pass, _data, _children)) in q_passes.iter_mut().enumerate() {
                    replay_pass.current = 0;
                }
            }
            if ui.button("Show all").clicked() {
                for (_, (mut replay_pass, _data, _children)) in q_passes.iter_mut().enumerate() {
                    replay_pass.current = replay_pass.length;
                }
            }
        });
        for (_, (mut replay_pass, data, _children)) in q_passes.iter_mut().enumerate() {
            ui.collapsing(
                data.label.as_deref().unwrap_or("Unnamed Pass").to_string(),
                |ui| {
                    ui.horizontal(|ui| {
                        if replay_pass.playing {
                            if ui.button("Pause").clicked() {
                                replay_pass.playing = false;
                            }
                        } else if ui.button("Play").clicked() {
                            replay_pass.playing = true;
                            if replay_pass.current >= replay_pass.length {
                                replay_pass.current = 0;
                            }
                        }
                        if replay_pass.current == 0 {
                            if ui.button("Show").clicked() {
                                replay_pass.current = replay_pass.length;
                            }
                        } else if ui.button("Hide").clicked() {
                            replay_pass.current = 0;
                        }
                    });
                    let progress = (replay_pass.current as f32
                        / (replay_pass.length as f32).max(1.0))
                    .clamp(0.0, 1.0);

                    let mut updated_progress = progress;
                    ui.add(egui::Slider::new(&mut updated_progress, 0f32..=1f32).show_value(false));
                    if progress != updated_progress {
                        replay_pass.current = ((replay_pass.length as f32).max(0.0)
                            * updated_progress.clamp(0.0, 1.0))
                            as usize;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Duration:");
                        ui.add(
                            egui::DragValue::new(&mut replay_pass.duration)
                                .suffix("s")
                                .speed(0.1),
                        );
                    });
                },
            );
        }

        self.system_state.apply(world);
    }
}
