// primitives/rect.rs
use crate::core::renderer::WidgetInstance;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
    pub radius: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color: [1.0, 1.0, 1.0, 1.0],
            radius: 0.0,
        }
    }

    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn to_instance(&self) -> WidgetInstance {
        WidgetInstance {
            pos: [self.x, self.y],
            size: [self.width, self.height],
            color: self.color,
            radius: self.radius,
            ..Default::default()
        }
    }

    pub fn to_stroke_instances(&self, stroke_width: f32) -> Vec<WidgetInstance> {
        Vec::from(self.stroke_instances(stroke_width))
    }

    pub fn stroke_instances(&self, stroke_width: f32) -> [WidgetInstance; 4] {
        [
            WidgetInstance {
                pos: [self.x, self.y],
                size: [self.width, stroke_width],
                color: self.color,
                radius: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [self.x, self.y + self.height - stroke_width],
                size: [self.width, stroke_width],
                color: self.color,
                radius: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [self.x, self.y],
                size: [stroke_width, self.height],
                color: self.color,
                radius: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [self.x + self.width - stroke_width, self.y],
                size: [stroke_width, self.height],
                color: self.color,
                radius: 0.0,
                ..Default::default()
            },
        ]
    }
}
