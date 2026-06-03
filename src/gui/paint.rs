use crate::core::renderer::WidgetInstance;
use crate::gui::clip::{
    ClipRect, ClipStack, ClipToken, InstanceClip, ScissorRect, instance_bounds, sanitize_instance,
};
use std::ops::Range;

pub const FIRST_CUSTOM_SHADER_MODE: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderLayer {
    Regular,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPolicy {
    InheritClip,
    EscapeClip,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaderMode {
    Solid,
    HsvSquare,
    HueBar,
    Custom(f32),
}

impl ShaderMode {
    #[inline]
    pub fn as_f32(self) -> f32 {
        match self {
            Self::Solid => 0.0,
            Self::HsvSquare => 1.0,
            Self::HueBar => 2.0,
            Self::Custom(v) => v,
        }
    }

    #[inline]
    pub fn custom_key(self) -> Option<u32> {
        custom_shader_key(self.as_f32())
    }
}

#[inline(always)]
pub fn custom_shader_key(mode: f32) -> Option<u32> {
    if mode < FIRST_CUSTOM_SHADER_MODE {
        None
    } else if mode.is_finite() {
        Some(mode.round() as u32)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PaintRect {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub radius: f32,
    pub mode: ShaderMode,
}

impl PaintRect {
    #[inline(always)]
    pub fn new(pos: [f32; 2], size: [f32; 2], color: [f32; 4]) -> Self {
        Self {
            pos,
            size,
            color,
            radius: 0.0,
            mode: ShaderMode::Solid,
        }
    }

    #[inline(always)]
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    #[inline(always)]
    pub fn mode(mut self, mode: ShaderMode) -> Self {
        self.mode = mode;
        self
    }

    #[inline(always)]
    pub fn into_instance(self) -> WidgetInstance {
        WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: self.color,
            radius: self.radius,
            mode: self.mode.as_f32(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderBatch {
    pub layer: RenderLayer,
    pub range: Range<u32>,
    pub clip: Option<ClipRect>,
    pub shader_key: Option<u32>,
}

impl RenderBatch {
    #[inline]
    pub fn scissor(&self, width: u32, height: u32) -> Option<ScissorRect> {
        self.clip.and_then(|clip| clip.scissor(width, height))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaintStats {
    pub pushed_instances: usize,
    pub skipped_instances: usize,
    pub culled_by_clip: usize,
    pub regular_batches: usize,
    pub overlay_batches: usize,
    pub instance_capacity_growths: usize,
    pub max_clip_depth: usize,
}

#[derive(Debug, Clone)]
pub struct FramePaint {
    instances: Vec<WidgetInstance>,
    batches: Vec<RenderBatch>,
    regular_count: usize,
    stats: PaintStats,
    last_batch_key: Option<BatchKey>,
}

impl FramePaint {
    pub fn new() -> Self {
        Self::with_capacity(512, 64)
    }

    pub fn with_capacity(instance_capacity: usize, batch_capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(instance_capacity),
            batches: Vec::with_capacity(batch_capacity),
            regular_count: 0,
            stats: PaintStats::default(),
            last_batch_key: None,
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.instances.clear();
        self.batches.clear();
        self.regular_count = 0;
        self.stats = PaintStats::default();
        self.last_batch_key = None;
    }

    #[inline]
    pub fn flush_batch(&mut self) {
        self.last_batch_key = None;
    }

    #[inline]
    pub fn instances(&self) -> &[WidgetInstance] {
        &self.instances
    }

    #[inline]
    pub fn batches(&self) -> &[RenderBatch] {
        &self.batches
    }

    #[inline]
    pub fn regular_batches(&self) -> &[RenderBatch] {
        let end = self
            .batches
            .iter()
            .position(|batch| batch.layer == RenderLayer::Overlay)
            .unwrap_or(self.batches.len());
        &self.batches[..end]
    }

    #[inline]
    pub fn overlay_batches(&self) -> &[RenderBatch] {
        let start = self
            .batches
            .iter()
            .position(|batch| batch.layer == RenderLayer::Overlay)
            .unwrap_or(self.batches.len());
        &self.batches[start..]
    }

    #[inline]
    pub fn regular_count(&self) -> usize {
        self.regular_count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    #[inline]
    pub fn stats(&self) -> &PaintStats {
        &self.stats
    }

    #[inline]
    pub fn instance_capacity(&self) -> usize {
        self.instances.capacity()
    }

    #[inline]
    pub fn batch_capacity(&self) -> usize {
        self.batches.capacity()
    }

    #[inline(always)]
    fn push(&mut self, layer: RenderLayer, clip: Option<ClipRect>, instance: WidgetInstance) {
        if self.instances.len() == self.instances.capacity() {
            self.stats.instance_capacity_growths += 1;
        }

        let index = self.instances.len() as u32;
        self.instances.push(instance);
        self.stats.pushed_instances += 1;

        if layer == RenderLayer::Regular {
            self.regular_count = self.instances.len();
        }

        let shader_key = custom_shader_key(instance.mode);
        let key = BatchKey {
            layer,
            clip,
            shader_key,
        };
        if self.last_batch_key == Some(key) {
            if let Some(last) = self.batches.last_mut() {
                last.range.end = index + 1;
            }
        } else {
            self.batches.push(RenderBatch {
                layer,
                range: index..index + 1,
                clip,
                shader_key,
            });
            self.last_batch_key = Some(key);
            match layer {
                RenderLayer::Regular => self.stats.regular_batches += 1,
                RenderLayer::Overlay => self.stats.overlay_batches += 1,
            }
        }
    }
}

impl Default for FramePaint {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BatchKey {
    layer: RenderLayer,
    clip: Option<ClipRect>,
    shader_key: Option<u32>,
}

pub struct PaintCtx<'a> {
    frame: &'a mut FramePaint,
    clips: ClipStack,
    layer: RenderLayer,
    overlay_policy: OverlayPolicy,
}

impl<'a> PaintCtx<'a> {
    pub fn new(frame: &'a mut FramePaint) -> Self {
        Self {
            frame,
            clips: ClipStack::with_capacity(8),
            layer: RenderLayer::Regular,
            overlay_policy: OverlayPolicy::EscapeClip,
        }
    }

    #[inline]
    pub fn clear_clip_stack(&mut self) {
        self.clips.clear();
    }

    #[inline]
    pub fn layer(&self) -> RenderLayer {
        self.layer
    }

    #[inline]
    pub fn set_layer(&mut self, layer: RenderLayer) -> RenderLayer {
        let prev = self.layer;
        self.layer = layer;
        prev
    }

    #[inline]
    pub fn set_overlay_policy(&mut self, policy: OverlayPolicy) -> OverlayPolicy {
        let prev = self.overlay_policy;
        self.overlay_policy = policy;
        prev
    }

    #[inline]
    pub fn current_clip(&self) -> Option<ClipRect> {
        self.clips.current()
    }

    #[inline]
    pub fn push_clip_rect(&mut self, clip: ClipRect) -> Option<ClipToken> {
        let token = self.clips.push(clip);
        self.frame.stats.max_clip_depth = self.frame.stats.max_clip_depth.max(self.clips.len());
        token
    }

    #[inline]
    pub fn pop_clip(&mut self, token: ClipToken) {
        self.clips.pop(token);
    }

    #[inline(always)]
    pub fn push_rect(&mut self, rect: PaintRect) {
        self.push_instance(rect.into_instance());
    }

    pub fn push_instances(&mut self, instances: &[WidgetInstance]) {
        for instance in instances {
            self.push_instance(*instance);
        }
    }

    #[inline]
    pub fn flush_batch(&mut self) {
        self.frame.flush_batch();
    }

    #[inline]
    pub fn instance_len(&self) -> usize {
        self.frame.instances().len()
    }

    #[inline(always)]
    pub fn push_instance(&mut self, mut instance: WidgetInstance) {
        if !sanitize_instance(&mut instance) {
            self.frame.stats.skipped_instances += 1;
            return;
        }

        let overlay_escapes_clip =
            self.layer == RenderLayer::Overlay && self.overlay_policy == OverlayPolicy::EscapeClip;
        if instance.use_clip <= 0.5 && (self.clips.is_empty() || overlay_escapes_clip) {
            instance.use_clip = 0.0;
            self.frame.push(self.layer, None, instance);
            return;
        }

        let instance_clip = InstanceClip::from_instance(&instance);
        let resolved_clip = if overlay_escapes_clip && instance_clip.rect.is_none() {
            Some(None)
        } else {
            self.clips.resolve(instance_clip)
        };

        let Some(clip) = resolved_clip else {
            self.frame.stats.culled_by_clip += 1;
            return;
        };

        if let Some(clip) = clip {
            let Some(bounds) = instance_bounds(&instance) else {
                self.frame.stats.skipped_instances += 1;
                return;
            };

            if !clip.intersects(bounds) {
                self.frame.stats.culled_by_clip += 1;
                return;
            }

            clip.apply_to_instance(&mut instance);
        } else {
            instance.use_clip = 0.0;
        }

        self.frame.push(self.layer, clip, instance);
    }
}

pub trait PaintExt {
    fn solid(self, color: [f32; 4]) -> PaintRect;
}

impl PaintExt for ([f32; 2], [f32; 2]) {
    #[inline]
    fn solid(self, color: [f32; 4]) -> PaintRect {
        PaintRect::new(self.0, self.1, color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_ctx_culls_instances_outside_clip() {
        let mut frame = FramePaint::new();
        let mut ctx = PaintCtx::new(&mut frame);
        let clip = ClipRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let Some(token) = ctx.push_clip_rect(clip) else {
            unreachable!("valid root clip should push");
        };
        ctx.push_rect(PaintRect::new([20.0, 20.0], [5.0, 5.0], [1.0; 4]));
        ctx.pop_clip(token);

        assert_eq!(frame.instances().len(), 0);
        assert_eq!(frame.stats().culled_by_clip, 1);
    }

    #[test]
    fn overlay_rect_escapes_clip_by_default() {
        let mut frame = FramePaint::new();
        let mut ctx = PaintCtx::new(&mut frame);
        let Some(token) = ctx.push_clip_rect(ClipRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        }) else {
            unreachable!("valid root clip should push");
        };

        ctx.set_layer(RenderLayer::Overlay);
        ctx.push_rect(PaintRect::new([20.0, 20.0], [5.0, 5.0], [1.0; 4]));
        ctx.pop_clip(token);

        assert_eq!(frame.instances().len(), 1);
        assert_eq!(frame.instances()[0].use_clip, 0.0);
        assert_eq!(frame.overlay_batches().len(), 1);
    }

    #[test]
    fn paint_ctx_groups_same_clip_into_one_batch() {
        let mut frame = FramePaint::new();
        let mut ctx = PaintCtx::new(&mut frame);
        let clip = ClipRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let Some(token) = ctx.push_clip_rect(clip) else {
            unreachable!("valid root clip should push");
        };
        ctx.push_rect(PaintRect::new([0.0, 0.0], [5.0, 5.0], [1.0; 4]));
        ctx.push_rect(PaintRect::new([10.0, 10.0], [5.0, 5.0], [1.0; 4]));
        ctx.pop_clip(token);

        assert_eq!(frame.instances().len(), 2);
        assert_eq!(frame.batches().len(), 1);
        assert_eq!(frame.batches()[0].range, 0..2);
    }

    #[test]
    fn paint_ctx_splits_batches_by_custom_shader_mode() {
        let mut frame = FramePaint::new();
        let mut ctx = PaintCtx::new(&mut frame);
        ctx.push_rect(PaintRect::new([0.0, 0.0], [5.0, 5.0], [1.0; 4]));
        ctx.push_rect(
            PaintRect::new([10.0, 0.0], [5.0, 5.0], [1.0; 4])
                .mode(ShaderMode::Custom(FIRST_CUSTOM_SHADER_MODE)),
        );
        ctx.push_rect(PaintRect::new([20.0, 0.0], [5.0, 5.0], [1.0; 4]));

        assert_eq!(frame.batches().len(), 3);
        assert_eq!(frame.batches()[0].shader_key, None);
        assert_eq!(
            frame.batches()[1].shader_key,
            Some(FIRST_CUSTOM_SHADER_MODE as u32)
        );
        assert_eq!(frame.batches()[2].shader_key, None);
    }

    #[test]
    fn custom_shader_key_accepts_only_custom_finite_modes() {
        assert_eq!(custom_shader_key(0.0), None);
        assert_eq!(
            custom_shader_key(FIRST_CUSTOM_SHADER_MODE),
            Some(FIRST_CUSTOM_SHADER_MODE as u32)
        );
        assert_eq!(custom_shader_key(f32::INFINITY), None);
        assert_eq!(custom_shader_key(f32::NAN), None);
    }
}
