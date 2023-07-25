use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView},
        Render, RenderApp, RenderSet,
    },
};
pub use node::KerrPassNode;

use super::MainPassSettings;

mod node;

pub struct KerrPassPlugin;

impl Plugin for KerrPassPlugin {
    fn build(&self, app: &mut App) {
        // setup custom render pipeline
        app.sub_app_mut(RenderApp)
            .add_systems(Render, prepare_uniforms.in_set(RenderSet::Prepare));
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<KerrPassPipelineData>();
    }
}

#[derive(Resource)]
struct KerrPassPipelineData {
    pipeline_id: CachedRenderPipelineId,
    bind_group_layout: BindGroupLayout,
}

// #[derive(Clone, PartialEq, Reflect)]
// pub struct KerrSettings {
//     pub velocity: Vec3,
//     pub surface_bool: bool,
//     pub disk_bool: bool,
//     pub disk_hide: bool,
//     pub step_count: i32,
//     pub rel_error: f32,
//     pub abs_error: f32,
//     pub initial_step: f32,
//     pub max_step: f32,
//     pub method: IntegrationMethod,
//     pub disk_start: f32,
//     pub disk_end: f32,
//     pub spin: f32,
//     pub misc_bool: bool,
//     pub misc_float: f32,
// }

// impl Default for KerrSettings {
//     fn default() -> Self {
//         Self {
//             velocity: Vec3::ZERO,
//             surface_bool: false,
//             disk_bool: false,
//             disk_hide: false,
//             misc_bool: false,
//             step_count: 100,
//             initial_step: 0.00050,
//             rel_error: 0.0000010,
//             abs_error: 0.0000010,
//             max_step: 1.0,
//             method: IntegrationMethod::Rk4,
//             disk_start: 1.0,
//             disk_end: 12.0,
//             spin: 1.0,
//             misc_float: 1.0,
//         }
//     }
// }

#[derive(Clone, ShaderType)]
pub struct TraceUniforms {
    pub color: Vec4,
}

#[derive(Component, Deref, DerefMut)]
struct BackgroundPassUniformBuffer(UniformBuffer<TraceUniforms>);

fn prepare_uniforms(
    mut commands: Commands,
    query: Query<(Entity, &MainPassSettings, &ExtractedView)>,
    time: Res<Time>,
    world: &World,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    dbg!(&world);

    let _elapsed = time.elapsed_seconds_f64();

    for (entity, _settings, _view) in query.iter() {
        let uniforms = TraceUniforms {
            color: Vec4::splat(0.5),
        };
        let mut uniform_buffer = UniformBuffer::from(uniforms);
        uniform_buffer.write_buffer(&render_device, &render_queue);

        commands
            .entity(entity)
            .insert(BackgroundPassUniformBuffer(uniform_buffer));
    }
}

impl FromWorld for KerrPassPipelineData {
    fn from_world(render_world: &mut World) -> Self {
        dbg!(&render_world);
        let asset_server = render_world.get_resource::<AssetServer>().unwrap();

        let bind_group_layout = render_world
            .resource::<RenderDevice>()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("kerr bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(TraceUniforms::SHADER_SIZE.into()),
                    },
                    count: None,
                }],
            });

        let trace_shader = asset_server.load("shaders/background_grid.wgsl");

        let trace_pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("kerr pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: trace_shader,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb, //ViewTarget::TEXTURE_FORMAT_HDR,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
        };

        let cache = render_world.resource::<PipelineCache>();
        let pipeline_id = cache.queue_render_pipeline(trace_pipeline_descriptor);

        KerrPassPipelineData {
            pipeline_id,
            bind_group_layout,
        }
    }
}
