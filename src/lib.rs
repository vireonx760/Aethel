// src/lib.rs
pub mod color;
pub mod commands;
pub mod core;
pub mod experimental;
pub mod gpu_core;
pub mod gui;
pub mod layout;
pub mod prelude;
pub mod primitives;
pub mod style;
pub mod ui;
pub mod widgets;

mod app;

pub use app::{AethelGui, AethelRunError, Result};
pub use gui::binding::{
    BoolSignal, F32Signal, I32Signal, SelectionSignal, TextSignal, U32Signal, VecSignal,
};
pub use gui::command::{CommandId, CommandIds, CommandPayload, CommandQueue, UiCommand};
pub use gui::shader::{CustomShader, CustomShaderRegistry};
pub use gui::widget::{GuiManager, UiController};
pub use gui::widget_builder::{BuiltRect, BuiltText, BuiltWidget, WidgetBuilder};
pub use ui::{Response, Ui, UiDiagnostics, UiLayout, UiState};
