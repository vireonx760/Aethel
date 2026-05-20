# AethelGUI

AethelGUI is an attempt at a retained/immediate hybrid Rust GUI runtime built on `wgpu`, `winit`, and `glyphon`.
Version `0.1.4` is the migration baseline toward the `0.2.0` foundation: current crates, explicit renderer errors, overlay-safe painting, retained scratch storage, and CI-ready verification.

## Highlights

- Event-driven frame scheduler with idle `Wait` behavior and dirty redraws.
- Batched widget rendering with clip/scissor support.
- Layered text rendering for panel and overlay ordering.
- Typed bindings and command helpers for low-overhead widget interactions.
- Custom WGSL shader pipelines selected per render batch.
- Retained scratch storage for paint/text paths to avoid warm-frame allocations.
- SIMD fast paths for instance validation, color clamps, constraints, rect intersection, and primitive translation.
- Primitive builder for retained 2D primitives and projected 3D wireframes.
- `gpu_core` accelerator with retained instance buffers, custom pipeline cache, scissor-aware draw planning, and per-frame GPU stats.

## Quick Start

```rust
use aethel_gui::{AethelGui, widgets::*};

fn main() -> Result<(), aethel_gui::AethelRunError> {
    AethelGui::new()
        .title("AethelGUI")
        .size(1200, 800)
        .run(|gui| {
            gui.add(Label::new([24.0, 24.0], "AethelGUI").scale(32.0));
            gui.add(Button::new([24.0, 84.0], [180.0, 42.0], "Run"));
        })
}
```

Run the main demo:

```powershell
cargo run --release
```

Run the GPU scene stress demo:

```powershell
cargo run --release --example demo
```

`demo` renders a procedural three-star sandbox with thousands of instanced asteroids, orbit prediction lines, a custom WGSL space shader, and an AethelGUI editor overlay. Drag in the space viewport to launch bodies, right-drag to pan, and use the mouse wheel to zoom.

Run CPU-side benchmarks:

```powershell
cargo run --release --example bench -- --quick
cargo run --release --example bench -- --save bench-current.tsv
cargo run --release --example bench -- --baseline bench-current.tsv
```

The benchmark harness covers instance validation, SIMD translation, paint batching, clip culling, primitive building, scratch reuse, and command queue emission.

## Custom Shader Widgets

Widgets can expose custom shaders through `Widget::custom_shaders()`. The renderer compiles each shader once after the GUI is built and switches pipelines by `ShaderMode::Custom(mode)`.

Use `FIRST_CUSTOM_SHADER_MODE` as the start of the custom mode range.

## Primitive Builder

`PrimitiveBuilder` creates retained primitive batches without forcing widget authors to allocate per frame.

```rust
use aethel_gui::primitives::{Camera3d, Point3, PrimitiveBuilder};

let camera = Camera3d::new([300.0, 240.0]).scale(2.0).perspective(0.001);
let mut builder = PrimitiveBuilder::with_capacity(64);
builder
    .rect_xywh(20.0, 20.0, 160.0, 80.0, [0.2, 0.6, 1.0, 1.0], 8.0)
    .cube_wireframe(camera, Point3::new(0.0, 0.0, 0.0), 80.0, [1.0; 4], 2.0);

let widget = builder.build_widget();
```

## GPU Core

The `gpu_core` module owns the low-level GPU acceleration path used by the renderer:

- `InstanceBufferArena` grows geometrically and reuses GPU memory after warm-up.
- `PipelineCache` stores the base GUI pipeline and custom WGSL pipelines keyed by shader mode.
- `DrawPlanner` precomputes scissor visibility, pipeline switches, and draw packets without per-draw allocation.
- `GpuStats` exposes upload size, buffer growth, draw packets, skipped batches, and custom pipeline count.

`Renderer` uses this module internally, so existing widgets benefit from it without API changes.

More detail is documented in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Frame Scheduling

AethelGUI does not rely on a fixed FPS cap by default. Widgets can request a repaint interval through `Widget::repaint_interval()`, and `GuiManager::next_repaint_interval()` feeds the `FrameScheduler`. Idle frames use `ControlFlow::Wait`; focused text, active drags, popups, and animated shaders use `WaitUntil` deadlines so they continue updating even when the mouse is still.

## Verification

Release verification used for this baseline:

```powershell
cargo fmt --all
cargo check --release
cargo clippy --release --all-targets -- -D warnings
cargo test --release
cargo run --release --example bench -- --quick
cargo build --release --examples
cargo build --release
```

## Notes

- The current custom shader pipeline uses the same instance vertex format as built-in widgets.
- True backdrop blur requires rendering the scene into a texture and sampling it from a custom shader. The included liquid-glass demo is a procedural material that approximates frosted glass, rim lensing, and caustics without a backbuffer sample.
- Compatibility APIs that return `Vec<WidgetInstance>` remain available, but hot paths should prefer `paint(&mut PaintCtx)` and retained builders.
