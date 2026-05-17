// primitives/mod.rs

pub mod builder;
pub mod line;
pub mod point;
#[allow(clippy::module_inception)]
pub mod primitives;
pub mod rect;
pub mod triangle;

pub use builder::{Camera3d, Point3, PrimitiveBatch, PrimitiveBuilder, PrimitiveSink};
pub use line::Line;
pub use point::Point;
pub use primitives::PrimitiveWidget;
pub use rect::Rect;
pub use triangle::Triangle;
