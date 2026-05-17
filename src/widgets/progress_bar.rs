use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::binding::F32Signal;
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use std::any::Any;
use std::sync::{Arc, Mutex};

pub struct ProgressBar {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    natural_size: [f32; 2],
    pub progress: f32,
    rect: Rect,
    clip_rect: Option<Rect>,
    binding: Option<Arc<Mutex<f32>>>,
    signal: Option<F32Signal>,
}

impl ProgressBar {
    pub fn new(pos: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            pos,
            size,
            natural_size: size,
            progress: 0.0,
            rect: Rect::new(pos[0], pos[1], size[0], size[1]),
            clip_rect: None,
            binding: None,
            signal: None,
        }
    }

    pub fn bind(mut self, target: Arc<Mutex<f32>>) -> Self {
        if let Ok(value) = target.lock() {
            self.progress = value.clamp(0.0, 1.0);
        }
        self.binding = Some(target);
        self
    }

    pub fn bind_signal(mut self, target: F32Signal) -> Self {
        self.progress = target.get().clamp(0.0, 1.0);
        self.signal = Some(target);
        self
    }

    pub fn set_progress(&mut self, value: f32) {
        self.progress = value.clamp(0.0, 1.0);
        if let Some(signal) = &self.signal {
            signal.set(self.progress);
        }
    }

    #[inline]
    fn clip_info(&self) -> ([f32; 2], [f32; 2], f32) {
        if let Some(clip) = self.clip_rect {
            (
                [clip.x, clip.y],
                [clip.x + clip.width, clip.y + clip.height],
                1.0,
            )
        } else {
            ([0.0; 2], [1e5; 2], 0.0)
        }
    }
}

impl Widget for ProgressBar {
    fn update(&mut self, _dt: f32, _input: &InputManager) {
        if let Some(signal) = &self.signal {
            self.progress = signal.get().clamp(0.0, 1.0);
        }
        if let Some(binding) = &self.binding
            && let Ok(value) = binding.lock()
        {
            self.progress = value.clamp(0.0, 1.0);
        }
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let (clip_min, clip_max, use_clip) = self.clip_info();
        vec![
            WidgetInstance {
                pos: self.pos,
                size: self.size,
                color: [0.12, 0.12, 0.15, 1.0],
                radius: 8.0,
                clip_min,
                clip_max,
                use_clip,
                ..Default::default()
            },
            WidgetInstance {
                pos: self.pos,
                size: [(self.size[0] * self.progress).max(0.0), self.size[1]],
                color: [0.0, 0.65, 0.85, 1.0],
                radius: 8.0,
                clip_min,
                clip_max,
                use_clip,
                ..Default::default()
            },
        ]
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let (clip_min, clip_max, use_clip) = self.clip_info();
        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: [0.12, 0.12, 0.15, 1.0],
            radius: 8.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: [(self.size[0] * self.progress).max(0.0), self.size[1]],
            color: [0.0, 0.65, 0.85, 1.0],
            radius: 8.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let size = constraints.constrain(Size::new(self.natural_size[0], self.natural_size[1]));
        self.size = [size.width, size.height];
        self.rect.width = size.width;
        self.rect.height = size.height;
        size
    }

    fn set_position(&mut self, position: Point) {
        self.pos = [position.x, position.y];
        self.rect.x = position.x;
        self.rect.y = position.y;
    }

    fn set_clip_rect(&mut self, clip: Rect) {
        self.clip_rect = Some(clip);
    }

    fn get_rect(&self) -> Rect {
        self.rect
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
