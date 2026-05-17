pub mod buffers;
pub mod pipeline;
pub mod plan;
pub mod stats;

pub use buffers::{DEFAULT_INSTANCE_CAPACITY, InstanceBufferArena};
pub use pipeline::PipelineCache;
pub use plan::{DrawPacket, DrawPlanner};
pub use stats::{GpuDrawStats, GpuStats, GpuUploadStats};

use crate::core::renderer::WidgetInstance;
use crate::gui::paint::{RenderBatch, RenderLayer};
use crate::gui::shader::CustomShader;
use buffers::InstanceBufferArena as InstanceArena;
use pipeline::PipelineCache as Pipelines;
use plan::DrawPlanner as Planner;
use stats::GpuStats as Stats;
use wgpu::{
    BindGroup, Buffer, Device, Queue, RenderPass, ShaderModule, SurfaceConfiguration, TextureFormat,
};

pub struct GpuAccelerator {
    instances: InstanceArena,
    pipelines: Pipelines,
    planner: Planner,
    stats: Stats,
}

impl GpuAccelerator {
    pub fn new(
        device: &Device,
        pipeline_layout: wgpu::PipelineLayout,
        base_shader: &ShaderModule,
        surface_format: TextureFormat,
        initial_instance_capacity: u64,
    ) -> Self {
        Self {
            instances: InstanceArena::new(device, initial_instance_capacity),
            pipelines: Pipelines::new(device, pipeline_layout, base_shader, surface_format),
            planner: Planner::new(),
            stats: Stats::default(),
        }
    }

    pub fn register_custom_shader(&mut self, device: &Device, shader: &CustomShader) {
        self.pipelines.register_custom_shader(device, shader);
        self.stats.custom_pipelines = self.pipelines.custom_len();
    }

    pub fn upload_instances(
        &mut self,
        device: &Device,
        queue: &Queue,
        instances: &[WidgetInstance],
    ) {
        self.stats.upload = self.instances.upload(device, queue, instances);
        self.stats.custom_pipelines = self.pipelines.custom_len();
    }

    pub fn configure_surface(
        surface: &wgpu::Surface<'_>,
        device: &Device,
        config: &SurfaceConfiguration,
    ) {
        surface.configure(device, config);
    }

    #[inline]
    pub fn instance_buffer(&self) -> &Buffer {
        self.instances.buffer()
    }

    #[inline]
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    #[inline]
    pub fn bind_base<'a>(&'a self, rpass: &mut RenderPass<'a>, bind_group: &'a BindGroup) {
        self.pipelines
            .bind_base(rpass, bind_group, self.instances.buffer());
    }

    pub fn plan_batches(&mut self, batches: &[RenderBatch], width: u32, height: u32) {
        self.planner.plan(batches, width, height);
        self.stats.draw = self.planner.stats();
    }

    pub fn draw_planned_batches<'a>(
        &'a self,
        rpass: &mut RenderPass<'a>,
        batches: &[RenderBatch],
        bind_group: &'a BindGroup,
        surface_width: u32,
        surface_height: u32,
    ) {
        for packet in self.planner.packets() {
            let Some(batch) = batches.get(packet.batch_index) else {
                continue;
            };
            self.bind_for_packet(rpass, packet.shader_key, bind_group);
            apply_scissor(rpass, packet.scissor, surface_width, surface_height);
            rpass.draw(0..4, batch.range.clone());
        }
    }

    pub fn draw_batch_immediate<'a>(
        &'a self,
        rpass: &mut RenderPass<'a>,
        batch: &RenderBatch,
        bind_group: &'a BindGroup,
        surface_width: u32,
        surface_height: u32,
    ) {
        if batch.range.is_empty() {
            return;
        }
        if batch.clip.is_some() && batch.scissor(surface_width, surface_height).is_none() {
            return;
        }
        self.bind_for_packet(rpass, batch.shader_key, bind_group);
        apply_scissor(
            rpass,
            batch.scissor(surface_width, surface_height),
            surface_width,
            surface_height,
        );
        rpass.draw(0..4, batch.range.clone());
    }

    pub fn draw_raw_range<'a>(
        &'a self,
        rpass: &mut RenderPass<'a>,
        bind_group: &'a BindGroup,
        range: std::ops::Range<u32>,
    ) {
        if range.is_empty() {
            return;
        }
        self.bind_base(rpass, bind_group);
        rpass.draw(0..4, range);
    }

    fn bind_for_packet<'a>(
        &'a self,
        rpass: &mut RenderPass<'a>,
        shader_key: Option<u32>,
        bind_group: &'a BindGroup,
    ) {
        self.pipelines
            .bind_for_key(rpass, shader_key, bind_group, self.instances.buffer());
    }
}

pub fn create_pipeline_layout(
    device: &Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::PipelineLayout {
    Pipelines::create_layout(device, bind_group_layouts)
}

pub fn regular_then_overlay(batches: &[RenderBatch]) -> (&[RenderBatch], &[RenderBatch]) {
    let overlay_start = batches
        .iter()
        .position(|batch| batch.layer == RenderLayer::Overlay)
        .unwrap_or(batches.len());
    (&batches[..overlay_start], &batches[overlay_start..])
}

pub fn apply_scissor(
    rpass: &mut RenderPass<'_>,
    scissor: Option<crate::gui::clip::ScissorRect>,
    width: u32,
    height: u32,
) {
    if let Some(scissor) = scissor {
        rpass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
    } else if width > 0 && height > 0 {
        rpass.set_scissor_rect(0, 0, width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::paint::RenderLayer;

    #[test]
    fn regular_then_overlay_splits_at_first_overlay() {
        let batches = vec![
            RenderBatch {
                layer: RenderLayer::Regular,
                range: 0..1,
                clip: None,
                shader_key: None,
            },
            RenderBatch {
                layer: RenderLayer::Overlay,
                range: 1..2,
                clip: None,
                shader_key: None,
            },
            RenderBatch {
                layer: RenderLayer::Overlay,
                range: 2..3,
                clip: None,
                shader_key: None,
            },
        ];
        let (regular, overlay) = regular_then_overlay(&batches);
        assert_eq!(regular.len(), 1);
        assert_eq!(overlay.len(), 2);
    }
}
