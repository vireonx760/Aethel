use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use glam::Vec2;
use std::any::Any;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
enum DragMode {
    None,
    Move,
    ResizeRight,
    ResizeBottom,
    ResizeCorner,
}

const ABS_MIN: f32 = 78.0;
const EDGE: f32 = 10.0;
const TITLE_H: f32 = 28.0;

pub struct Panel {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub min_size: [f32; 2],
    pub color: [f32; 4],
    pub radius: f32,
    pub resizable: bool,
    pub draggable: bool,
    rect: Rect,
    mode: DragMode,
    drag_start_mouse: Vec2,
    drag_start_size: Vec2,
    drag_start_pos: Vec2,
    just_activated: bool,
}

impl Panel {
    pub fn new(pos: [f32; 2], size: [f32; 2]) -> Self {
        let w = size[0].max(ABS_MIN);
        let h = size[1].max(ABS_MIN);
        Self {
            pos,
            size: [w, h],
            min_size: [0.0, 0.0],
            color: [0.13, 0.13, 0.16, 0.96],
            radius: 12.0,
            resizable: false,
            draggable: false,
            rect: Rect::new(pos[0], pos[1], w, h),
            mode: DragMode::None,
            drag_start_mouse: Vec2::ZERO,
            drag_start_size: Vec2::ZERO,
            drag_start_pos: Vec2::ZERO,
            just_activated: false,
        }
    }

    pub fn resizable(mut self, v: bool) -> Self {
        self.resizable = v;
        self
    }
    pub fn draggable(mut self, v: bool) -> Self {
        self.draggable = v;
        self
    }
    pub fn color(mut self, c: [f32; 4]) -> Self {
        self.color = c;
        self
    }
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    pub fn min_size(mut self, m: [f32; 2]) -> Self {
        self.min_size = m;
        self
    }
    // backward compat
    pub fn clip_content(self, _: bool) -> Self {
        self
    }

    fn effective_min(&self) -> [f32; 2] {
        [self.min_size[0].max(ABS_MIN), self.min_size[1].max(ABS_MIN)]
    }

    fn detect_mode(&self, mouse: Vec2) -> DragMode {
        let pmin = Vec2::from_array(self.pos);
        let pmax = pmin + Vec2::from_array(self.size);
        let near_r = (mouse.x - pmax.x).abs() < EDGE && mouse.y > pmin.y && mouse.y < pmax.y;
        let near_b = (mouse.y - pmax.y).abs() < EDGE && mouse.x > pmin.x && mouse.x < pmax.x;
        let near_c = (mouse.x - pmax.x).abs() < EDGE && (mouse.y - pmax.y).abs() < EDGE;
        if self.resizable {
            if near_c {
                return DragMode::ResizeCorner;
            }
            if near_r {
                return DragMode::ResizeRight;
            }
            if near_b {
                return DragMode::ResizeBottom;
            }
        }
        if self.draggable {
            let in_title = mouse.x > pmin.x
                && mouse.x < pmax.x
                && mouse.y > pmin.y
                && mouse.y < pmin.y + TITLE_H;
            if in_title {
                return DragMode::Move;
            }
        }
        DragMode::None
    }

    fn apply_size(&mut self, w: f32, h: f32) {
        let m = self.effective_min();
        self.size[0] = w.max(m[0]);
        self.size[1] = h.max(m[1]);
        self.rect.width = self.size[0];
        self.rect.height = self.size[1];
    }
}

impl Widget for Panel {
    fn update(&mut self, _dt: f32, input: &InputManager) {
        let mouse = input.mouse_pos;
        let just_pressed = input.lmb.just_pressed;
        let held = input.lmb.held;

        self.just_activated = false;

        match self.mode {
            DragMode::None => {
                if just_pressed {
                    let m = self.detect_mode(mouse);
                    if m != DragMode::None {
                        self.mode = m;
                        self.drag_start_mouse = mouse;
                        self.drag_start_size = Vec2::from_array(self.size);
                        self.drag_start_pos = Vec2::from_array(self.pos);
                        self.just_activated = true;
                    }
                }
            }
            DragMode::Move => {
                if held {
                    let np = self.drag_start_pos + (mouse - self.drag_start_mouse);
                    self.pos = [np.x, np.y];
                    self.rect.x = np.x;
                    self.rect.y = np.y;
                } else {
                    self.mode = DragMode::None;
                }
            }
            DragMode::ResizeRight => {
                if held {
                    let dw = mouse.x - self.drag_start_mouse.x;
                    self.apply_size(self.drag_start_size.x + dw, self.size[1]);
                } else {
                    self.mode = DragMode::None;
                }
            }
            DragMode::ResizeBottom => {
                if held {
                    let dh = mouse.y - self.drag_start_mouse.y;
                    self.apply_size(self.size[0], self.drag_start_size.y + dh);
                } else {
                    self.mode = DragMode::None;
                }
            }
            DragMode::ResizeCorner => {
                if held {
                    let d = mouse - self.drag_start_mouse;
                    self.apply_size(self.drag_start_size.x + d.x, self.drag_start_size.y + d.y);
                } else {
                    self.mode = DragMode::None;
                }
            }
        }
    }

    fn priority_click(&self) -> bool {
        self.just_activated
    }

    fn requests_repaint(&self) -> bool {
        self.mode != DragMode::None
    }

    fn repaint_interval(&self) -> Option<Duration> {
        (self.mode != DragMode::None).then_some(Duration::from_millis(16))
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let mut out = vec![WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: self.color,
            radius: self.radius,
            use_clip: 0.0,
            ..Default::default()
        }];

        if self.resizable {
            let [px, py] = self.pos;
            let [pw, ph] = self.size;
            let c = [0.3, 0.6, 0.9, 0.4];
            out.push(WidgetInstance {
                pos: [px + pw - 3.0, py + 12.0],
                size: [3.0, ph - 24.0],
                color: c,
                radius: 1.5,
                use_clip: 0.0,
                ..Default::default()
            });
            out.push(WidgetInstance {
                pos: [px + 12.0, py + ph - 3.0],
                size: [pw - 24.0, 3.0],
                color: c,
                radius: 1.5,
                use_clip: 0.0,
                ..Default::default()
            });
            out.push(WidgetInstance {
                pos: [px + pw - 8.0, py + ph - 8.0],
                size: [8.0, 8.0],
                color: [0.4, 0.7, 1.0, 0.6],
                radius: 4.0,
                use_clip: 0.0,
                ..Default::default()
            });
        }
        out
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: self.color,
            radius: self.radius,
            use_clip: 0.0,
            ..Default::default()
        });

        if self.resizable {
            let [px, py] = self.pos;
            let [pw, ph] = self.size;
            let color = [0.3, 0.6, 0.9, 0.4];
            for (pos, size, radius, color) in [
                ([px + pw - 3.0, py + 12.0], [3.0, ph - 24.0], 1.5, color),
                ([px + 12.0, py + ph - 3.0], [pw - 24.0, 3.0], 1.5, color),
                (
                    [px + pw - 8.0, py + ph - 8.0],
                    [8.0, 8.0],
                    4.0,
                    [0.4, 0.7, 1.0, 0.6],
                ),
            ] {
                ctx.push_instance(WidgetInstance {
                    pos,
                    size,
                    color,
                    radius,
                    use_clip: 0.0,
                    ..Default::default()
                });
            }
        }
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let s = constraints.constrain(Size::new(self.size[0], self.size[1]));
        let w = s.width.max(ABS_MIN);
        let h = s.height.max(ABS_MIN);
        self.size = [w, h];
        self.rect.width = w;
        self.rect.height = h;
        Size::new(w, h)
    }

    fn set_position(&mut self, p: Point) {
        self.pos = [p.x, p.y];
        self.rect.x = p.x;
        self.rect.y = p.y;
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
