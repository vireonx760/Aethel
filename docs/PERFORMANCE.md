# Performance

AethelGUI performance work is split into CPU hot paths and GPU renderer behavior.

## CPU Benchmarks

Run:

```powershell
cargo run --release --example bench
```

Quick CI smoke:

```powershell
cargo run --release --example bench -- --quick
```

The benchmark harness covers instance validation, SIMD translation, paint batching, clip culling, primitive building, scratch reuse, and command queue emission.

## GPU Profiling

The 0.3.0 developer preview does not claim GPU timestamp numbers yet. GPU timestamp profiling should be added once renderer profiling APIs are stable.

Renderer and GPU stats are available under `aethel_gui::experimental::gpu_stats` for application-level diagnostics.
