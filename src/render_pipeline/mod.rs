use self::background_pass::{KerrPassNode, KerrPassPlugin};
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_graph::RenderGraph;
use bevy::{
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        RenderApp,
    },
};

mod background_pass;

pub struct RenderPlugin;

#[derive(Component, ExtractComponent, Clone)]
pub struct MainPassSettings;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RenderGraphSettings::default())
            .add_plugins(ExtractResourcePlugin::<RenderGraphSettings>::default())
            .add_plugins(KerrPassPlugin);

        app.add_plugins(ExtractComponentPlugin::<MainPassSettings>::default());
        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        // build main render graph
        let background_node = KerrPassNode::new(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        let sub_graph_2d = render_graph.sub_graph_mut("core_2d");
        // let input_node_id =
        //     render_graph.set_input(vec![SlotInfo::new("view_entity", SlotType::Entity)]);

        sub_graph_2d.add_node("background_pass", background_node);
        sub_graph_2d.add_node_edge("main_pass", "background_pass");
        sub_graph_2d.add_node_edge("background_pass", "bloom");
        sub_graph_2d.remove_node_edge("main_pass", "bloom").unwrap();

        // sub_graph_2d.add_node_edge("msaa_writeback", "background_pass");
        // sub_graph_2d.add_node_edge("background_pass", "main_pass");
        // sub_graph_2d.remove_node_edge("msaa_writeback", "main_pass");

        // render_graph.add_slot_edge(input_node_id, "view_entity", "background_pass", "view");

        // let mut graph = render_app.world.resource_mut::<RenderGraph>();
        // graph.add_sub_graph("main_render", render_graph);

        // // build main render graph
        // let mut render_graph = RenderGraph::default();
        // let input_node_id =
        //     render_graph.set_input(vec![SlotInfo::new("view_entity", SlotType::Entity)]);

        // let background_node = KerrPassNode::new(&mut render_app.world);

        // render_graph.add_node("background_pass", background_node);
        // render_graph.add_slot_edge(input_node_id, "view_entity", "background_pass", "view");
        // // render_graph.add_slot_edge(input_node_id, "view_entity", "background_pass", "view");

        // let mut graph = render_app.world.resource_mut::<RenderGraph>();
        // graph.add_sub_graph("main_render", render_graph);
    }
}

#[derive(Resource, Clone, ExtractResource)]
pub struct RenderGraphSettings {
    pub clear: bool,
    pub automata: bool,
    pub animation: bool,
    pub voxelization: bool,
    pub rebuild: bool,
    pub physics: bool,
    pub trace: bool,
    pub denoise: bool,
}

impl Default for RenderGraphSettings {
    fn default() -> Self {
        Self {
            clear: true,
            automata: true,
            animation: true,
            voxelization: true,
            rebuild: true,
            physics: true,
            trace: true,
            denoise: false,
        }
    }
}
