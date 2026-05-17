// primitives/line.rs
use crate::core::renderer::WidgetInstance;

#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub color: [f32; 4],
    pub width: f32,
}

impl Line {
    pub fn new(start_x: f32, start_y: f32, end_x: f32, end_y: f32) -> Self {
        Self {
            start_x,
            start_y,
            end_x,
            end_y,
            color: [1.0, 1.0, 1.0, 1.0],
            width: 2.0,
        }
    }

    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn to_instance(&self) -> WidgetInstance {
        let dx = self.end_x - self.start_x;
        let dy = self.end_y - self.start_y;
        let length = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);

        let center_x = (self.start_x + self.end_x) / 2.0;
        let center_y = (self.start_y + self.end_y) / 2.0;

        WidgetInstance {
            pos: [center_x - length / 2.0, center_y - self.width / 2.0],
            size: [length, self.width],
            color: self.color,
            radius: self.width / 2.0,
            rotation: angle,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_line_emits_rotation() {
        let instance = Line::new(0.0, 0.0, 10.0, 10.0).width(2.0).to_instance();
        assert!(instance.rotation > 0.7);
        assert!(instance.rotation < 0.9);
        assert!(instance.size[0] > 14.0);
    }
}
