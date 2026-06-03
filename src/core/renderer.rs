use crate::gpu_core::{DEFAULT_INSTANCE_CAPACITY, GpuAccelerator, create_pipeline_layout};
use crate::gui::paint::FramePaint;
use crate::gui::shader::CustomShader;
use bytemuck::{Pod, Zeroable};
use glyphon::{
    Cache as GlyphCache, FontSystem, Resolution, SwashCache, TextArea, TextAtlas, TextRenderer,
    Viewport,
};
use std::error::Error;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::*;
use winit::window::Window;

fn select_backends() -> Backends {
    #[cfg(target_os = "macos")]
    return Backends::METAL;
    #[cfg(target_os = "windows")]
    return Backends::DX12 | Backends::VULKAN;
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    return Backends::VULKAN;
}

/// Fifo is the stable vsync fallback when low-latency modes are unavailable.
fn select_present_mode(caps: &SurfaceCapabilities) -> Option<PresentMode> {
    if caps.present_modes.contains(&PresentMode::Mailbox) {
        Some(PresentMode::Mailbox)
    } else if caps.present_modes.contains(&PresentMode::Immediate) {
        Some(PresentMode::Immediate)
    } else if caps.present_modes.contains(&PresentMode::Fifo) {
        Some(PresentMode::Fifo)
    } else {
        caps.present_modes.first().copied()
    }
}

fn select_alpha_mode(caps: &SurfaceCapabilities, transparent: bool) -> Option<CompositeAlphaMode> {
    if transparent {
        for mode in [
            CompositeAlphaMode::PreMultiplied,
            CompositeAlphaMode::PostMultiplied,
            CompositeAlphaMode::Inherit,
            CompositeAlphaMode::Auto,
        ] {
            if caps.alpha_modes.contains(&mode) {
                return Some(mode);
            }
        }
    }

    if caps.alpha_modes.contains(&CompositeAlphaMode::Opaque) {
        Some(CompositeAlphaMode::Opaque)
    } else {
        caps.alpha_modes.first().copied()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RendererOptions {
    pub clear_color: Color,
    pub transparent: bool,
}

impl RendererOptions {
    pub const fn new(clear_color: Color, transparent: bool) -> Self {
        Self {
            clear_color,
            transparent,
        }
    }

    pub const fn overlay() -> Self {
        Self {
            clear_color: Color::TRANSPARENT,
            transparent: true,
        }
    }
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            clear_color: Color {
                r: 0.05,
                g: 0.05,
                b: 0.05,
                a: 1.0,
            },
            transparent: false,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WidgetInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub radius: f32,
    pub mode: f32,
    pub clip_min: [f32; 2],
    pub clip_max: [f32; 2],
    pub use_clip: f32,
    pub rotation: f32,
}

impl Default for WidgetInstance {
    fn default() -> Self {
        Self {
            pos: [0.0; 2],
            size: [100.0; 2],
            color: [1.0; 4],
            radius: 0.0,
            mode: 0.0,
            clip_min: [0.0; 2],
            clip_max: [1e5; 2],
            use_clip: 0.0,
            rotation: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLayer {
    pub instance_end: u32,
    pub area_start: usize,
    pub area_end: usize,
}

impl TextLayer {
    #[inline]
    pub fn new(instance_end: u32, area_start: usize, area_end: usize) -> Self {
        Self {
            instance_end,
            area_start,
            area_end,
        }
    }

    #[inline]
    pub fn area_range(&self) -> Range<usize> {
        self.area_start..self.area_end
    }

    #[inline]
    pub fn has_text(&self) -> bool {
        self.area_start < self.area_end
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

#[derive(Debug)]
pub enum RendererInitError {
    CreateSurface(CreateSurfaceError),
    RequestAdapter(RequestAdapterError),
    RequestDevice(RequestDeviceError),
    MissingSurfaceFormat,
    MissingPresentMode,
    MissingAlphaMode,
}

impl fmt::Display for RendererInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateSurface(err) => write!(f, "failed to create wgpu surface: {err}"),
            Self::RequestAdapter(err) => write!(f, "failed to request GPU adapter: {err}"),
            Self::RequestDevice(err) => write!(f, "failed to create wgpu device: {err}"),
            Self::MissingSurfaceFormat => write!(f, "surface does not expose a texture format"),
            Self::MissingPresentMode => write!(f, "surface does not expose a present mode"),
            Self::MissingAlphaMode => write!(f, "surface does not expose an alpha mode"),
        }
    }
}

impl Error for RendererInitError {}

pub struct Renderer {
    _window: Arc<Window>,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    gpu: GpuAccelerator,
    uniform_buffer: wgpu::Buffer,
    bind_group: BindGroup,
    pub font_system: FontSystem,
    swash_cache: SwashCache,
    _glyph_cache: GlyphCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    regular_text_active: bool,
    text_renderer_layers: Vec<TextRenderer>,
    text_layer_active: Vec<bool>,
    active_text_layers: usize,
    text_renderer_overlay: TextRenderer,
    overlay_text_active: bool,
    clear_color: Color,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, RendererInitError> {
        Self::new_with_options(window, RendererOptions::default()).await
    }

    pub async fn new_with_options(
        window: Arc<Window>,
        options: RendererOptions,
    ) -> Result<Self, RendererInitError> {
        let mut instance_desc = InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = select_backends();
        let instance = Instance::new(instance_desc);

        let surface: Surface<'static> = instance
            .create_surface(Arc::clone(&window))
            .map_err(RendererInitError::CreateSurface)?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(RendererInitError::RequestAdapter)?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                required_limits: Limits::downlevel_defaults().using_resolution(adapter.limits()),
                ..DeviceDescriptor::default()
            })
            .await
            .map_err(RendererInitError::RequestDevice)?;

        let caps = surface.get_capabilities(&adapter);
        let surface_size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: caps
                .formats
                .first()
                .copied()
                .ok_or(RendererInitError::MissingSurfaceFormat)?,
            width: surface_size.width.max(1),
            height: surface_size.height.max(1),
            present_mode: select_present_mode(&caps)
                .ok_or(RendererInitError::MissingPresentMode)?,
            alpha_mode: select_alpha_mode(&caps, options.transparent)
                .ok_or(RendererInitError::MissingAlphaMode)?,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("GUI Shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/gui.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Uniforms"),
            contents: bytemuck::cast_slice(&[Uniforms {
                screen_size: [config.width as f32, config.height as f32],
                _pad: [0.0; 2],
            }]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let pipeline_layout = create_pipeline_layout(&device, &[&bgl]);
        let gpu = GpuAccelerator::new(
            &device,
            pipeline_layout,
            &shader,
            config.format,
            DEFAULT_INSTANCE_CAPACITY,
        );

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let glyph_cache = GlyphCache::new(&device);
        let mut viewport = Viewport::new(&device, &glyph_cache);
        viewport.update(
            &queue,
            Resolution {
                width: config.width,
                height: config.height,
            },
        );
        let mut atlas = TextAtlas::new(&device, &queue, &glyph_cache, config.format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);
        let text_renderer_overlay =
            TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

        Ok(Self {
            _window: window,
            device,
            queue,
            surface,
            config,
            gpu,
            uniform_buffer,
            bind_group,
            font_system,
            swash_cache,
            _glyph_cache: glyph_cache,
            viewport,
            atlas,
            text_renderer,
            regular_text_active: false,
            text_renderer_layers: Vec::with_capacity(8),
            text_layer_active: Vec::with_capacity(8),
            active_text_layers: 0,
            text_renderer_overlay,
            overlay_text_active: false,
            clear_color: options.clear_color,
        })
    }

    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn surface_format(&self) -> TextureFormat {
        self.config.format
    }

    pub fn surface_size(&self) -> [u32; 2] {
        [self.config.width, self.config.height]
    }

    pub fn register_custom_shader(&mut self, shader: &CustomShader) {
        self.gpu.register_custom_shader(&self.device, shader);
    }

    pub fn gpu_stats(&self) -> &crate::gpu_core::GpuStats {
        self.gpu.stats()
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.viewport.update(
            &self.queue,
            Resolution {
                width: size.width,
                height: size.height,
            },
        );
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms {
                screen_size: [size.width as f32, size.height as f32],
                _pad: [0.0; 2],
            }]),
        );
    }

    pub fn prepare_regular_text(&mut self, areas: &[TextArea]) {
        if areas.is_empty() {
            self.regular_text_active = false;
            self.active_text_layers = 0;
            return;
        }
        self.active_text_layers = 0;
        self.regular_text_active = self
            .text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                areas.iter().cloned(),
                &mut self.swash_cache,
            )
            .map_err(|err| eprintln!("text prepare error: {err}"))
            .is_ok();
    }

    pub fn prepare_regular_text_layers(&mut self, areas: &[TextArea], layers: &[TextLayer]) {
        self.regular_text_active = false;
        self.active_text_layers = layers.len();
        while self.text_renderer_layers.len() < layers.len() {
            self.text_renderer_layers.push(TextRenderer::new(
                &mut self.atlas,
                &self.device,
                MultisampleState::default(),
                None,
            ));
        }
        if self.text_layer_active.len() < layers.len() {
            self.text_layer_active.resize(layers.len(), false);
        }
        self.text_layer_active[..layers.len()].fill(false);

        for (index, (renderer, layer)) in self
            .text_renderer_layers
            .iter_mut()
            .zip(layers.iter())
            .enumerate()
        {
            if !layer.has_text() {
                continue;
            }
            self.text_layer_active[index] = renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    &mut self.font_system,
                    &mut self.atlas,
                    &self.viewport,
                    areas[layer.area_range()].iter().cloned(),
                    &mut self.swash_cache,
                )
                .map_err(|err| eprintln!("text layer prepare error: {err}"))
                .is_ok();
        }
    }

    pub fn prepare_overlay_text(&mut self, areas: &[TextArea]) {
        if areas.is_empty() {
            self.overlay_text_active = false;
            return;
        }
        self.overlay_text_active = self
            .text_renderer_overlay
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                areas.iter().cloned(),
                &mut self.swash_cache,
            )
            .map_err(|err| eprintln!("overlay text prepare error: {err}"))
            .is_ok();
    }

    pub fn render(&mut self, instances: &[WidgetInstance], regular_count: usize) {
        if instances.is_empty() {
            return;
        }

        self.gpu
            .upload_instances(&self.device, &self.queue, instances);

        let Some(frame) = self.acquire_frame() else {
            return;
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("GUI"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.clear_color),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.gpu.bind_base(&mut rpass, &self.bind_group);

            let regular_end = regular_count.min(instances.len()) as u32;
            let total = instances.len() as u32;

            if regular_end > 0 {
                self.gpu
                    .draw_raw_range(&mut rpass, &self.bind_group, 0..regular_end);
            }

            if self.regular_text_active {
                if let Err(err) = self
                    .text_renderer
                    .render(&self.atlas, &self.viewport, &mut rpass)
                {
                    eprintln!("text render error: {err}");
                }
                self.gpu.bind_base(&mut rpass, &self.bind_group);
                crate::gpu_core::apply_scissor(
                    &mut rpass,
                    None,
                    self.config.width,
                    self.config.height,
                );
            }

            if regular_end < total {
                self.gpu
                    .draw_raw_range(&mut rpass, &self.bind_group, regular_end..total);
            }

            if self.overlay_text_active
                && let Err(err) =
                    self.text_renderer_overlay
                        .render(&self.atlas, &self.viewport, &mut rpass)
            {
                eprintln!("overlay text render error: {err}");
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        self.atlas.trim();
    }

    pub fn render_frame(&mut self, frame_paint: &FramePaint) {
        let instances = frame_paint.instances();
        if instances.is_empty() {
            return;
        }

        self.gpu
            .upload_instances(&self.device, &self.queue, instances);
        self.gpu
            .plan_batches(frame_paint.batches(), self.config.width, self.config.height);

        let Some(frame) = self.acquire_frame() else {
            return;
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("GUI Batched"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.clear_color),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.gpu.bind_base(&mut rpass, &self.bind_group);

            for batch in frame_paint.regular_batches() {
                self.gpu.draw_batch_immediate(
                    &mut rpass,
                    batch,
                    &self.bind_group,
                    self.config.width,
                    self.config.height,
                );
            }

            crate::gpu_core::apply_scissor(&mut rpass, None, self.config.width, self.config.height);
            if self.regular_text_active {
                if let Err(err) = self
                    .text_renderer
                    .render(&self.atlas, &self.viewport, &mut rpass)
                {
                    eprintln!("text render error: {err}");
                }
                self.gpu.bind_base(&mut rpass, &self.bind_group);
            }

            for batch in frame_paint.overlay_batches() {
                self.gpu.draw_batch_immediate(
                    &mut rpass,
                    batch,
                    &self.bind_group,
                    self.config.width,
                    self.config.height,
                );
            }

            crate::gpu_core::apply_scissor(&mut rpass, None, self.config.width, self.config.height);
            if self.overlay_text_active
                && let Err(err) =
                    self.text_renderer_overlay
                        .render(&self.atlas, &self.viewport, &mut rpass)
            {
                eprintln!("overlay text render error: {err}");
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.atlas.trim();
    }

    pub fn render_frame_layered(&mut self, frame_paint: &FramePaint, text_layers: &[TextLayer]) {
        self.render_frame_layered_inner(
            frame_paint,
            text_layers,
            None::<fn(&Device, &Queue, &mut CommandEncoder, &TextureView)>,
        );
    }

    pub fn render_frame_layered_with_prepass<F>(
        &mut self,
        frame_paint: &FramePaint,
        text_layers: &[TextLayer],
        prepass: F,
    ) where
        F: FnOnce(&Device, &Queue, &mut CommandEncoder, &TextureView),
    {
        self.render_frame_layered_inner(frame_paint, text_layers, Some(prepass));
    }

    fn render_frame_layered_inner<F>(
        &mut self,
        frame_paint: &FramePaint,
        text_layers: &[TextLayer],
        prepass: Option<F>,
    ) where
        F: FnOnce(&Device, &Queue, &mut CommandEncoder, &TextureView),
    {
        let instances = frame_paint.instances();
        let has_prepass = prepass.is_some();
        if instances.is_empty() && !has_prepass {
            return;
        }

        self.gpu
            .upload_instances(&self.device, &self.queue, instances);
        self.gpu
            .plan_batches(frame_paint.batches(), self.config.width, self.config.height);

        let Some(frame) = self.acquire_frame() else {
            return;
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        if let Some(prepass) = prepass {
            prepass(&self.device, &self.queue, &mut encoder, &view);
        }

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("GUI Layered"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: if has_prepass {
                            LoadOp::Load
                        } else {
                            LoadOp::Clear(self.clear_color)
                        },
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.gpu.bind_base(&mut rpass, &self.bind_group);

            let regular_batches = frame_paint.regular_batches();
            let mut batch_index = 0usize;

            for (layer_index, layer) in text_layers.iter().enumerate() {
                while let Some(batch) = regular_batches.get(batch_index) {
                    if batch.range.end > layer.instance_end {
                        break;
                    }
                    self.gpu.draw_batch_immediate(
                        &mut rpass,
                        batch,
                        &self.bind_group,
                        self.config.width,
                        self.config.height,
                    );
                    batch_index += 1;
                }

                if layer.has_text()
                    && layer_index < self.active_text_layers
                    && layer_index < self.text_renderer_layers.len()
                    && self
                        .text_layer_active
                        .get(layer_index)
                        .copied()
                        .unwrap_or(false)
                {
                    crate::gpu_core::apply_scissor(
                        &mut rpass,
                        None,
                        self.config.width,
                        self.config.height,
                    );
                    if let Err(err) = self.text_renderer_layers[layer_index].render(
                        &self.atlas,
                        &self.viewport,
                        &mut rpass,
                    ) {
                        eprintln!("text layer render error: {err}");
                    }
                    self.gpu.bind_base(&mut rpass, &self.bind_group);
                }
            }

            for batch in &regular_batches[batch_index..] {
                self.gpu.draw_batch_immediate(
                    &mut rpass,
                    batch,
                    &self.bind_group,
                    self.config.width,
                    self.config.height,
                );
            }

            for batch in frame_paint.overlay_batches() {
                self.gpu.draw_batch_immediate(
                    &mut rpass,
                    batch,
                    &self.bind_group,
                    self.config.width,
                    self.config.height,
                );
            }

            crate::gpu_core::apply_scissor(&mut rpass, None, self.config.width, self.config.height);
            if self.overlay_text_active
                && let Err(err) =
                    self.text_renderer_overlay
                        .render(&self.atlas, &self.viewport, &mut rpass)
            {
                eprintln!("overlay text render error: {err}");
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.atlas.trim();
    }

    fn acquire_frame(&mut self) -> Option<SurfaceTexture> {
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                Some(frame)
            }
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                let size = winit::dpi::PhysicalSize::new(self.config.width, self.config.height);
                self.resize(size);
                None
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => None,
            CurrentSurfaceTexture::Validation => {
                eprintln!("render error: surface validation failed");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_shader_parses_as_wgsl() {
        let shader = include_str!("../shaders/gui.wgsl");
        let parsed = naga::front::wgsl::parse_str(shader);
        assert!(parsed.is_ok(), "gui shader must be valid WGSL: {parsed:?}");
    }

    #[test]
    fn widget_instance_layout_matches_shader_attributes() {
        assert_eq!(std::mem::size_of::<WidgetInstance>(), 64);
        assert_eq!(std::mem::align_of::<WidgetInstance>(), 4);

        let instance = WidgetInstance::default();
        assert!(instance.pos.iter().all(|value| value.is_finite()));
        assert!(instance.size.iter().all(|value| value.is_finite()));
        assert!(instance.color.iter().all(|value| value.is_finite()));
        assert!(instance.radius.is_finite());
        assert!(instance.mode.is_finite());
        assert!(instance.rotation.is_finite());
    }

    #[test]
    fn opaque_alpha_mode_is_preferred_for_regular_windows() {
        let caps = SurfaceCapabilities {
            alpha_modes: vec![
                CompositeAlphaMode::PreMultiplied,
                CompositeAlphaMode::Opaque,
            ],
            ..SurfaceCapabilities::default()
        };

        assert_eq!(
            select_alpha_mode(&caps, false),
            Some(CompositeAlphaMode::Opaque)
        );
    }

    #[test]
    fn transparent_alpha_mode_is_preferred_for_overlay_windows() {
        let caps = SurfaceCapabilities {
            alpha_modes: vec![
                CompositeAlphaMode::Opaque,
                CompositeAlphaMode::PreMultiplied,
            ],
            ..SurfaceCapabilities::default()
        };

        assert_eq!(
            select_alpha_mode(&caps, true),
            Some(CompositeAlphaMode::PreMultiplied)
        );
    }
}
