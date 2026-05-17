use crate::core::renderer::WidgetInstance;
use crate::gui::geometry::Rect as GeomRect;
use crate::gui::paint::PaintCtx;
use crate::primitives::{Line, Rect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3d {
    pub origin: Point3,
    pub screen_center: [f32; 2],
    pub scale: f32,
    pub perspective: f32,
}

impl Camera3d {
    #[inline]
    pub const fn new(screen_center: [f32; 2]) -> Self {
        Self {
            origin: Point3::new(0.0, 0.0, 0.0),
            screen_center,
            scale: 1.0,
            perspective: 0.0,
        }
    }

    #[inline]
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale.max(0.0001);
        self
    }

    #[inline]
    pub fn perspective(mut self, perspective: f32) -> Self {
        self.perspective = perspective.max(0.0);
        self
    }

    #[inline]
    pub fn origin(mut self, origin: Point3) -> Self {
        self.origin = origin;
        self
    }

    #[inline]
    pub fn project(self, point: Point3) -> [f32; 2] {
        let x = point.x - self.origin.x;
        let y = point.y - self.origin.y;
        let z = point.z - self.origin.z;
        let depth = 1.0 + z * self.perspective;
        let inv_depth = if depth.abs() > 0.0001 {
            depth.recip()
        } else {
            1.0
        };
        [
            self.screen_center[0] + x * self.scale * inv_depth,
            self.screen_center[1] - y * self.scale * inv_depth,
        ]
    }
}

pub trait PrimitiveSink {
    fn push_instance(&mut self, instance: WidgetInstance);
}

impl PrimitiveSink for PaintCtx<'_> {
    #[inline]
    fn push_instance(&mut self, instance: WidgetInstance) {
        PaintCtx::push_instance(self, instance);
    }
}

#[derive(Debug, Clone)]
pub struct PrimitiveBatch {
    instances: Vec<WidgetInstance>,
    bounds: GeomRect,
}

impl PrimitiveBatch {
    #[inline]
    pub fn instances(&self) -> &[WidgetInstance] {
        &self.instances
    }

    #[inline]
    pub fn into_instances(self) -> Vec<WidgetInstance> {
        self.instances
    }

    #[inline]
    pub fn bounds(&self) -> GeomRect {
        self.bounds
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct PrimitiveBuilder {
    instances: Vec<WidgetInstance>,
    bounds: Option<GeomRect>,
}

impl PrimitiveBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            bounds: None,
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.instances.clear();
        self.bounds = None;
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.instances.reserve(additional);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.instances.capacity()
    }

    #[inline]
    pub fn bounds(&self) -> Option<GeomRect> {
        self.bounds
    }

    #[inline]
    pub fn as_slice(&self) -> &[WidgetInstance] {
        &self.instances
    }

    #[inline]
    pub fn extend_instances(
        &mut self,
        instances: impl IntoIterator<Item = WidgetInstance>,
    ) -> &mut Self {
        for instance in instances {
            self.push_instance(instance);
        }
        self
    }

    pub fn rect(&mut self, rect: Rect) -> &mut Self {
        self.push_instance(rect.to_instance());
        self
    }

    pub fn rect_xywh(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        radius: f32,
    ) -> &mut Self {
        self.rect(Rect::new(x, y, width, height).color(color).radius(radius))
    }

    pub fn rect_stroke(&mut self, rect: Rect, stroke_width: f32) -> &mut Self {
        self.extend_instances(rect.stroke_instances(stroke_width.max(0.0)));
        self
    }

    pub fn point(&mut self, x: f32, y: f32, color: [f32; 4], size: f32) -> &mut Self {
        self.push_instance(WidgetInstance {
            pos: [x - size * 0.5, y - size * 0.5],
            size: [size, size],
            color,
            radius: size * 0.5,
            ..Default::default()
        });
        self
    }

    pub fn circle(&mut self, x: f32, y: f32, radius: f32, color: [f32; 4]) -> &mut Self {
        self.point(x, y, color, radius * 2.0)
    }

    pub fn line(
        &mut self,
        start: [f32; 2],
        end: [f32; 2],
        color: [f32; 4],
        width: f32,
    ) -> &mut Self {
        self.push_instance(
            Line::new(start[0], start[1], end[0], end[1])
                .color(color)
                .width(width)
                .to_instance(),
        );
        self
    }

    pub fn polyline(&mut self, points: &[[f32; 2]], color: [f32; 4], width: f32) -> &mut Self {
        for segment in points.windows(2) {
            self.line(segment[0], segment[1], color, width);
        }
        self
    }

    pub fn polygon_outline(
        &mut self,
        points: &[[f32; 2]],
        color: [f32; 4],
        width: f32,
    ) -> &mut Self {
        if points.len() < 2 {
            return self;
        }
        self.polyline(points, color, width);
        self.line(*points.last().unwrap(), points[0], color, width)
    }

    pub fn line_3d(
        &mut self,
        camera: Camera3d,
        start: Point3,
        end: Point3,
        color: [f32; 4],
        width: f32,
    ) -> &mut Self {
        self.line(camera.project(start), camera.project(end), color, width)
    }

    pub fn cube_wireframe(
        &mut self,
        camera: Camera3d,
        center: Point3,
        size: f32,
        color: [f32; 4],
        width: f32,
    ) -> &mut Self {
        let h = size * 0.5;
        let v = [
            Point3::new(center.x - h, center.y - h, center.z - h),
            Point3::new(center.x + h, center.y - h, center.z - h),
            Point3::new(center.x + h, center.y + h, center.z - h),
            Point3::new(center.x - h, center.y + h, center.z - h),
            Point3::new(center.x - h, center.y - h, center.z + h),
            Point3::new(center.x + h, center.y - h, center.z + h),
            Point3::new(center.x + h, center.y + h, center.z + h),
            Point3::new(center.x - h, center.y + h, center.z + h),
        ];
        for (a, b) in [
            (0usize, 1usize),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            self.line_3d(camera, v[a], v[b], color, width);
        }
        self
    }

    pub fn custom(&mut self, build: impl FnOnce(&mut dyn PrimitiveSink)) -> &mut Self {
        build(self);
        self
    }

    pub fn build(self) -> PrimitiveBatch {
        PrimitiveBatch {
            instances: self.instances,
            bounds: self.bounds.unwrap_or(GeomRect::new(0.0, 0.0, 0.0, 0.0)),
        }
    }

    pub fn build_widget(self) -> crate::primitives::PrimitiveWidget {
        crate::primitives::PrimitiveWidget::from_batch(self.build())
    }

    #[inline]
    fn expand_bounds(&mut self, instance: &WidgetInstance) {
        let Some(rect) = instance_bounds(instance) else {
            return;
        };
        self.bounds = Some(match self.bounds {
            Some(current) => union_rect(current, rect),
            None => rect,
        });
    }
}

impl Default for PrimitiveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimitiveSink for PrimitiveBuilder {
    #[inline]
    fn push_instance(&mut self, instance: WidgetInstance) {
        self.expand_bounds(&instance);
        self.instances.push(instance);
    }
}

#[inline]
fn union_rect(a: GeomRect, b: GeomRect) -> GeomRect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = a.right().max(b.right());
    let y1 = a.bottom().max(b.bottom());
    GeomRect::new(x0, y0, x1 - x0, y1 - y0)
}

fn instance_bounds(instance: &WidgetInstance) -> Option<GeomRect> {
    if instance.size[0] <= 0.0 || instance.size[1] <= 0.0 {
        return None;
    }

    if instance.rotation.abs() <= 0.000001 {
        return Some(GeomRect::new(
            instance.pos[0],
            instance.pos[1],
            instance.size[0],
            instance.size[1],
        ));
    }

    let center = [
        instance.pos[0] + instance.size[0] * 0.5,
        instance.pos[1] + instance.size[1] * 0.5,
    ];
    let half = [instance.size[0] * 0.5, instance.size[1] * 0.5];
    let sin = instance.rotation.sin();
    let cos = instance.rotation.cos();
    let mut min = [f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN];

    for [x, y] in [
        [-half[0], -half[1]],
        [half[0], -half[1]],
        [half[0], half[1]],
        [-half[0], half[1]],
    ] {
        let p = [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos];
        min[0] = min[0].min(p[0]);
        min[1] = min[1].min(p[1]);
        max[0] = max[0].max(p[0]);
        max[1] = max[1].max(p[1]);
    }

    Some(GeomRect::new(
        min[0],
        min[1],
        max[0] - min[0],
        max[1] - min[1],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_reuses_capacity_after_clear() {
        let mut builder = PrimitiveBuilder::with_capacity(4);
        builder.rect_xywh(0.0, 0.0, 10.0, 10.0, [1.0; 4], 2.0);
        let cap = builder.capacity();
        builder.clear();
        builder.rect_xywh(5.0, 5.0, 10.0, 10.0, [1.0; 4], 2.0);
        assert_eq!(builder.capacity(), cap);
    }

    #[test]
    fn cube_wireframe_emits_twelve_rotatable_edges() {
        let camera = Camera3d::new([100.0, 100.0]).scale(2.0).perspective(0.001);
        let mut builder = PrimitiveBuilder::new();
        builder.cube_wireframe(camera, Point3::new(0.0, 0.0, 0.0), 40.0, [1.0; 4], 2.0);
        assert_eq!(builder.len(), 12);
        assert!(
            builder
                .as_slice()
                .iter()
                .all(|instance| instance.rotation.is_finite())
        );
    }

    #[test]
    fn rotated_line_expands_bounds() {
        let mut builder = PrimitiveBuilder::new();
        builder.line([0.0, 0.0], [10.0, 10.0], [1.0; 4], 2.0);
        let bounds = builder.bounds().unwrap();
        assert!(bounds.width > 10.0);
        assert!(bounds.height > 10.0);
    }
}
