// src/lib.rs
pub mod core;
pub mod gpu_core;
pub mod gui;
pub mod primitives;
pub mod widgets;

mod app;

pub use app::AethelGui;
pub use gui::binding::{
    BoolSignal, F32Signal, I32Signal, SelectionSignal, TextSignal, U32Signal, VecSignal,
};
pub use gui::command::{CommandId, CommandIds, CommandPayload, CommandQueue, UiCommand};
pub use gui::shader::{CustomShader, CustomShaderRegistry};
pub use gui::widget::{GuiManager, UiController};
pub use gui::widget_builder::{BuiltRect, BuiltText, BuiltWidget, WidgetBuilder};
