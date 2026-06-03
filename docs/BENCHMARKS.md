# Benchmarks

AethelGUI ships a dependency-free CPU benchmark harness as a release example:

```powershell
cargo run --release --example bench
```

For CI or quick smoke checks:

```powershell
cargo run --release --example bench -- --quick
```

To save a baseline and compare a later commit:

```powershell
cargo run --release --example bench -- --save bench-main.tsv
cargo run --release --example bench -- --baseline bench-main.tsv
```

The baseline file stores best and mean nanoseconds per benchmark iteration as tab-separated text. The comparison column reports `old best ns/it / current best ns/it`, so values above `1.00x` are faster than the baseline.

## Covered Paths

- `sanitize_instances`: validates and clamps `WidgetInstance` data through the same helper used by paint collection.
- `translate_instances`: measures the SIMD/scalar translation path used when retained primitives move.
- `paint_batches`: pushes many rectangles through `PaintCtx` and records batch splitting by shader mode.
- `paint_clip_cull`: exercises clip resolution, instance bounds, and culling.
- `primitive_builder`: builds retained primitive line batches and a projected cube wireframe.
- `scratch_reuse`: checks warm-frame scratch buffer checkout and return behavior.
- `command_queue`: measures command emission and retained queue capacity reuse.

These benchmarks intentionally avoid window creation and GPU submission, so they are deterministic enough for CI smoke checks and local regression tracking. GPU timing should be added separately with timestamp queries once renderer profiling APIs are stable.

## Optimization Notes

The `0.3.0` CPU pass targets the paths that showed the clearest sustained wins:

- inactive clip bounds are no longer validated for every unclipped instance;
- unclipped instances without an active clip return before clip-stack resolution;
- custom shader batch keys return early for built-in shader modes before checking `is_finite()`;
- `ClipRect::intersects` computes edge values once per comparison;
- retained primitive line bounds use conservative endpoint bounds instead of recomputing rotated rectangle bounds with `sin`/`cos` for every segment.
- scratch pools track retained capacity incrementally so frame snapshots do not scan idle buffers.

Avoid accepting a micro-optimization solely because the Rust source looks shorter. The `paint_batches` path is sensitive to branch shape and inlining; attempted direct `push_rect` fast paths regressed on the benchmark harness and were not kept.

## Latest Benchmark Results

Measured on June 2, 2026 with `cargo run --release --example bench`. No saved baseline TSV was present in the workspace for this run, so speedup is not reported here.

| Benchmark | Iterations | Best ns/it | Mean ns/it | Throughput | Speedup |
|---|---:|---:|---:|---:|---:|
| `sanitize_instances` | 180 | 66,378.9 | 72,215.7 | 246.83M instances/s | - |
| `translate_instances` | 220 | 13,300.0 | 14,468.8 | 1.232B instances/s | - |
| `paint_batches` | 220 | 89,979.1 | 94,170.0 | 91.04M rects/s | - |
| `paint_clip_cull` | 220 | 82,675.9 | 84,914.8 | 99.09M rects/s | - |
| `primitive_builder` | 320 | 48,284.7 | 51,884.4 | 84.83M segments/s | - |
| `scratch_reuse` | 18,000 | 48.3 | 50.9 | 20.70M frames/s | - |
| `command_queue` | 1,800 | 19,628.0 | 20,810.1 | 417.36M commands/s | - |

Values above `1.00x` are faster than the saved baseline. These are CPU-side microbenchmarks measured in release mode; they intentionally exclude window creation, GPU upload, render pass execution, presentation, and driver scheduling.
