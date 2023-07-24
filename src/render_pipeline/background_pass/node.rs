use crate::render_pipeline::{MainPassSettings, RenderGraphSettings};

use super::{BackgroundPassUniformBuffer, KerrPassPipelineData};

use bevy::{
    prelude::*,
    render::{
        render_graph::{self, SlotInfo, SlotType},
        render_resource::*,
        view::ViewTarget,
    },
};

pub struct KerrPassNode {
    query: QueryState<(
        &'static ViewTarget,
        Option<&'static BackgroundPassUniformBuffer>,
        &'static MainPassSettings,
    )>,
}

impl KerrPassNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl render_graph::Node for KerrPassNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        println!("running background pass");
        let view_entity = graph.view_entity();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline_data = world.get_resource::<KerrPassPipelineData>().unwrap();
        let render_graph_settings = world.get_resource::<RenderGraphSettings>().unwrap();

        if !render_graph_settings.trace {
            return Ok(());
        }

        let (target, uniform_buffer, main_pass_settings) =
            match self.query.get_manual(world, view_entity) {
                Ok(result) => result,
                Err(_) => panic!("Camera missing component!"),
            };

        let trace_pipeline = match pipeline_cache.get_render_pipeline(pipeline_data.pipeline_id) {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        let post_process = target.post_process_write();

        let bind_group = render_context
            .render_device()
            .create_bind_group(&BindGroupDescriptor {
                label: Some("kerr pass bind group"),
                layout: &pipeline_data.bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer
                        .expect("Metric set but uniform not extracted!")
                        .binding()
                        .unwrap(),
                }],
            });

        let render_pass_descriptor = RenderPassDescriptor {
            label: Some("kerr pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &post_process.destination, //&target.main_texture_view(),
                // view: &target.main_texture().create_view(&desc),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&render_pass_descriptor);

        // render_pass.set_bind_group(0, &voxel_data.bind_group, &[]);
        render_pass.set_bind_group(0, &bind_group, &[]);

        render_pass.set_pipeline(trace_pipeline);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
