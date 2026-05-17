// primitives/point.rs
use crate::core::renderer::WidgetInstance;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub color: [f32; 4],
    pub size: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            color: [1.0, 1.0, 1.0, 1.0],
            size: 2.0,
        }
    }

    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn to_instance(&self) -> WidgetInstance {
        WidgetInstance {
            pos: [self.x - self.size / 2.0, self.y - self.size / 2.0],
            size: [self.size, self.size],
            color: self.color,
            radius: self.size / 2.0,
            ..Default::default()
        }
    }
}
