use std::any::Any;
use std::sync::{Arc, Mutex};

use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::command::{CommandId, UpdateCtx};
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::text::{set_buffer_size, set_buffer_text, shape_text, text_area};
use crate::gui::widget::Widget;
use glam::Vec2;
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, TextArea, TextBounds};

pub type ButtonCallback = Arc<Mutex<dyn FnMut() + Send + Sync>>;

pub struct Button {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    natural_size: [f32; 2],
    pub text: String,
    pub base_color: [f32; 4],
    pub hover_color: [f32; 4],
    pub pressed_color: [f32; 4],
    pub text_color: [f32; 4],
    pub hovered: bool,
    pub pressed: bool,
    pub enabled: bool,
    rect: Rect,
    clip_rect: Option<Rect>,

    on_click: Option<ButtonCallback>,
    on_press: Option<ButtonCallback>,
    on_release: Option<ButtonCallback>,
    on_hover_enter: Option<ButtonCallback>,
    on_hover_exit: Option<ButtonCallback>,
    on_click_cmd: Option<CommandId<()>>,

    was_hovered: bool,
    was_pressed: bool,
}

impl Button {
    pub fn new(pos: [f32; 2], size: [f32; 2], text: impl Into<String>) -> Self {
        Self {
            pos,
            size,
            natural_size: size,
            text: text.into(),
            base_color: [0.25, 0.25, 0.28, 1.0],
            hover_color: [0.35, 0.35, 0.38, 1.0],
            pressed_color: [0.20, 0.20, 0.23, 1.0],
            text_color: [0.9, 0.9, 0.95, 1.0],
            hovered: false,
            pressed: false,
            enabled: true,
            rect: Rect::new(pos[0], pos[1], size[0], size[1]),
            clip_rect: None,
            on_click: None,
            on_press: None,
            on_release: None,
            on_hover_enter: None,
            on_hover_exit: None,
            on_click_cmd: None,
            was_hovered: false,
            was_pressed: false,
        }
    }

    pub fn on_click<F: FnMut() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_click = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn on_click_cmd(mut self, command: CommandId<()>) -> Self {
        self.on_click_cmd = Some(command);
        self
    }
    pub fn on_press<F: FnMut() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_press = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn on_release<F: FnMut() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_release = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn on_hover_enter<F: FnMut() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_hover_enter = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn on_hover_exit<F: FnMut() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_hover_exit = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn enabled(mut self, v: bool) -> Self {
        self.enabled = v;
        self
    }
    pub fn colors(mut self, base: [f32; 4], hover: [f32; 4], pressed: [f32; 4]) -> Self {
        self.base_color = base;
        self.hover_color = hover;
        self.pressed_color = pressed;
        self
    }
    pub fn text_color(mut self, c: [f32; 4]) -> Self {
        self.text_color = c;
        self
    }

    fn fire(cb: &Option<ButtonCallback>) {
        if let Some(c) = cb
            && let Ok(mut f) = c.lock()
        {
            f();
        }
    }
}

impl Widget for Button {
    fn update(&mut self, _dt: f32, input: &InputManager) {
        if !self.enabled {
            self.hovered = false;
            self.pressed = false;
            self.was_hovered = false;
            self.was_pressed = false;
            return;
        }

        let rect_min = Vec2::from_array(self.pos);
        let rect_max = rect_min + Vec2::from_array(self.size);
        let mut mouse_in =
            input.mouse_pos.cmpge(rect_min).all() && input.mouse_pos.cmplt(rect_max).all();

        if mouse_in && let Some(clip) = self.clip_rect {
            let cmin = Vec2::new(clip.x, clip.y);
            let cmax = Vec2::new(clip.x + clip.width, clip.y + clip.height);
            if !input.mouse_pos.cmpge(cmin).all() || !input.mouse_pos.cmplt(cmax).all() {
                mouse_in = false;
            }
        }

        self.hovered = mouse_in;
        let currently_pressed = mouse_in && input.lmb.held;

        if self.hovered && !self.was_hovered {
            Self::fire(&self.on_hover_enter);
        }
        if !self.hovered && self.was_hovered {
            Self::fire(&self.on_hover_exit);
        }
        if mouse_in && input.lmb.just_pressed {
            Self::fire(&self.on_press);
        }
        if mouse_in && input.lmb.just_released {
            Self::fire(&self.on_release);
            Self::fire(&self.on_click);
        }

        self.pressed = currently_pressed;
        self.was_hovered = self.hovered;
        self.was_pressed = currently_pressed;
    }

    fn update_ctx(&mut self, dt: f32, input: &InputManager, ctx: &mut UpdateCtx) {
        self.update(dt, input);
        if self.enabled
            && self.hovered
            && input.lmb.just_released
            && let Some(command) = self.on_click_cmd
        {
            ctx.emit_id(command);
        }
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let color = if !self.enabled {
            [0.15, 0.15, 0.18, 1.0]
        } else if self.pressed {
            self.pressed_color
        } else if self.hovered {
            self.hover_color
        } else {
            self.base_color
        };

        let (clip_min, clip_max, use_clip) = clip_info(self.clip_rect);

        vec![WidgetInstance {
            pos: self.pos,
            size: self.size,
            color,
            radius: 8.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        }]
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let color = if !self.enabled {
            [0.15, 0.15, 0.18, 1.0]
        } else if self.pressed {
            self.pressed_color
        } else if self.hovered {
            self.hover_color
        } else {
            self.base_color
        };

        let (clip_min, clip_max, use_clip) = clip_info(self.clip_rect);
        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: self.size,
            color,
            radius: 8.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
    }

    fn prepare_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        if self.text.is_empty() {
            return;
        }

        let font_size = (self.size[1] * 0.5).clamp(12.0, 32.0);
        let mut buffer = Buffer::new(font_system, Metrics::new(font_size, font_size * 1.2));

        set_buffer_size(&mut buffer, font_system, self.size[0], self.size[1]);

        let text_color = if self.enabled {
            self.text_color
        } else {
            [0.4, 0.4, 0.45, 1.0]
        };
        let [r, g, b, a] = text_color;
        let color = Color::rgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            (a * 255.0) as u8,
        );

        set_buffer_text(
            &mut buffer,
            font_system,
            &self.text,
            Attrs::new().family(Family::SansSerif).color(color),
        );
        shape_text(&mut buffer, font_system);
        buffers.push(buffer);
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if self.text.is_empty() {
            return;
        }

        if let Some(buffer) = buffers.get(*bi) {
            let mut text_w = 0.0f32;
            let mut line_count = 0usize;
            for run in buffer.layout_runs() {
                text_w = text_w.max(run.line_w);
                line_count += 1;
            }
            let text_h = line_count as f32 * buffer.metrics().line_height;

            let left = self.pos[0] + (self.size[0] - text_w).max(0.0) * 0.5;
            let top = self.pos[1] + (self.size[1] - text_h).max(0.0) * 0.5;

            let text_color = if self.enabled {
                self.text_color
            } else {
                [0.4, 0.4, 0.45, 1.0]
            };
            let [r, g, b, a] = text_color;

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
                    right: (self.pos[0] + self.size[0]) as i32,
                    bottom: (self.pos[1] + self.size[1]) as i32,
                }
            };

            areas.push(text_area(
                buffer,
                left,
                top,
                bounds,
                Color::rgba(
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                    (a * 255.0) as u8,
                ),
            ));
            *bi += 1;
        }
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

#[inline]
fn clip_info(c: Option<Rect>) -> ([f32; 2], [f32; 2], f32) {
    match c {
        Some(r) => ([r.x, r.y], [r.x + r.width, r.y + r.height], 1.0),
        None => ([0.0; 2], [1e5; 2], 0.0),
    }
}
