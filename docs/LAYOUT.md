# Layout

AethelGUI 0.3.0 intentionally keeps layout small:

- `ui.column(...)`
- `ui.row(...)`
- `ui.panel(...)`
- `ui.panel_with(...)`
- simple spacing and padding through `UiLayout`

This is not full flexbox or grid. The goal is predictable layout for tools, editors, overlays, and examples.

```rust
ui.panel_with("settings", [360.0, 240.0], |ui| {
    ui.row(|ui| {
        ui.button("Save");
        ui.button("Cancel");
    });

    ui.column(|ui| {
        ui.label("Exposure");
        ui.slider("exposure", &mut exposure, 0.0..=1.0);
    });
});
```

The legacy retained `Flex` container remains available for lower-level retained composition.
