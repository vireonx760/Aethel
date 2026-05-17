use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::binding::F32Signal;
use crate::gui::command::{CommandId, UpdateCtx};
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use glam::Vec2;
use std::any::Any;
use winit::event::MouseButton;

const WIDGET_H: f32 = 36.0;
const KNOB_R: f32 = 12.0;
const KNOB_H: f32 = 36.0;

pub struct Slider {
    pub pos: [f32; 2],
    pub length: f32,
    natural_length: f32,
    pub track_height: f32,
    pub value: f32,
    pub dragging: bool,
    rect: Rect,
    clip_rect: Option<Rect>,
    signal: Option<F32Signal>,
    on_change_cmd: Option<CommandId<f32>>,
}

impl Slider {
    pub fn new(pos: [f32; 2], length: f32) -> Self {
        Self {
            pos,
            length,
            natural_length: length,
            track_height: 8.0,
            value: 0.5,
            dragging: false,
            rect: Rect::new(pos[0], pos[1], length, WIDGET_H),
            clip_rect: None,
            signal: None,
            on_change_cmd: None,
        }
    }

    pub fn bind_signal(mut self, target: F32Signal) -> Self {
        self.value = target.get().clamp(0.0, 1.0);
        self.signal = Some(target);
        self
    }

    pub fn on_change_cmd(mut self, command: CommandId<f32>) -> Self {
        self.on_change_cmd = Some(command);
        self
    }

    #[inline]
    fn track_cy(&self) -> f32 {
        self.pos[1] + WIDGET_H * 0.5
    }

    #[inline]
    fn effective_len(&self) -> f32 {
        (self.length - KNOB_R * 2.0).max(0.0)
    }

    #[inline]
    fn knob_cx(&self) -> f32 {
        self.pos[0] + KNOB_R + self.value * self.effective_len()
    }
}

impl Widget for Slider {
    fn update(&mut self, _dt: f32, input: &InputManager) {
        if let Some(signal) = &self.signal
            && !self.dragging
        {
            self.value = signal.get().clamp(0.0, 1.0);
        }

        let knob_cx = self.knob_cx();
        let cy = self.track_cy();

        let knob_min = Vec2::new(knob_cx - KNOB_R, cy - KNOB_H * 0.5);
        let knob_max = Vec2::new(knob_cx + KNOB_R, cy + KNOB_H * 0.5);

        let mouse_in_knob =
            input.mouse_pos.cmpge(knob_min).all() && input.mouse_pos.cmplt(knob_max).all();

        let mouse_pressed = input.is_mouse_down(MouseButton::Left);

        let knob_visible = self.clip_rect.is_none_or(|clip| {
            let cmin = Vec2::new(clip.x, clip.y);
            let cmax = Vec2::new(clip.x + clip.width, clip.y + clip.height);
            knob_min.cmplt(cmax).all() && knob_max.cmpge(cmin).all()
        });

        if !self.dragging && mouse_pressed && mouse_in_knob && knob_visible {
            self.dragging = true;
        }

        if self.dragging {
            if mouse_pressed {
                let eff = self.effective_len();
                if eff > 0.0 {
                    let new_value = (input.mouse_pos.x - self.pos[0] - KNOB_R) / eff;
                    self.value = new_value.clamp(0.0, 1.0);
                    if let Some(signal) = &self.signal {
                        signal.set(self.value);
                    }
                }
            } else {
                self.dragging = false;
            }
        }
    }

    fn update_ctx(&mut self, dt: f32, input: &InputManager, ctx: &mut UpdateCtx) {
        let before = self.value;
        self.update(dt, input);
        if (before - self.value).abs() > f32::EPSILON
            && let Some(command) = self.on_change_cmd
        {
            ctx.emit_f32(command, self.value);
        }
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let (clip_min, clip_max, use_clip) = clip_info(self.clip_rect);

        let cy = self.track_cy();
        let th = self.track_height;

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
            size: [KNOB_R + self.value * self.effective_len(), th],
            color: [0.0, 0.65, 0.85, 1.0],
            radius: th * 0.5,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        };

        let knob_x = self.knob_cx() - KNOB_R;
        let knob = WidgetInstance {
            pos: [knob_x, cy - KNOB_H * 0.5],
            size: [KNOB_R * 2.0, KNOB_H],
            color: [0.35, 0.35, 0.40, 1.0],
            radius: KNOB_R,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        };

        vec![track, fill, knob]
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let (clip_min, clip_max, use_clip) = clip_info(self.clip_rect);
        let cy = self.track_cy();
        let th = self.track_height;
        let knob_x = self.knob_cx() - KNOB_R;

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
            size: [KNOB_R + self.value * self.effective_len(), th],
            color: [0.0, 0.65, 0.85, 1.0],
            radius: th * 0.5,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
        ctx.push_instance(WidgetInstance {
            pos: [knob_x, cy - KNOB_H * 0.5],
            size: [KNOB_R * 2.0, KNOB_H],
            color: [0.35, 0.35, 0.40, 1.0],
            radius: KNOB_R,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let size = constraints.constrain(Size::new(self.natural_length, WIDGET_H));
        self.length = size.width;
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
