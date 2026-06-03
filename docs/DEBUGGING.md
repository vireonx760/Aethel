# Debugging

Developer-facing diagnostics in 0.3.0 focus on API misuse that can break retained state.

## Duplicate Keys

Repeated labels in the same `Ui` scope can produce duplicate widget keys. AethelGUI keeps the frame working by generating a salted fallback id, and records the duplicate in `UiDiagnostics`.

Use explicit scopes to avoid ambiguity:

```rust
ui.with_id("sidebar", |ui| {
    ui.button("Apply");
});
ui.with_id("inspector", |ui| {
    ui.button("Apply");
});
```

## Frame Stats

Low-level frame and GPU stats are available through `aethel_gui::experimental::gpu_stats`.

CPU hot-path benchmark documentation is in `docs/BENCHMARKS.md`.
