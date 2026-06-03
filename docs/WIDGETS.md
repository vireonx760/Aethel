# Widgets

The 0.3.0 developer-preview API exposes basic widgets through `aethel_gui::prelude::*`.

## Immediate-Style Use

```rust
ui.label("Settings");

if ui.button("Apply").clicked() {
    println!("apply");
}

let mut enabled = true;
ui.checkbox("Enabled", &mut enabled);
```

## Retained State

`run_ui` rebuilds the widget list every frame, but matching keys reuse previous widget instances. This preserves internal state such as text-input focus/cursor state and slider dragging.

Use `ui.with_id(...)` to disambiguate repeated labels:

```rust
ui.with_id("left_panel", |ui| {
    ui.button("Apply");
});
ui.with_id("right_panel", |ui| {
    ui.button("Apply");
});
```

Duplicate keys are reported through `UiState::diagnostics()`.
