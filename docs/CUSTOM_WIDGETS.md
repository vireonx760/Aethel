# Custom Widgets

There are two supported authoring paths in 0.3.0.

## Retained Widget Trait

For low-level retained widgets, implement `gui::widget::Widget`:

```rust
use aethel_gui::prelude::*;
use aethel_gui::gui::widget::Widget;
use aethel_gui::core::renderer::WidgetInstance;

struct Meter;

impl Widget for Meter {
    fn update(&mut self, _dt: f32, _input: &aethel_gui::core::input::InputManager) {}

    fn instances(&self) -> Vec<WidgetInstance> {
        Vec::new()
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
```

## Immediate-Style Composition

For application code, prefer composing existing widgets with `Ui` first. This keeps retained state reuse automatic and avoids exposing renderer details.

Custom paint builders and a polished custom-widget facade are planned after the core 0.3 API settles.
