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

The `0.1.4` CPU pass targets the paths that showed the clearest sustained wins:

- inactive clip bounds are no longer validated for every unclipped instance;
- custom shader batch keys return early for built-in shader modes before checking `is_finite()`;
- `ClipRect::intersects` computes edge values once per comparison;
- retained primitive line bounds use conservative endpoint bounds instead of recomputing rotated rectangle bounds with `sin`/`cos` for every segment.
- scratch pools track retained capacity incrementally so frame snapshots do not scan idle buffers.

Avoid accepting a micro-optimization solely because the Rust source looks shorter. The `paint_batches` path is sensitive to branch shape and inlining; attempted direct `push_rect` fast paths regressed on the benchmark harness and were not kept.

## Latest Benchmark Results

| Benchmark | Iterations | Best ns/it | Mean ns/it | Throughput | Speedup |
|---|---:|---:|---:|---:|---:|
| `sanitize_instances` | 180 | 57,378.3 | 62,551.5 | 285.54M instances/s | 1.23x |
| `translate_instances` | 220 | 11,906.8 | 12,365.7 | 1.376B instances/s | 0.98x |
| `paint_batches` | 220 | 90,822.7 | 92,533.5 | 90.20M rects/s | 0.90x |
| `paint_clip_cull` | 220 | 87,588.2 | 88,841.2 | 93.53M rects/s | 1.06x |
| `primitive_builder` | 320 | 76,525.6 | 84,965.5 | 53.52M segments/s | 1.39x |
| `scratch_reuse` | 18,000 | 47.4 | 50.1 | 21.08M frames/s | 0.90x |
| `command_queue` | 1,800 | 17,137.1 | 18,430.9 | 478.03M commands/s | 0.96x |

Values above `1.00x` are faster than the saved baseline. These are CPU-side microbenchmarks measured in release mode; they intentionally exclude window creation, GPU upload, render pass execution, presentation, and driver scheduling.
