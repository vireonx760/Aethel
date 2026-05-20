use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::core::simd;
use crate::gui::geometry::{BoxConstraints, Point as GeomPoint, Rect as GeomRect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use crate::primitives::{Line, Point, PrimitiveBatch, Rect, Triangle};
use std::any::Any;

pub struct PrimitiveWidget {
    instances: Vec<WidgetInstance>,
    rect: GeomRect,
    use_layout: bool,
}

impl PrimitiveWidget {
    pub fn from_point(point: Point) -> Self {
        let instance = point.to_instance();
        let rect = GeomRect::new(
            point.x - point.size / 2.0,
            point.y - point.size / 2.0,
            point.size,
            point.size,
        );
        Self {
            instances: vec![instance],
            rect,
            use_layout: false,
        }
    }

    pub fn from_line(line: Line) -> Self {
        let instance = line.to_instance();
        let min_x = line.start_x.min(line.end_x);
        let min_y = line.start_y.min(line.end_y);
        let max_x = line.start_x.max(line.end_x);
        let max_y = line.start_y.max(line.end_y);
        Self {
            instances: vec![instance],
            rect: GeomRect::new(min_x, min_y, max_x - min_x, max_y - min_y),
            use_layout: false,
        }
    }

    pub fn from_rect(rect: Rect) -> Self {
        let instance = rect.to_instance();
        let geom_rect = GeomRect::new(rect.x, rect.y, rect.width, rect.height);
        Self {
            instances: vec![instance],
            rect: geom_rect,
            use_layout: false,
        }
    }

    pub fn from_rect_stroke(rect: Rect, stroke_width: f32) -> Self {
        let instances = rect.to_stroke_instances(stroke_width);
        let geom_rect = GeomRect::new(rect.x, rect.y, rect.width, rect.height);
        Self {
            instances,
            rect: geom_rect,
            use_layout: false,
        }
    }

    pub fn from_triangle(triangle: Triangle) -> Self {
        let instances = triangle.to_fill_instances();
        bounds_from_triangle(triangle, instances)
    }

    pub fn from_triangle_step(triangle: Triangle, step: f32) -> Self {
        let instances = triangle.to_fill_instances_with_step(step);
        bounds_from_triangle(triangle, instances)
    }

    pub fn from_triangle_outline(triangle: Triangle) -> Self {
        let instances = triangle.to_outline_instances();
        bounds_from_triangle(triangle, instances)
    }

    pub fn from_triangle_stroke(triangle: Triangle, stroke_width: f32) -> Self {
        let instances = triangle.to_stroke_instances(stroke_width);
        bounds_from_triangle(triangle, instances)
    }

    pub fn with_layout(mut self) -> Self {
        self.use_layout = true;
        self
    }

    pub fn custom(instances: Vec<WidgetInstance>) -> Self {
        if instances.is_empty() {
            return Self {
                instances,
                rect: GeomRect::new(0.0, 0.0, 0.0, 0.0),
                use_layout: false,
            };
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for inst in &instances {
            min_x = min_x.min(inst.pos[0]);
            min_y = min_y.min(inst.pos[1]);
            max_x = max_x.max(inst.pos[0] + inst.size[0]);
            max_y = max_y.max(inst.pos[1] + inst.size[1]);
        }

        let rect = GeomRect::new(min_x, min_y, max_x - min_x, max_y - min_y);
        Self {
            instances,
            rect,
            use_layout: false,
        }
    }

    pub fn from_batch(batch: PrimitiveBatch) -> Self {
        Self {
            rect: batch.bounds(),
            instances: batch.into_instances(),
            use_layout: false,
        }
    }
}

fn bounds_from_triangle(t: Triangle, instances: Vec<WidgetInstance>) -> PrimitiveWidget {
    let min_x = t.x1.min(t.x2).min(t.x3);
    let min_y = t.y1.min(t.y2).min(t.y3);
    let max_x = t.x1.max(t.x2).max(t.x3);
    let max_y = t.y1.max(t.y2).max(t.y3);
    PrimitiveWidget {
        instances,
        rect: GeomRect::new(min_x, min_y, max_x - min_x, max_y - min_y),
        use_layout: false,
    }
}

impl Widget for PrimitiveWidget {
    fn update(&mut self, _dt: f32, _input: &InputManager) {}

    fn instances(&self) -> Vec<WidgetInstance> {
        self.instances.clone()
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.push_instances(&self.instances);
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        if self.use_layout {
            let size = constraints.constrain(Size::new(self.rect.width, self.rect.height));
            self.rect.width = size.width;
            self.rect.height = size.height;
            size
        } else {
            Size::new(self.rect.width, self.rect.height)
        }
    }

    fn set_position(&mut self, position: GeomPoint) {
        let dx = position.x - self.rect.x;
        let dy = position.y - self.rect.y;
        self.rect.x = position.x;
        self.rect.y = position.y;
        simd::translate_widget_instances(&mut self.instances, [dx, dy]);
    }

    fn get_rect(&self) -> GeomRect {
        self.rect
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub mod helpers {
    use super::*;

    pub fn draw_point(x: f32, y: f32, color: [f32; 4], size: f32) -> Vec<WidgetInstance> {
        vec![Point::new(x, y).color(color).size(size).to_instance()]
    }

    pub fn draw_line(
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 4],
        width: f32,
    ) -> Vec<WidgetInstance> {
        vec![
            Line::new(x1, y1, x2, y2)
                .color(color)
                .width(width)
                .to_instance(),
        ]
    }

    pub fn draw_rect(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        radius: f32,
    ) -> Vec<WidgetInstance> {
        vec![
            Rect::new(x, y, w, h)
                .color(color)
                .radius(radius)
                .to_instance(),
        ]
    }

    pub fn draw_rect_stroke(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        stroke: f32,
    ) -> Vec<WidgetInstance> {
        Rect::new(x, y, w, h)
            .color(color)
            .to_stroke_instances(stroke)
    }

    pub fn draw_triangle(
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        color: [f32; 4],
    ) -> Vec<WidgetInstance> {
        Triangle::new(x1, y1, x2, y2, x3, y3)
            .color(color)
            .to_fill_instances()
    }

    pub fn draw_triangle_outline(
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        color: [f32; 4],
    ) -> Vec<WidgetInstance> {
        Triangle::new(x1, y1, x2, y2, x3, y3)
            .color(color)
            .to_outline_instances()
    }

    pub fn draw_triangle_stroke(
        points: [[f32; 2]; 3],
        color: [f32; 4],
        width: f32,
    ) -> Vec<WidgetInstance> {
        Triangle::new(
            points[0][0],
            points[0][1],
            points[1][0],
            points[1][1],
            points[2][0],
            points[2][1],
        )
        .color(color)
        .to_stroke_instances(width)
    }

    pub fn draw_circle(x: f32, y: f32, radius: f32, color: [f32; 4]) -> Vec<WidgetInstance> {
        vec![
            Rect::new(x - radius, y - radius, radius * 2.0, radius * 2.0)
                .color(color)
                .radius(radius)
                .to_instance(),
        ]
    }

    pub fn draw_polygon(points: &[[f32; 2]], color: [f32; 4], width: f32) -> Vec<WidgetInstance> {
        let n = points.len();
        if n < 2 {
            return vec![];
        }
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let next = (i + 1) % n;
            let [x1, y1] = points[i];
            let [x2, y2] = points[next];
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.1 {
                let angle = dy.atan2(dx);
                out.push(WidgetInstance {
                    pos: [(x1 + x2) / 2.0 - len / 2.0, (y1 + y2) / 2.0 - width / 2.0],
                    size: [len, width],
                    color,
                    radius: width / 2.0,
                    rotation: angle,
                    ..Default::default()
                });
            }
        }
        out
    }
}
