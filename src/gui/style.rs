use crate::core::renderer::WidgetInstance;
use crate::core::simd;
use crate::gui::paint::{PaintRect, ShaderMode};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub const TRANSPARENT: Self = Self::new_const(0.0, 0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::new_const(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new_const(0.0, 0.0, 0.0, 1.0);

    #[inline]
    pub const fn new_const(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }.clamped()
    }

    #[inline]
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    #[inline]
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    #[inline]
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[inline]
    pub fn with_alpha(self, alpha: f32) -> Self {
        Self::new(self.r, self.g, self.b, alpha)
    }

    #[inline]
    pub fn multiply_alpha(self, alpha: f32) -> Self {
        Self::new(self.r, self.g, self.b, self.a * alpha)
    }

    #[inline]
    pub fn lighten(self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self::new(
            self.r + (1.0 - self.r) * amount,
            self.g + (1.0 - self.g) * amount,
            self.b + (1.0 - self.b) * amount,
            self.a,
        )
    }

    #[inline]
    pub fn darken(self, amount: f32) -> Self {
        let scale = 1.0 - amount.clamp(0.0, 1.0);
        Self::new(self.r * scale, self.g * scale, self.b * scale, self.a)
    }

    #[inline]
    pub fn mix(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::new(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }

    #[inline]
    pub fn clamped(self) -> Self {
        let [r, g, b, a] = simd::clamp01_f32x4([self.r, self.g, self.b, self.a]);
        Self { r, g, b, a }
    }
}

impl From<[f32; 4]> for ColorRgba {
    fn from(value: [f32; 4]) -> Self {
        Self::new(value[0], value[1], value[2], value[3])
    }
}

impl From<ColorRgba> for [f32; 4] {
    fn from(value: ColorRgba) -> Self {
        value.to_array()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self::same_const(0.0);

    #[inline]
    pub const fn same_const(value: f32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    #[inline]
    pub fn same(value: f32) -> Self {
        Self::same_const(value.max(0.0))
    }

    #[inline]
    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal.max(0.0),
            top: vertical.max(0.0),
            right: horizontal.max(0.0),
            bottom: vertical.max(0.0),
        }
    }

    #[inline]
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left: left.max(0.0),
            top: top.max(0.0),
            right: right.max(0.0),
            bottom: bottom.max(0.0),
        }
    }

    #[inline]
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    #[inline]
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }

    #[inline]
    pub fn to_array_ltrb(self) -> [f32; 4] {
        [self.left, self.top, self.right, self.bottom]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadius {
    pub const ZERO: Self = Self::same_const(0.0);

    #[inline]
    pub const fn same_const(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }

    #[inline]
    pub fn same(value: f32) -> Self {
        Self::same_const(value.max(0.0))
    }

    #[inline]
    pub fn uniform_for_shader(self) -> f32 {
        self.top_left
            .min(self.top_right)
            .min(self.bottom_right)
            .min(self.bottom_left)
            .max(0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub color: ColorRgba,
}

impl Stroke {
    #[inline]
    pub fn new(width: f32, color: impl Into<ColorRgba>) -> Option<Self> {
        let width = width.max(0.0);
        if width <= 0.0 {
            None
        } else {
            Some(Self {
                width,
                color: color.into(),
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualState<T> {
    pub normal: T,
    pub hovered: T,
    pub pressed: T,
    pub disabled: T,
}

impl<T: Copy> VisualState<T> {
    #[inline]
    pub fn resolve(self, hovered: bool, pressed: bool, enabled: bool) -> T {
        if !enabled {
            self.disabled
        } else if pressed {
            self.pressed
        } else if hovered {
            self.hovered
        } else {
            self.normal
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceStyle {
    pub fill: ColorRgba,
    pub radius: CornerRadius,
    pub stroke: Option<Stroke>,
    pub shader_mode: ShaderMode,
}

impl SurfaceStyle {
    #[inline]
    pub fn new(fill: impl Into<ColorRgba>) -> Self {
        Self {
            fill: fill.into(),
            radius: CornerRadius::ZERO,
            stroke: None,
            shader_mode: ShaderMode::Solid,
        }
    }

    #[inline]
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = CornerRadius::same(radius);
        self
    }

    #[inline]
    pub fn stroke(mut self, stroke: Option<Stroke>) -> Self {
        self.stroke = stroke;
        self
    }

    #[inline]
    pub fn shader_mode(mut self, mode: ShaderMode) -> Self {
        self.shader_mode = mode;
        self
    }

    #[inline]
    pub fn rect(self, pos: [f32; 2], size: [f32; 2]) -> PaintRect {
        PaintRect::new(pos, size, self.fill.to_array())
            .radius(self.radius.uniform_for_shader())
            .mode(self.shader_mode)
    }

    #[inline]
    pub fn instance(self, pos: [f32; 2], size: [f32; 2]) -> WidgetInstance {
        self.rect(pos, size).into_instance()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub line_height: f32,
    pub color: ColorRgba,
}

impl TextStyle {
    #[inline]
    pub fn new(size: f32, color: impl Into<ColorRgba>) -> Self {
        let size = size.max(1.0);
        Self {
            size,
            line_height: size * 1.2,
            color: color.into(),
        }
    }

    #[inline]
    pub fn line_height(mut self, value: f32) -> Self {
        self.line_height = value.max(self.size);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub panel: SurfaceStyle,
    pub button: VisualState<SurfaceStyle>,
    pub checkbox: VisualState<SurfaceStyle>,
    pub accent: ColorRgba,
    pub text: TextStyle,
    pub muted_text: TextStyle,
    pub padding: EdgeInsets,
}

impl Theme {
    pub fn dark() -> Self {
        let accent = ColorRgba::rgb(0.0, 0.65, 0.85);
        Self {
            panel: SurfaceStyle::new([0.13, 0.13, 0.16, 0.96]).radius(12.0),
            button: VisualState {
                normal: SurfaceStyle::new([0.25, 0.25, 0.28, 1.0]).radius(8.0),
                hovered: SurfaceStyle::new([0.35, 0.35, 0.38, 1.0]).radius(8.0),
                pressed: SurfaceStyle::new([0.20, 0.20, 0.23, 1.0]).radius(8.0),
                disabled: SurfaceStyle::new([0.15, 0.15, 0.18, 1.0]).radius(8.0),
            },
            checkbox: VisualState {
                normal: SurfaceStyle::new([0.22, 0.22, 0.25, 1.0]).radius(4.0),
                hovered: SurfaceStyle::new([0.28, 0.28, 0.32, 1.0]).radius(4.0),
                pressed: SurfaceStyle::new([0.18, 0.18, 0.22, 1.0]).radius(4.0),
                disabled: SurfaceStyle::new([0.12, 0.12, 0.14, 1.0]).radius(4.0),
            },
            accent,
            text: TextStyle::new(16.0, [0.9, 0.9, 0.95, 1.0]),
            muted_text: TextStyle::new(14.0, [0.6, 0.6, 0.68, 1.0]),
            padding: EdgeInsets::same(8.0),
        }
    }

    pub fn high_contrast() -> Self {
        let mut theme = Self::dark();
        theme.accent = ColorRgba::rgb(0.2, 0.8, 1.0);
        theme.text = TextStyle::new(17.0, ColorRgba::WHITE);
        theme.muted_text = TextStyle::new(15.0, [0.78, 0.78, 0.82, 1.0]);
        theme
    }

    #[inline]
    pub fn button_style(&self, hovered: bool, pressed: bool, enabled: bool) -> SurfaceStyle {
        self.button.resolve(hovered, pressed, enabled)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_mix_interpolates_alpha() {
        let a = ColorRgba::new(0.0, 0.0, 0.0, 0.0);
        let b = ColorRgba::new(1.0, 1.0, 1.0, 1.0);
        assert_eq!(a.mix(b, 0.5), ColorRgba::new(0.5, 0.5, 0.5, 0.5));
    }

    #[test]
    fn visual_state_resolves_disabled_first() {
        let state = VisualState {
            normal: 1,
            hovered: 2,
            pressed: 3,
            disabled: 4,
        };
        assert_eq!(state.resolve(true, true, false), 4);
    }
}
