use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::binding::{F32Signal, I32Signal, U32Signal};
use crate::gui::command::{CommandId, UpdateCtx};
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use glam::Vec2;
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, TextArea, TextBounds};
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use winit::event::MouseButton;

const WIDGET_H: f32 = 60.0;
const LABEL_Y: f32 = 4.0;
const TRACK_CY_OFFSET: f32 = 42.0;
const KNOB_R: f32 = 12.0;
const KNOB_H: f32 = 30.0;

#[derive(Clone, Copy)]
pub enum SliderValue {
    F32(f32, f32, f32),
    I32(i32, i32, i32),
    U32(u32, u32, u32),
}

pub type SliderCallbackF32 = Arc<Mutex<dyn FnMut(f32) + Send + Sync>>;
pub type SliderCallbackI32 = Arc<Mutex<dyn FnMut(i32) + Send + Sync>>;
pub type SliderCallbackU32 = Arc<Mutex<dyn FnMut(u32) + Send + Sync>>;

pub struct SliderLabeled {
    pub pos: [f32; 2],
    pub length: f32,
    natural_length: f32,
    pub track_height: f32,
    pub value: SliderValue,
    pub label: String,
    pub suffix: String,
    pub dragging: bool,
    pub hovered: bool,
    rect: Rect,
    clip_rect: Option<Rect>,

    binding_f32: Option<Arc<Mutex<f32>>>,
    binding_i32: Option<Arc<Mutex<i32>>>,
    binding_u32: Option<Arc<Mutex<u32>>>,
    signal_f32: Option<F32Signal>,
    signal_i32: Option<I32Signal>,
    signal_u32: Option<U32Signal>,

    on_change_f32: Option<SliderCallbackF32>,
    on_change_i32: Option<SliderCallbackI32>,
    on_change_u32: Option<SliderCallbackU32>,
    on_change_f32_cmd: Option<CommandId<f32>>,
    on_change_i32_cmd: Option<CommandId<i32>>,
    on_change_u32_cmd: Option<CommandId<u32>>,
}

impl SliderLabeled {
    pub fn new_f32(
        pos: [f32; 2],
        length: f32,
        label: impl Into<String>,
        min: f32,
        max: f32,
        initial: f32,
    ) -> Self {
        Self::new_inner(
            pos,
            length,
            label,
            SliderValue::F32(initial.clamp(min, max), min, max),
        )
    }

    pub fn new_i32(
        pos: [f32; 2],
        length: f32,
        label: impl Into<String>,
        min: i32,
        max: i32,
        initial: i32,
    ) -> Self {
        Self::new_inner(
            pos,
            length,
            label,
            SliderValue::I32(initial.clamp(min, max), min, max),
        )
    }

    pub fn new_u32(
        pos: [f32; 2],
        length: f32,
        label: impl Into<String>,
        min: u32,
        max: u32,
        initial: u32,
    ) -> Self {
        Self::new_inner(
            pos,
            length,
            label,
            SliderValue::U32(initial.clamp(min, max), min, max),
        )
    }

    fn new_inner(pos: [f32; 2], length: f32, label: impl Into<String>, value: SliderValue) -> Self {
        Self {
            pos,
            length,
            natural_length: length,
            track_height: 8.0,
            value,
            label: label.into(),
            suffix: String::new(),
            dragging: false,
            hovered: false,
            rect: Rect::new(pos[0], pos[1], length, WIDGET_H),
            clip_rect: None,
            binding_f32: None,
            binding_i32: None,
            binding_u32: None,
            signal_f32: None,
            signal_i32: None,
            signal_u32: None,
            on_change_f32: None,
            on_change_i32: None,
            on_change_u32: None,
            on_change_f32_cmd: None,
            on_change_i32_cmd: None,
            on_change_u32_cmd: None,
        }
    }

    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }

    pub fn bind_f32(mut self, target: Arc<Mutex<f32>>) -> Self {
        if let SliderValue::F32(_, min, max) = self.value
            && let Ok(val) = target.lock()
        {
            self.value = SliderValue::F32((*val).clamp(min, max), min, max);
        }
        self.binding_f32 = Some(target);
        self
    }

    pub fn bind_i32(mut self, target: Arc<Mutex<i32>>) -> Self {
        if let SliderValue::I32(_, min, max) = self.value
            && let Ok(val) = target.lock()
        {
            self.value = SliderValue::I32((*val).clamp(min, max), min, max);
        }
        self.binding_i32 = Some(target);
        self
    }

    pub fn bind_u32(mut self, target: Arc<Mutex<u32>>) -> Self {
        if let SliderValue::U32(_, min, max) = self.value
            && let Ok(val) = target.lock()
        {
            self.value = SliderValue::U32((*val).clamp(min, max), min, max);
        }
        self.binding_u32 = Some(target);
        self
    }

    pub fn bind_f32_signal(mut self, target: F32Signal) -> Self {
        if let SliderValue::F32(_, min, max) = self.value {
            self.value = SliderValue::F32(target.get().clamp(min, max), min, max);
        }
        self.signal_f32 = Some(target);
        self
    }

    pub fn bind_i32_signal(mut self, target: I32Signal) -> Self {
        if let SliderValue::I32(_, min, max) = self.value {
            self.value = SliderValue::I32(target.get().clamp(min, max), min, max);
        }
        self.signal_i32 = Some(target);
        self
    }

    pub fn bind_u32_signal(mut self, target: U32Signal) -> Self {
        if let SliderValue::U32(_, min, max) = self.value {
            self.value = SliderValue::U32(target.get().clamp(min, max), min, max);
        }
        self.signal_u32 = Some(target);
        self
    }

    pub fn on_change_f32<F: FnMut(f32) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_change_f32 = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn on_change_i32<F: FnMut(i32) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_change_i32 = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn on_change_u32<F: FnMut(u32) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_change_u32 = Some(Arc::new(Mutex::new(f)));
        self
    }

    pub fn on_change_f32_cmd(mut self, command: CommandId<f32>) -> Self {
        self.on_change_f32_cmd = Some(command);
        self
    }

    pub fn on_change_i32_cmd(mut self, command: CommandId<i32>) -> Self {
        self.on_change_i32_cmd = Some(command);
        self
    }

    pub fn on_change_u32_cmd(mut self, command: CommandId<u32>) -> Self {
        self.on_change_u32_cmd = Some(command);
        self
    }

    pub fn get_f32(&self) -> Option<f32> {
        if let SliderValue::F32(v, ..) = self.value {
            Some(v)
        } else {
            None
        }
    }
    pub fn get_i32(&self) -> Option<i32> {
        if let SliderValue::I32(v, ..) = self.value {
            Some(v)
        } else {
            None
        }
    }
    pub fn get_u32(&self) -> Option<u32> {
        if let SliderValue::U32(v, ..) = self.value {
            Some(v)
        } else {
            None
        }
    }

    fn get_norm_value(&self) -> f32 {
        match self.value {
            SliderValue::F32(v, min, max) => (v - min) / (max - min),
            SliderValue::I32(v, min, max) => (v - min) as f32 / (max - min) as f32,
            SliderValue::U32(v, min, max) => (v - min) as f32 / (max - min) as f32,
        }
    }

    fn set_from_norm(&mut self, norm: f32) {
        let n = norm.clamp(0.0, 1.0);
        match self.value {
            SliderValue::F32(_, min, max) => {
                let v = min + n * (max - min);
                self.value = SliderValue::F32(v, min, max);
                self.sync_binding_f32();
                self.trigger_callback_f32(v);
            }
            SliderValue::I32(_, min, max) => {
                let v = min + (n * (max - min) as f32).round() as i32;
                self.value = SliderValue::I32(v.clamp(min, max), min, max);
                self.sync_binding_i32();
                self.trigger_callback_i32(v);
            }
            SliderValue::U32(_, min, max) => {
                let v = min + (n * (max - min) as f32).round() as u32;
                self.value = SliderValue::U32(v.clamp(min, max), min, max);
                self.sync_binding_u32();
                self.trigger_callback_u32(v);
            }
        }
    }

    fn sync_binding_f32(&self) {
        if let SliderValue::F32(v, ..) = self.value {
            if let Some(signal) = &self.signal_f32 {
                signal.set(v);
            }
            if let Some(b) = &self.binding_f32
                && let Ok(mut g) = b.lock()
            {
                *g = v;
            }
        }
    }
    fn sync_binding_i32(&self) {
        if let SliderValue::I32(v, ..) = self.value {
            if let Some(signal) = &self.signal_i32 {
                signal.set(v);
            }
            if let Some(b) = &self.binding_i32
                && let Ok(mut g) = b.lock()
            {
                *g = v;
            }
        }
    }
    fn sync_binding_u32(&self) {
        if let SliderValue::U32(v, ..) = self.value {
            if let Some(signal) = &self.signal_u32 {
                signal.set(v);
            }
            if let Some(b) = &self.binding_u32
                && let Ok(mut g) = b.lock()
            {
                *g = v;
            }
        }
    }

    fn trigger_callback_f32(&self, v: f32) {
        if let Some(cb) = &self.on_change_f32
            && let Ok(mut f) = cb.lock()
        {
            f(v);
        }
    }
    fn trigger_callback_i32(&self, v: i32) {
        if let Some(cb) = &self.on_change_i32
            && let Ok(mut f) = cb.lock()
        {
            f(v);
        }
    }
    fn trigger_callback_u32(&self, v: u32) {
        if let Some(cb) = &self.on_change_u32
            && let Ok(mut f) = cb.lock()
        {
            f(v);
        }
    }

    fn format_value(&self) -> String {
        match self.value {
            SliderValue::F32(v, ..) => format!("{:.2}{}", v, self.suffix),
            SliderValue::I32(v, ..) => format!("{}{}", v, self.suffix),
            SliderValue::U32(v, ..) => format!("{}{}", v, self.suffix),
        }
    }

    #[inline]
    fn track_cy(&self) -> f32 {
        self.pos[1] + TRACK_CY_OFFSET
    }

    #[inline]
    fn effective_len(&self) -> f32 {
        (self.length - KNOB_R * 2.0).max(0.0)
    }

    #[inline]
    fn knob_cx(&self) -> f32 {
        self.pos[0] + KNOB_R + self.get_norm_value() * self.effective_len()
    }

    fn tooltip_width_for(text: &str) -> f32 {
        (text.chars().count() as f32 * 8.0 + 24.0).clamp(56.0, 180.0)
    }

    fn tooltip_rect(&self) -> Rect {
        let text = self.format_value();
        let width = Self::tooltip_width_for(&text);
        let knob_cx = self.knob_cx();
        Rect::new(knob_cx - width * 0.5, self.pos[1] - 10.0, width, 28.0)
    }

    #[inline]
    fn tooltip_visible(&self) -> bool {
        self.hovered
    }
}

impl Widget for SliderLabeled {
    fn update(&mut self, _dt: f32, input: &InputManager) {
        if !self.dragging {
            match self.value {
                SliderValue::F32(_, min, max) => {
                    if let Some(signal) = &self.signal_f32 {
                        self.value = SliderValue::F32(signal.get().clamp(min, max), min, max);
                    }
                }
                SliderValue::I32(_, min, max) => {
                    if let Some(signal) = &self.signal_i32 {
                        self.value = SliderValue::I32(signal.get().clamp(min, max), min, max);
                    }
                }
                SliderValue::U32(_, min, max) => {
                    if let Some(signal) = &self.signal_u32 {
                        self.value = SliderValue::U32(signal.get().clamp(min, max), min, max);
                    }
                }
            }
        }

        let knob_cx = self.knob_cx();
        let cy = self.track_cy();

        let knob_min = Vec2::new(knob_cx - KNOB_R, cy - KNOB_H * 0.5);
        let knob_max = Vec2::new(knob_cx + KNOB_R, cy + KNOB_H * 0.5);

        let mouse_pressed = input.is_mouse_down(MouseButton::Left);
        let mouse_in_knob =
            input.mouse_pos.cmpge(knob_min).all() && input.mouse_pos.cmplt(knob_max).all();

        self.hovered = mouse_in_knob;

        if mouse_pressed && mouse_in_knob && !self.dragging {
            self.dragging = true;
        }

        if self.dragging {
            if mouse_pressed {
                let eff = self.effective_len();
                if eff > 0.0 {
                    let norm = (input.mouse_pos.x - self.pos[0] - KNOB_R) / eff;
                    self.set_from_norm(norm);
                }
            } else {
                self.dragging = false;
            }
        }
    }

    fn update_ctx(&mut self, dt: f32, input: &InputManager, ctx: &mut UpdateCtx) {
        let before = self.value;
        self.update(dt, input);
        match (before, self.value) {
            (SliderValue::F32(old, ..), SliderValue::F32(new, ..))
                if (old - new).abs() > f32::EPSILON =>
            {
                if let Some(command) = self.on_change_f32_cmd {
                    ctx.emit_f32(command, new);
                }
            }
            (SliderValue::I32(old, ..), SliderValue::I32(new, ..)) if old != new => {
                if let Some(command) = self.on_change_i32_cmd {
                    ctx.emit_i32(command, new);
                }
            }
            (SliderValue::U32(old, ..), SliderValue::U32(new, ..)) if old != new => {
                if let Some(command) = self.on_change_u32_cmd {
                    ctx.emit_u32(command, new);
                }
            }
            _ => {}
        }
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let (clip_min, clip_max, use_clip) = clip_info(self.clip_rect);

        let norm = self.get_norm_value();
        let cy = self.track_cy();
        let th = self.track_height;
        let knob_cx = self.knob_cx();

        let track = WidgetInstance {
            pos: [self.pos[0], cy - th * 0.5],
            size: [self.length, th],
            color: [0.12, 0.12, 0.15, 1.0],
            radius: th * 0.5,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        };

        let fill = WidgetInstance {
            pos: [self.pos[0], cy - th * 0.5],
            size: [KNOB_R + norm * self.effective_len(), th],
            color: [0.0, 0.65, 0.85, 1.0],
            radius: th * 0.5,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        };

        let knob = WidgetInstance {
            pos: [knob_cx - KNOB_R, cy - KNOB_H * 0.5],
            size: [KNOB_R * 2.0, KNOB_H],
            color: if self.hovered {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                [0.9, 0.9, 0.95, 1.0]
            },
            radius: 6.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        };

        vec![track, fill, knob]
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let (clip_min, clip_max, use_clip) = clip_info(self.clip_rect);
        let norm = self.get_norm_value();
        let cy = self.track_cy();
        let th = self.track_height;
        let knob_cx = self.knob_cx();

        ctx.push_instance(WidgetInstance {
            pos: [self.pos[0], cy - th * 0.5],
            size: [self.length, th],
            color: [0.12, 0.12, 0.15, 1.0],
            radius: th * 0.5,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
        ctx.push_instance(WidgetInstance {
            pos: [self.pos[0], cy - th * 0.5],
            size: [KNOB_R + norm * self.effective_len(), th],
            color: [0.0, 0.65, 0.85, 1.0],
            radius: th * 0.5,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
        ctx.push_instance(WidgetInstance {
            pos: [knob_cx - KNOB_R, cy - KNOB_H * 0.5],
            size: [KNOB_R * 2.0, KNOB_H],
            color: if self.hovered {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                [0.9, 0.9, 0.95, 1.0]
            },
            radius: 6.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
    }

    fn overlay_instances(&self) -> Vec<WidgetInstance> {
        if !self.tooltip_visible() {
            return vec![];
        }
        let rect = self.tooltip_rect();
        vec![WidgetInstance {
            pos: [rect.x, rect.y],
            size: [rect.width, rect.height],
            color: [0.15, 0.15, 0.20, 0.97],
            radius: 6.0,
            use_clip: 0.0,
            ..Default::default()
        }]
    }

    fn paint_overlay(&self, ctx: &mut PaintCtx) {
        if !self.tooltip_visible() {
            return;
        }
        let rect = self.tooltip_rect();
        ctx.push_instance(WidgetInstance {
            pos: [rect.x, rect.y],
            size: [rect.width, rect.height],
            color: [0.15, 0.15, 0.20, 0.97],
            radius: 6.0,
            use_clip: 0.0,
            ..Default::default()
        });
    }

    fn prepare_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        let mut label_buf = Buffer::new(font_system, Metrics::new(14.0, 18.0));
        label_buf.set_size(font_system, self.length, 20.0);
        label_buf.set_text(
            font_system,
            &self.label,
            Attrs::new()
                .family(Family::SansSerif)
                .color(Color::rgb(180, 180, 190)),
            Shaping::Advanced,
        );
        label_buf.shape_until_scroll(font_system);
        buffers.push(label_buf);
    }

    fn overlay_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        if !self.tooltip_visible() {
            return;
        }
        let text = self.format_value();
        let width = Self::tooltip_width_for(&text);
        let mut val_buf = Buffer::new(font_system, Metrics::new(13.0, 16.0));
        val_buf.set_size(font_system, width - 12.0, 20.0);
        val_buf.set_text(
            font_system,
            &text,
            Attrs::new()
                .family(Family::Monospace)
                .color(Color::rgb(50, 200, 255)),
            Shaping::Advanced,
        );
        val_buf.shape_until_scroll(font_system);
        buffers.push(val_buf);
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if let Some(buf) = buffers.get(*bi) {
            let left = self.pos[0];
            let top = self.pos[1] + LABEL_Y;

            let bounds = if let Some(clip) = self.clip_rect {
                TextBounds {
                    left: (left as i32).max(clip.x as i32),
                    top: (top as i32).max(clip.y as i32),
                    right: ((left + self.length) as i32).min((clip.x + clip.width) as i32),
                    bottom: ((top + 20.0) as i32).min((clip.y + clip.height) as i32),
                }
            } else {
                TextBounds {
                    left: left as i32,
                    top: top as i32,
                    right: (left + self.length) as i32,
                    bottom: (top + 20.0) as i32,
                }
            };

            areas.push(TextArea {
                buffer: buf,
                left,
                top,
                scale: 1.0,
                bounds,
                default_color: Color::rgb(180, 180, 190),
            });
            *bi += 1;
        }
    }

    fn overlay_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if !self.tooltip_visible() {
            return;
        }
        if let Some(buf) = buffers.get(*bi) {
            let rect = self.tooltip_rect();

            let mut text_w = 0.0f32;
            for run in buf.layout_runs() {
                text_w = text_w.max(run.line_w);
            }
            let left = rect.x + (rect.width - text_w).max(0.0) * 0.5;
            let top = rect.y + 6.0;

            areas.push(TextArea {
                buffer: buf,
                left,
                top,
                scale: 1.0,
                bounds: TextBounds {
                    left: rect.x as i32,
                    top: rect.y as i32,
                    right: (rect.x + rect.width) as i32,
                    bottom: (rect.y + rect.height) as i32,
                },
                default_color: Color::rgb(50, 200, 255),
            });
            *bi += 1;
        }
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let size = constraints.constrain(Size::new(self.natural_length, WIDGET_H));
        self.length = size.width;
        self.rect.width = size.width;
        self.rect.height = WIDGET_H;
        Size::new(size.width, WIDGET_H)
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
    fn requests_repaint(&self) -> bool {
        self.dragging
    }
    fn repaint_interval(&self) -> Option<Duration> {
        self.dragging.then_some(Duration::from_millis(16))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[inline]
fn clip_info(c: Option<Rect>) -> ([f32; 2], [f32; 2], f32) {
    match c {
        Some(r) => ([r.x, r.y], [r.x + r.width, r.y + r.height], 1.0),
        None => ([0.0; 2], [1e5; 2], 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_width_grows_with_value_text() {
        let short = SliderLabeled::tooltip_width_for("1");
        let long = SliderLabeled::tooltip_width_for("100.00/100.00%");
        assert!(long > short);
        assert!(long <= 180.0);
    }

    #[test]
    fn tooltip_visibility_ignores_drag_when_cursor_left_knob() {
        let mut slider = SliderLabeled::new_f32([0.0, 0.0], 100.0, "Volume", 0.0, 1.0, 0.5);
        slider.dragging = true;
        slider.hovered = false;

        assert!(!slider.tooltip_visible());
        assert!(slider.overlay_instances().is_empty());
    }
}
