use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use std::any::Any;

pub struct Separator {
    pos: [f32; 2],
    size: [f32; 2],
    natural_size: [f32; 2],
    color: [f32; 4],
    rect: Rect,
}

impl Separator {
    pub fn horizontal(width: f32) -> Self {
        Self::new([0.0, 0.0], [width.max(1.0), 1.0])
    }

    pub fn vertical(height: f32) -> Self {
        Self::new([0.0, 0.0], [1.0, height.max(1.0)])
    }

    pub fn new(pos: [f32; 2], size: [f32; 2]) -> Self {
        let size = [size[0].max(1.0), size[1].max(1.0)];
        Self {
            pos,
            size,
            natural_size: size,
            color: [0.32, 0.34, 0.39, 1.0],
            rect: Rect::new(pos[0], pos[1], size[0], size[1]),
        }
    }

    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

impl Widget for Separator {
    fn update(&mut self, _dt: f32, _input: &InputManager) {}

    fn instances(&self) -> Vec<WidgetInstance> {
        vec![WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: self.color,
            radius: 0.0,
            ..Default::default()
        }]
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: self.color,
            radius: 0.0,
            ..Default::default()
        });
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let size = constraints.constrain(Size::new(self.natural_size[0], self.natural_size[1]));
        self.size = [size.width.max(1.0), size.height.max(1.0)];
        self.rect.width = self.size[0];
        self.rect.height = self.size[1];
        Size::new(self.size[0], self.size[1])
    }

    fn set_position(&mut self, position: Point) {
        self.pos = [position.x, position.y];
        self.rect.x = position.x;
        self.rect.y = position.y;
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
