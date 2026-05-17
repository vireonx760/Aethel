use crate::core::renderer::WidgetInstance;
use crate::gui::shader::CustomShader;
use std::mem;
use wgpu::{
    BindGroup, BlendState, Buffer, BufferAddress, ColorTargetState, ColorWrites, Device,
    FragmentState, MultisampleState, PipelineLayout, PipelineLayoutDescriptor, PrimitiveState,
    PrimitiveTopology, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, TextureFormat, VertexBufferLayout, VertexState,
    VertexStepMode, vertex_attr_array,
};

pub struct PipelineCache {
    layout: PipelineLayout,
    format: TextureFormat,
    base: RenderPipeline,
    custom: Vec<CustomPipeline>,
}

struct CustomPipeline {
    mode_key: u32,
    pipeline: RenderPipeline,
}

impl PipelineCache {
    pub fn new(
        device: &Device,
        layout: PipelineLayout,
        shader: &ShaderModule,
        format: TextureFormat,
    ) -> Self {
        let base = create_instance_pipeline(
            device,
            &layout,
            shader,
            format,
            "vs_main",
            "fs_main",
            "AethelGUI Base Pipeline",
        );
        Self {
            layout,
            format,
            base,
            custom: Vec::with_capacity(4),
        }
    }

    pub fn create_layout(
        device: &Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> PipelineLayout {
        device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("AethelGUI Pipeline Layout"),
            bind_group_layouts,
            ..Default::default()
        })
    }

    pub fn register_custom_shader(&mut self, device: &Device, shader: &CustomShader) {
        let Some(mode_key) = shader.mode.custom_key() else {
            return;
        };
        if self
            .custom
            .iter()
            .any(|pipeline| pipeline.mode_key == mode_key)
        {
            return;
        }

        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(&shader.name),
            source: ShaderSource::Wgsl(shader.wgsl_source.clone().into()),
        });
        let pipeline = create_instance_pipeline(
            device,
            &self.layout,
            &module,
            self.format,
            &shader.vertex_entry,
            &shader.fragment_entry,
            &shader.name,
        );
        self.custom.push(CustomPipeline { mode_key, pipeline });
    }

    #[inline]
    pub fn custom_len(&self) -> usize {
        self.custom.len()
    }

    #[inline]
    pub fn pipeline_for_key(&self, mode_key: Option<u32>) -> &RenderPipeline {
        mode_key
            .and_then(|key| {
                self.custom
                    .iter()
                    .find(|pipeline| pipeline.mode_key == key)
                    .map(|pipeline| &pipeline.pipeline)
            })
            .unwrap_or(&self.base)
    }

    pub fn bind_base<'a>(
        &'a self,
        rpass: &mut RenderPass<'a>,
        bind_group: &'a BindGroup,
        instance_buffer: &'a Buffer,
    ) {
        bind_pipeline(rpass, &self.base, bind_group, instance_buffer);
    }

    pub fn bind_for_key<'a>(
        &'a self,
        rpass: &mut RenderPass<'a>,
        mode_key: Option<u32>,
        bind_group: &'a BindGroup,
        instance_buffer: &'a Buffer,
    ) {
        bind_pipeline(
            rpass,
            self.pipeline_for_key(mode_key),
            bind_group,
            instance_buffer,
        );
    }
}

pub fn create_instance_pipeline(
    device: &Device,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    format: TextureFormat,
    vertex_entry: &str,
    fragment_entry: &str,
    label: &str,
) -> RenderPipeline {
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: VertexState {
            module: shader,
            entry_point: vertex_entry,
            buffers: &[VertexBufferLayout {
                array_stride: mem::size_of::<WidgetInstance>() as BufferAddress,
                step_mode: VertexStepMode::Instance,
                attributes: &vertex_attr_array![
                    0 => Float32x2, 1 => Float32x2, 2 => Float32x4,
                    3 => Float32,   4 => Float32,   5 => Float32x2,
                    6 => Float32x2, 7 => Float32,   8 => Float32,
                ],
            }],
        },
        fragment: Some(FragmentState {
            module: shader,
            entry_point: fragment_entry,
            targets: &[Some(ColorTargetState {
                format,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    })
}

fn bind_pipeline<'a>(
    rpass: &mut RenderPass<'a>,
    pipeline: &'a RenderPipeline,
    bind_group: &'a BindGroup,
    instance_buffer: &'a Buffer,
) {
    rpass.set_pipeline(pipeline);
    rpass.set_bind_group(0, bind_group, &[]);
    rpass.set_vertex_buffer(0, instance_buffer.slice(..));
}
