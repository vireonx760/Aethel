use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::{PaintCtx, ShaderMode};
use crate::gui::shader::CustomShader;
use crate::gui::text::{set_buffer_size, set_buffer_text, shape_text, text_area};
use crate::gui::widget::Widget;
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, TextArea, TextBounds};
use std::any::Any;

#[derive(Debug, Clone)]
pub struct BuiltRect {
    pub rect: Rect,
    pub color: [f32; 4],
    pub radius: f32,
    pub mode: ShaderMode,
    pub overlay: bool,
}

impl BuiltRect {
    pub fn new(rect: Rect, color: [f32; 4]) -> Self {
        Self {
            rect,
            color,
            radius: 0.0,
            mode: ShaderMode::Solid,
            overlay: false,
        }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn shader(mut self, mode: ShaderMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }
}

#[derive(Debug, Clone)]
pub struct BuiltText {
    pub rect: Rect,
    pub text: String,
    pub font_size: f32,
    pub color: [u8; 4],
    pub overlay: bool,
}

impl BuiltText {
    pub fn new(rect: Rect, text: impl Into<String>) -> Self {
        Self {
            rect,
            text: text.into(),
            font_size: 14.0,
            color: [230, 230, 240, 255],
            overlay: false,
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.clamp(8.0, 96.0);
        self
    }

    pub fn color(mut self, color: [u8; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }
}

#[derive(Debug, Clone)]
pub struct BuiltWidget {
    name: String,
    rect: Rect,
    natural_size: Size,
    clip_rect: Option<Rect>,
    rects: Vec<BuiltRect>,
    texts: Vec<BuiltText>,
    custom_shaders: Vec<CustomShader>,
}

impl BuiltWidget {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn custom_shaders(&self) -> &[CustomShader] {
        &self.custom_shaders
    }

    fn absolute_rect(&self, local: Rect) -> Rect {
        Rect::new(
            self.rect.x + local.x,
            self.rect.y + local.y,
            local.width,
            local.height,
        )
    }

    fn clip_info(&self) -> ([f32; 2], [f32; 2], f32) {
        if let Some(clip) = self.clip_rect {
            (
                [clip.x, clip.y],
                [clip.x + clip.width, clip.y + clip.height],
                1.0,
            )
        } else {
            ([0.0, 0.0], [1e5, 1e5], 0.0)
        }
    }

    fn push_rect(&self, ctx: &mut PaintCtx, item: &BuiltRect) {
        let abs = self.absolute_rect(item.rect);
        let (clip_min, clip_max, use_clip) = if item.overlay {
            ([0.0, 0.0], [1e5, 1e5], 0.0)
        } else {
            self.clip_info()
        };
        ctx.push_instance(WidgetInstance {
            pos: [abs.x, abs.y],
            size: [abs.width, abs.height],
            color: item.color,
            radius: item.radius,
            mode: item.mode.as_f32(),
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
    }

    fn text_bounds(&self, text: &BuiltText) -> TextBounds {
        let abs = self.absolute_rect(text.rect);
        let mut left = abs.x as i32;
        let mut top = abs.y as i32;
        let mut right = (abs.x + abs.width) as i32;
        let mut bottom = (abs.y + abs.height) as i32;

        if !text.overlay
            && let Some(clip) = self.clip_rect
        {
            left = left.max(clip.x as i32);
            top = top.max(clip.y as i32);
            right = right.min((clip.x + clip.width) as i32);
            bottom = bottom.min((clip.y + clip.height) as i32);
        }

        TextBounds {
            left,
            top,
            right,
            bottom,
        }
    }
}

impl Widget for BuiltWidget {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let size = constraints.constrain(self.natural_size);
        self.rect.width = size.width;
        self.rect.height = size.height;
        size
    }

    fn set_position(&mut self, position: Point) {
        self.rect.x = position.x;
        self.rect.y = position.y;
    }

    fn get_rect(&self) -> Rect {
        self.rect
    }

    fn set_clip_rect(&mut self, clip: Rect) {
        self.clip_rect = Some(clip);
    }

    fn update(&mut self, _dt: f32, _input: &InputManager) {}

    fn instances(&self) -> Vec<WidgetInstance> {
        let mut out = Vec::with_capacity(self.rects.iter().filter(|rect| !rect.overlay).count());
        let (clip_min, clip_max, use_clip) = self.clip_info();
        for item in self.rects.iter().filter(|rect| !rect.overlay) {
            let abs = self.absolute_rect(item.rect);
            out.push(WidgetInstance {
                pos: [abs.x, abs.y],
                size: [abs.width, abs.height],
                color: item.color,
                radius: item.radius,
                mode: item.mode.as_f32(),
                clip_min,
                clip_max,
                use_clip,
                ..Default::default()
            });
        }
        out
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        for item in self.rects.iter().filter(|rect| !rect.overlay) {
            self.push_rect(ctx, item);
        }
    }

    fn overlay_instances(&self) -> Vec<WidgetInstance> {
        let mut out = Vec::with_capacity(self.rects.iter().filter(|rect| rect.overlay).count());
        for item in self.rects.iter().filter(|rect| rect.overlay) {
            let abs = self.absolute_rect(item.rect);
            out.push(WidgetInstance {
                pos: [abs.x, abs.y],
                size: [abs.width, abs.height],
                color: item.color,
                radius: item.radius,
                mode: item.mode.as_f32(),
                use_clip: 0.0,
                ..Default::default()
            });
        }
        out
    }

    fn paint_overlay(&self, ctx: &mut PaintCtx) {
        for item in self.rects.iter().filter(|rect| rect.overlay) {
            self.push_rect(ctx, item);
        }
    }

    fn prepare_text_buffers(&mut self, fs: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        for item in self.texts.iter().filter(|text| !text.overlay) {
            push_buffer(fs, buffers, item);
        }
    }

    fn overlay_text_buffers(&mut self, fs: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        for item in self.texts.iter().filter(|text| text.overlay) {
            push_buffer(fs, buffers, item);
        }
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        for item in self.texts.iter().filter(|text| !text.overlay) {
            if let Some(buffer) = buffers.get(*bi) {
                push_area(
                    buffer,
                    self.absolute_rect(item.rect),
                    self.text_bounds(item),
                    item,
                    areas,
                );
                *bi += 1;
            }
        }
    }

    fn overlay_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        for item in self.texts.iter().filter(|text| text.overlay) {
            if let Some(buffer) = buffers.get(*bi) {
                push_area(
                    buffer,
                    self.absolute_rect(item.rect),
                    self.text_bounds(item),
                    item,
                    areas,
                );
                *bi += 1;
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn custom_shaders(&self) -> &[CustomShader] {
        &self.custom_shaders
    }
}

pub struct WidgetBuilder {
    name: String,
    size: Size,
    rects: Vec<BuiltRect>,
    texts: Vec<BuiltText>,
    custom_shaders: Vec<CustomShader>,
}

impl WidgetBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: Size::new(64.0, 32.0),
            rects: Vec::new(),
            texts: Vec::new(),
            custom_shaders: Vec::new(),
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Size::new(width.max(0.0), height.max(0.0));
        self
    }

    pub fn rect(mut self, rect: BuiltRect) -> Self {
        self.rects.push(rect);
        self
    }

    pub fn text(mut self, text: BuiltText) -> Self {
        self.texts.push(text);
        self
    }

    pub fn shader(mut self, shader: CustomShader) -> Self {
        self.custom_shaders.push(shader);
        self
    }

    pub fn build(self) -> BuiltWidget {
        BuiltWidget {
            name: self.name,
            rect: Rect::new(0.0, 0.0, self.size.width, self.size.height),
            natural_size: self.size,
            clip_rect: None,
            rects: self.rects,
            texts: self.texts,
            custom_shaders: self.custom_shaders,
        }
    }
}

fn push_buffer(fs: &mut FontSystem, buffers: &mut Vec<Buffer>, item: &BuiltText) {
    let mut buffer = Buffer::new(fs, Metrics::new(item.font_size, item.font_size * 1.25));
    set_buffer_size(
        &mut buffer,
        fs,
        item.rect.width.max(1.0),
        item.rect.height.max(1.0),
    );
    set_buffer_text(
        &mut buffer,
        fs,
        &item.text,
        Attrs::new().family(Family::SansSerif).color(Color::rgba(
            item.color[0],
            item.color[1],
            item.color[2],
            item.color[3],
        )),
    );
    shape_text(&mut buffer, fs);
    buffers.push(buffer);
}

fn push_area<'a>(
    buffer: &'a Buffer,
    abs: Rect,
    bounds: TextBounds,
    item: &BuiltText,
    areas: &mut Vec<TextArea<'a>>,
) {
    areas.push(text_area(
        buffer,
        abs.x,
        abs.y + ((abs.height - item.font_size * 1.25).max(0.0) * 0.5),
        bounds,
        Color::rgba(item.color[0], item.color[1], item.color[2], item.color[3]),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_emits_custom_shader_mode() {
        let widget = WidgetBuilder::new("glow")
            .size(80.0, 24.0)
            .rect(
                BuiltRect::new(Rect::new(0.0, 0.0, 80.0, 24.0), [1.0, 0.0, 0.0, 1.0])
                    .shader(ShaderMode::Custom(16.0)),
            )
            .build();

        let instances = widget.instances();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].mode, 16.0);
    }

    #[test]
    fn builder_offsets_rects_by_widget_position() {
        let mut widget = WidgetBuilder::new("offset")
            .size(80.0, 24.0)
            .rect(BuiltRect::new(
                Rect::new(4.0, 5.0, 10.0, 11.0),
                [1.0, 1.0, 1.0, 1.0],
            ))
            .build();
        widget.set_position(Point::new(20.0, 30.0));

        let instances = widget.instances();
        assert_eq!(instances[0].pos, [24.0, 35.0]);
    }
}
