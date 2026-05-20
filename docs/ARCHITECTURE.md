# AethelGUI Architecture

This document describes the `0.1.4` baseline and the direction for the `0.2.0` foundation.

## Retained-First Model

AethelGUI keeps widget objects and frame-owned rendering state across frames. Widgets still expose immediate-style `paint(&mut PaintCtx)` hooks, but hot paths should push into retained frame structures instead of allocating standalone `Vec<WidgetInstance>` values every frame.

The compatibility method `instances()` remains available for simple widgets and existing integrations. New widgets should prefer `paint()`, `paint_overlay()`, and retained primitive batches because those APIs preserve batching, clipping, and scratch-buffer reuse.

## Widget Identity

`GuiManager::add` returns a stable widget index for the lifetime of the manager. Higher-level grouping APIs, including clip groups, relayout groups, and panel z-order, store those indices rather than borrowing widget references. This keeps identity stable while allowing the manager to update layout and paint ordering in separate passes.

The current identity model is intentionally simple. Before `0.2.0`, public APIs should avoid implying that indices remain valid after a future removal API unless the removal behavior is explicitly defined.

## Dirty Scheduling

The app layer uses event-driven `winit` scheduling. Widgets can request animation with `requests_repaint()` or a custom `repaint_interval()`. The scheduler waits when the scene is idle and requests redraws only when input, layout, overlay state, or widget repaint intervals require it.

Dirty state is still coarse at the manager level. The stable target is to keep the external API retained-first while making internal dirty flags more granular: layout dirty, paint dirty, text dirty, and GPU upload dirty.

## Paint, Overlay, and Text Ordering

Painting is collected into `FramePaint`, which owns the frame's widget instances and render batches. Regular widgets are painted first, panel groups are painted according to z-order, and overlay widgets are painted into an explicit overlay layer.

Text is prepared after paint collection. `GuiManager` records text layer boundaries from paint ordering so text can be rendered at the same logical layer as its widget instances. Overlay text is prepared and rendered after regular text, matching overlay instances.

## GPU Path

The renderer delegates low-level GPU work to `gpu_core`:

- `InstanceBufferArena` retains GPU buffers and grows geometrically.
- `PipelineCache` stores the built-in GUI pipeline and custom WGSL pipelines.
- `DrawPlanner` resolves scissor visibility, shader switches, and draw packets before issuing render commands.
- `GpuStats` reports upload and draw behavior for profiling.

This keeps the renderer responsible for surface lifecycle, text, and high-level ordering, while `gpu_core` owns reusable GPU acceleration mechanics.

## Error Handling

Renderer initialization returns `RendererInitError` instead of panicking. Recoverable runtime errors, such as text preparation failures, are logged and skipped for the frame so the app can keep running.

Code that relies on internal invariants should prefer explicit `match` handling and `unreachable!` only when the invariant cannot be represented in the return type, such as `Deref` for a consumed scratch lease.

## Custom Shaders

Widgets can return `CustomShader` values from `custom_shaders()`. The renderer registers each custom WGSL source once and uses `ShaderMode::Custom` batch keys to switch pipelines during draw planning.

Custom shaders currently use the same instance vertex format as built-in widgets. A future API should make that constraint explicit before `0.2.0`.
