use crate::core::simd;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    #[inline]
    pub fn zero() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    #[inline]
    pub fn new(w: f32, h: f32) -> Self {
        Self {
            width: w,
            height: h,
        }
    }
    pub const ZERO: Size = Size {
        width: 0.0,
        height: 0.0,
    };
    pub const INFINITY: Size = Size {
        width: f32::INFINITY,
        height: f32::INFINITY,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    #[inline]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[inline]
    pub fn from_pos_size(pos: Point, size: Size) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
        }
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }
    #[inline]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    #[inline]
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    pub fn intersect(&self, other: Rect) -> Option<Rect> {
        let [lx, ly, rx, ry] = simd::intersect_ltrb(
            [self.x, self.y, self.right(), self.bottom()],
            [other.x, other.y, other.right(), other.bottom()],
        )?;
        Some(Rect::new(lx, ly, rx - lx, ry - ly))
    }
}

//

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl BoxConstraints {
    #[inline]
    pub fn new(min_w: f32, max_w: f32, min_h: f32, max_h: f32) -> Self {
        let min_w = min_w.max(0.0);
        let min_h = min_h.max(0.0);
        Self {
            min_width: min_w,
            max_width: max_w.max(min_w),
            min_height: min_h,
            max_height: max_h.max(min_h),
        }
    }

    #[inline]
    pub fn tight(size: Size) -> Self {
        Self::new(size.width, size.width, size.height, size.height)
    }

    #[inline]
    pub fn loose(size: Size) -> Self {
        Self::new(0.0, size.width, 0.0, size.height)
    }

    #[inline]
    pub fn width_only(max_w: f32) -> Self {
        Self::new(0.0, max_w, 0.0, f32::INFINITY)
    }

    #[inline]
    pub fn constrain(&self, size: Size) -> Size {
        let [width, height] = simd::clamp_f32x2(
            [size.width, size.height],
            [self.min_width, self.min_height],
            [self.max_width, self.max_height],
        );
        Size { width, height }
    }

    #[inline]
    pub fn constrain_max(&self, size: Size) -> Size {
        let [width, height] = simd::clamp_f32x2(
            [size.width, size.height],
            [f32::NEG_INFINITY, f32::NEG_INFINITY],
            [self.max_width, self.max_height],
        );
        Size { width, height }
    }

    #[inline]
    pub fn max_size(&self) -> Size {
        Size::new(self.max_width, self.max_height)
    }
    #[inline]
    pub fn has_bounded_width(&self) -> bool {
        self.max_width < f32::INFINITY
    }
    #[inline]
    pub fn has_bounded_height(&self) -> bool {
        self.max_height < f32::INFINITY
    }
}

impl Default for BoxConstraints {
    fn default() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }
}
