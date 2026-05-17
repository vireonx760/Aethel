use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, TextArea, TextBounds};
use std::any::Any;

pub struct Label {
    pub pos: [f32; 2],
    pub text: String,
    pub scale: f32,
    pub color: [f32; 4],
    rect: Rect,
    clip_rect: Option<Rect>,

    cached_text_width: f32,
    cached_text_height: f32,
}

impl Label {
    pub fn new(pos: [f32; 2], text: impl Into<String>) -> Self {
        Self {
            pos,
            text: text.into(),
            scale: 32.0,
            color: [0.9, 0.9, 0.95, 1.0],
            rect: Rect::new(pos[0], pos[1], 100.0, 32.0),
            clip_rect: None,
            cached_text_width: 0.0,
            cached_text_height: 0.0,
        }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self.rect.height = scale * 1.2;
        self
    }

    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    fn estimated_width(&self) -> f32 {
        self.text.chars().count() as f32 * self.scale * 0.60 + 4.0
    }
}

impl Widget for Label {
    fn update(&mut self, _dt: f32, _input: &InputManager) {}

    fn instances(&self) -> Vec<WidgetInstance> {
        vec![]
    }

    fn paint(&self, _ctx: &mut PaintCtx) {}

    fn prepare_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        let lh = self.scale * 1.2;
        let mut buffer = Buffer::new(font_system, Metrics::new(self.scale, lh));
        buffer.set_size(font_system, f32::INFINITY, f32::INFINITY);

        let [r, g, b, a] = self.color;
        buffer.set_text(
            font_system,
            &self.text,
            Attrs::new().family(Family::SansSerif).color(Color::rgba(
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                (a * 255.0) as u8,
            )),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(font_system);

        let mut max_w = 0.0f32;
        let mut line_cnt = 0usize;
        for run in buffer.layout_runs() {
            max_w = max_w.max(run.line_w);
            line_cnt += 1;
        }
        self.cached_text_width = (max_w + 4.0).max(1.0);
        self.cached_text_height = (line_cnt.max(1) as f32 * lh + 4.0).max(1.0);

        buffer.set_size(font_system, self.cached_text_width, self.cached_text_height);
        buffer.shape_until_scroll(font_system);

        buffers.push(buffer);
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if let Some(buffer) = buffers.get(*bi) {
            let [r, g, b, a] = self.color;

            let bounds = if let Some(clip) = self.clip_rect {
                TextBounds {
                    left: clip.x as i32,
                    top: clip.y as i32,
                    right: (clip.x + clip.width) as i32,
                    bottom: (clip.y + clip.height) as i32,
                }
            } else {
                TextBounds {
                    left: self.pos[0] as i32,
                    top: self.pos[1] as i32,
                    right: (self.pos[0] + self.cached_text_width) as i32,
                    bottom: (self.pos[1] + self.cached_text_height) as i32,
                }
            };

            areas.push(TextArea {
                buffer,
                left: self.pos[0],
                top: self.pos[1],
                scale: 1.0,
                bounds,
                default_color: Color::rgba(
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                    (a * 255.0) as u8,
                ),
            });
            *bi += 1;
        }
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let w = if self.cached_text_width > 0.0 {
            self.cached_text_width
        } else {
            self.estimated_width()
        };
        let h = if self.cached_text_height > 0.0 {
            self.cached_text_height
        } else {
            self.scale * 1.2 + 4.0
        };

        let size = constraints.constrain(Size::new(w, h));
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
