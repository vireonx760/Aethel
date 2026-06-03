pub use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
pub use crate::gui::layout_enums::{Axis, CrossAxisAlignment, MainAxisAlignment};
pub use crate::gui::style::EdgeInsets;

pub type Constraints = BoxConstraints;
pub type Direction = Axis;
pub type Align = MainAxisAlignment;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub direction: Direction,
    pub spacing: f32,
    pub padding: EdgeInsets,
}

impl Layout {
    pub fn row() -> Self {
        Self {
            direction: Direction::Horizontal,
            spacing: 8.0,
            padding: EdgeInsets::ZERO,
        }
    }

    pub fn column() -> Self {
        Self {
            direction: Direction::Vertical,
            spacing: 8.0,
            padding: EdgeInsets::ZERO,
        }
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = EdgeInsets::same(padding);
        self
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::column()
    }
}
