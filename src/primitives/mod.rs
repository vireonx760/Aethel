// primitives/mod.rs

pub mod builder;
pub mod line;
pub mod point;
pub mod rect;
pub mod triangle;
#[path = "primitives.rs"]
pub mod widget;

pub use builder::{Camera3d, Point3, PrimitiveBatch, PrimitiveBuilder, PrimitiveSink};
pub use line::Line;
pub use point::Point;
pub use rect::Rect;
pub use triangle::Triangle;
pub use widget::PrimitiveWidget;
