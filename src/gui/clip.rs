use crate::core::renderer::WidgetInstance;
use crate::core::simd;
use crate::gui::geometry::{Point, Rect};
use glyphon::TextBounds;

const CLIP_SENTINEL_MAX: f32 = 100_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ClipRect {
    #[inline]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Option<Self> {
        let rect = Self {
            x,
            y,
            width,
            height,
        };
        if rect.is_finite() && !rect.is_empty() {
            Some(rect)
        } else {
            None
        }
    }

    #[inline]
    pub fn from_rect(rect: Rect) -> Option<Self> {
        Self::new(rect.x, rect.y, rect.width, rect.height)
    }

    #[inline]
    pub fn from_min_max(min: [f32; 2], max: [f32; 2]) -> Option<Self> {
        let x0 = min[0].min(max[0]);
        let y0 = min[1].min(max[1]);
        let x1 = min[0].max(max[0]);
        let y1 = min[1].max(max[1]);
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }

    #[inline]
    pub fn covering_screen(width: u32, height: u32) -> Option<Self> {
        Self::new(0.0, 0.0, width as f32, height as f32)
    }

    #[inline]
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    #[inline]
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    #[inline]
    pub fn min(self) -> [f32; 2] {
        [self.x, self.y]
    }

    #[inline]
    pub fn max(self) -> [f32; 2] {
        [self.right(), self.bottom()]
    }

    #[inline]
    pub fn to_rect(self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    #[inline]
    pub fn normalized(self) -> Option<Self> {
        if !self.is_finite() {
            return None;
        }

        let x0 = self.x.min(self.right());
        let x1 = self.x.max(self.right());
        let y0 = self.y.min(self.bottom());
        let y1 = self.y.max(self.bottom());
        let width = x1 - x0;
        let height = y1 - y0;

        if width <= 0.0 || height <= 0.0 {
            None
        } else {
            Some(Self {
                x: x0,
                y: y0,
                width,
                height,
            })
        }
    }

    #[inline]
    pub fn contains_point(self, point: Point) -> bool {
        point.x >= self.x && point.x < self.right() && point.y >= self.y && point.y < self.bottom()
    }

    #[inline(always)]
    pub fn intersects(self, other: Self) -> bool {
        let ax1 = self.x + self.width;
        let ay1 = self.y + self.height;
        let bx1 = other.x + other.width;
        let by1 = other.y + other.height;
        self.x < bx1 && ax1 > other.x && self.y < by1 && ay1 > other.y
    }

    #[inline]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let [x0, y0, x1, y1] = simd::intersect_ltrb(
            [self.x, self.y, self.right(), self.bottom()],
            [other.x, other.y, other.right(), other.bottom()],
        )?;
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }

    #[inline]
    pub fn inset(self, amount: f32) -> Option<Self> {
        Self::new(
            self.x + amount,
            self.y + amount,
            self.width - amount * 2.0,
            self.height - amount * 2.0,
        )
    }

    #[inline]
    pub fn translate(self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            width: self.width,
            height: self.height,
        }
    }

    #[inline]
    pub fn text_bounds(self) -> TextBounds {
        TextBounds {
            left: self.x.floor() as i32,
            top: self.y.floor() as i32,
            right: self.right().ceil() as i32,
            bottom: self.bottom().ceil() as i32,
        }
    }

    #[inline]
    pub fn scissor(self, surface_width: u32, surface_height: u32) -> Option<ScissorRect> {
        if surface_width == 0 || surface_height == 0 {
            return None;
        }

        let x0 = self.x.floor().max(0.0).min(surface_width as f32);
        let y0 = self.y.floor().max(0.0).min(surface_height as f32);
        let x1 = self.right().ceil().max(0.0).min(surface_width as f32);
        let y1 = self.bottom().ceil().max(0.0).min(surface_height as f32);

        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        Some(ScissorRect {
            x: x0 as u32,
            y: y0 as u32,
            width: (x1 - x0) as u32,
            height: (y1 - y0) as u32,
        })
    }

    #[inline]
    pub fn apply_to_instance(self, instance: &mut WidgetInstance) {
        instance.clip_min = self.min();
        instance.clip_max = self.max();
        instance.use_clip = 1.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScissorRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ScissorRect {
    #[inline]
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipMode {
    Inherit,
    Disabled,
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceClip {
    pub mode: ClipMode,
    pub rect: Option<ClipRect>,
}

impl InstanceClip {
    #[inline]
    pub fn inherit() -> Self {
        Self {
            mode: ClipMode::Inherit,
            rect: None,
        }
    }

    #[inline]
    pub fn disabled() -> Self {
        Self {
            mode: ClipMode::Disabled,
            rect: None,
        }
    }

    #[inline]
    pub fn explicit(rect: ClipRect) -> Self {
        Self {
            mode: ClipMode::Explicit,
            rect: Some(rect),
        }
    }

    #[inline(always)]
    pub fn from_instance(instance: &WidgetInstance) -> Self {
        if instance.use_clip > 0.5 {
            if let Some(rect) = ClipRect::from_min_max(instance.clip_min, instance.clip_max) {
                Self::explicit(rect)
            } else {
                Self::disabled()
            }
        } else {
            Self::inherit()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClipStack {
    stack: Vec<ClipRect>,
}

impl ClipStack {
    #[inline]
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    #[inline]
    pub fn current(&self) -> Option<ClipRect> {
        self.stack.last().copied()
    }

    pub fn push(&mut self, rect: ClipRect) -> Option<ClipToken> {
        let next = match self.current() {
            Some(cur) => cur.intersection(rect)?,
            None => rect,
        };
        self.stack.push(next);
        Some(ClipToken {
            depth_after_push: self.stack.len(),
        })
    }

    pub fn push_raw(&mut self, rect: ClipRect) -> ClipToken {
        self.stack.push(rect);
        ClipToken {
            depth_after_push: self.stack.len(),
        }
    }

    pub fn pop(&mut self, token: ClipToken) {
        debug_assert!(token.depth_after_push <= self.stack.len());
        while self.stack.len() >= token.depth_after_push {
            self.stack.pop();
        }
    }

    #[inline(always)]
    pub fn resolve(&self, clip: InstanceClip) -> Option<Option<ClipRect>> {
        match clip.mode {
            ClipMode::Disabled => Some(None),
            ClipMode::Inherit => Some(self.current()),
            ClipMode::Explicit => match (self.current(), clip.rect) {
                (Some(parent), Some(explicit)) => parent.intersection(explicit).map(Some),
                (None, Some(explicit)) => Some(Some(explicit)),
                (_, None) => None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipToken {
    depth_after_push: usize,
}

#[inline]
pub fn instance_bounds(instance: &WidgetInstance) -> Option<ClipRect> {
    ClipRect::new(
        instance.pos[0],
        instance.pos[1],
        instance.size[0],
        instance.size[1],
    )
}

#[inline(always)]
pub fn sanitize_instance(instance: &mut WidgetInstance) -> bool {
    let finite = simd::all_finite2(instance.pos)
        && simd::all_finite2(instance.size)
        && simd::all_finite4(instance.color)
        && instance.radius.is_finite()
        && instance.mode.is_finite()
        && instance.use_clip.is_finite();

    if !finite || instance.size[0] <= 0.0 || instance.size[1] <= 0.0 {
        return false;
    }

    if instance.use_clip > 0.5
        && (!simd::all_finite2(instance.clip_min) || !simd::all_finite2(instance.clip_max))
    {
        return false;
    }

    let max_radius = instance.size[0].min(instance.size[1]) * 0.5;
    instance.radius = instance.radius.max(0.0).min(max_radius);
    instance.color = simd::clamp01_f32x4(instance.color);

    true
}

#[inline]
pub fn clip_info(rect: Option<Rect>) -> ([f32; 2], [f32; 2], f32) {
    match rect.and_then(ClipRect::from_rect) {
        Some(clip) => (clip.min(), clip.max(), 1.0),
        None => ([0.0; 2], [CLIP_SENTINEL_MAX; 2], 0.0),
    }
}

#[inline]
pub fn text_bounds_for_rect(rect: Rect) -> TextBounds {
    ClipRect::from_rect(rect)
        .map(ClipRect::text_bounds)
        .unwrap_or(TextBounds {
            left: 0,
            top: 0,
            right: CLIP_SENTINEL_MAX as i32,
            bottom: CLIP_SENTINEL_MAX as i32,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_intersection_rejects_empty() {
        let a = ClipRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let b = ClipRect {
            x: 20.0,
            y: 20.0,
            width: 10.0,
            height: 10.0,
        };
        assert_eq!(a.intersection(b), None);
    }

    #[test]
    fn clip_intersection_returns_overlap() {
        let a = ClipRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let b = ClipRect {
            x: 5.0,
            y: 4.0,
            width: 10.0,
            height: 10.0,
        };
        assert_eq!(
            a.intersection(b),
            Some(ClipRect {
                x: 5.0,
                y: 4.0,
                width: 5.0,
                height: 6.0
            })
        );
    }

    #[test]
    fn sanitize_clamps_radius_and_color() {
        let mut inst = WidgetInstance {
            pos: [0.0, 0.0],
            size: [10.0, 4.0],
            color: [-1.0, 0.5, 2.0, 1.2],
            radius: 99.0,
            ..Default::default()
        };
        assert!(sanitize_instance(&mut inst));
        assert_eq!(inst.radius, 2.0);
        assert_eq!(inst.color, [0.0, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn sanitize_skips_inactive_clip_bounds() {
        let mut inst = WidgetInstance {
            pos: [0.0, 0.0],
            size: [10.0, 10.0],
            color: [1.0; 4],
            use_clip: 0.0,
            clip_min: [f32::NAN, f32::NAN],
            clip_max: [f32::NAN, f32::NAN],
            ..Default::default()
        };
        assert!(sanitize_instance(&mut inst));
    }

    #[test]
    fn sanitize_rejects_invalid_active_clip_bounds() {
        let mut inst = WidgetInstance {
            pos: [0.0, 0.0],
            size: [10.0, 10.0],
            color: [1.0; 4],
            use_clip: 1.0,
            clip_min: [f32::NAN, 0.0],
            clip_max: [10.0, 10.0],
            ..Default::default()
        };
        assert!(!sanitize_instance(&mut inst));
    }
}
