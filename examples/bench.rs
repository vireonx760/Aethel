use aethel_gui::core::renderer::WidgetInstance;
use aethel_gui::core::scratch::FrameScratch;
use aethel_gui::core::simd;
use aethel_gui::gui::clip::{ClipRect, sanitize_instance};
use aethel_gui::gui::command::{CommandId, CommandQueue};
use aethel_gui::gui::paint::{
    FIRST_CUSTOM_SHADER_MODE, FramePaint, PaintCtx, PaintRect, ShaderMode,
};
use aethel_gui::primitives::{Point3, PrimitiveBuilder};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const INSTANCE_COUNT: usize = 16_384;
const PAINT_COUNT: usize = 8_192;
const POINT_COUNT: usize = 4_096;
const COMMAND_COUNT: usize = 8_192;

struct Options {
    quick: bool,
    save: Option<PathBuf>,
    baseline: Option<PathBuf>,
}

struct BenchConfig {
    warmups: usize,
    samples: usize,
}

#[derive(Clone)]
struct BenchResult {
    name: &'static str,
    iterations: u64,
    work_items: u64,
    unit: &'static str,
    best: Duration,
    mean: Duration,
    checksum: u64,
}

impl BenchResult {
    fn best_ns_per_iter(&self) -> f64 {
        self.best.as_nanos() as f64 / self.iterations as f64
    }

    fn mean_ns_per_iter(&self) -> f64 {
        self.mean.as_nanos() as f64 / self.iterations as f64
    }

    fn best_items_per_second(&self) -> f64 {
        let seconds = self.best_ns_per_iter() / 1_000_000_000.0;
        if seconds <= f64::EPSILON {
            0.0
        } else {
            self.work_items as f64 / seconds
        }
    }
}

fn main() {
    let options = parse_options();
    let config = if options.quick {
        BenchConfig {
            warmups: 1,
            samples: 3,
        }
    } else {
        BenchConfig {
            warmups: 3,
            samples: 8,
        }
    };

    let results = vec![
        bench_sanitize_instances(&config),
        bench_translate_instances(&config),
        bench_paint_batches(&config),
        bench_paint_clip_cull(&config),
        bench_primitive_builder(&config),
        bench_scratch_reuse(&config),
        bench_command_queue(&config),
    ];

    let baseline = options
        .baseline
        .as_deref()
        .and_then(|path| load_baseline(path).ok());
    print_results(&results, baseline.as_ref());

    if let Some(path) = options.save {
        match save_results(&path, &results) {
            Ok(()) => println!("\nSaved benchmark baseline to {}", path.display()),
            Err(err) => eprintln!("\nFailed to save benchmark baseline: {err}"),
        }
    }
}

fn parse_options() -> Options {
    let mut quick = false;
    let mut save = None;
    let mut baseline = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--quick" => quick = true,
            "--save" => {
                if let Some(path) = args.next() {
                    save = Some(PathBuf::from(path));
                }
            }
            "--baseline" => {
                if let Some(path) = args.next() {
                    baseline = Some(PathBuf::from(path));
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --release --example bench -- [--quick] [--save PATH] [--baseline PATH]"
                );
                println!("Baseline files are tab-separated and can be compared across commits.");
            }
            _ => {}
        }
    }

    Options {
        quick,
        save,
        baseline,
    }
}

fn run_bench(
    name: &'static str,
    iterations: u64,
    work_items: u64,
    unit: &'static str,
    config: &BenchConfig,
    mut f: impl FnMut() -> u64,
) -> BenchResult {
    for _ in 0..config.warmups {
        for _ in 0..iterations {
            black_box(f());
        }
    }

    let mut best = Duration::MAX;
    let mut total = Duration::ZERO;
    let mut checksum = 0u64;

    for _ in 0..config.samples {
        let start = Instant::now();
        for _ in 0..iterations {
            checksum = checksum.wrapping_add(black_box(f()));
        }
        let elapsed = start.elapsed();
        best = best.min(elapsed);
        total += elapsed;
    }

    BenchResult {
        name,
        iterations,
        work_items,
        unit,
        best,
        mean: total / config.samples as u32,
        checksum,
    }
}

fn bench_sanitize_instances(config: &BenchConfig) -> BenchResult {
    let mut instances = make_instances(INSTANCE_COUNT);
    run_bench(
        "sanitize_instances",
        if config.samples <= 3 { 60 } else { 180 },
        INSTANCE_COUNT as u64,
        "instances/s",
        config,
        || {
            let mut accepted = 0u64;
            for instance in &mut instances {
                if sanitize_instance(black_box(instance)) {
                    accepted += 1;
                }
            }
            accepted
        },
    )
}

fn bench_translate_instances(config: &BenchConfig) -> BenchResult {
    let mut instances = make_instances(INSTANCE_COUNT);
    run_bench(
        "translate_instances",
        if config.samples <= 3 { 80 } else { 220 },
        INSTANCE_COUNT as u64,
        "instances/s",
        config,
        || {
            simd::translate_widget_instances(&mut instances, [0.25, -0.125]);
            let first = instances.first().copied().unwrap_or_default();
            first.pos[0].to_bits() as u64 ^ first.pos[1].to_bits() as u64
        },
    )
}

fn bench_paint_batches(config: &BenchConfig) -> BenchResult {
    let mut frame = FramePaint::with_capacity(PAINT_COUNT, 256);
    run_bench(
        "paint_batches",
        if config.samples <= 3 { 80 } else { 220 },
        PAINT_COUNT as u64,
        "rects/s",
        config,
        || {
            frame.clear();
            {
                let mut ctx = PaintCtx::new(&mut frame);
                for i in 0..PAINT_COUNT {
                    let mode = if i % 17 == 0 {
                        ShaderMode::Custom(FIRST_CUSTOM_SHADER_MODE)
                    } else {
                        ShaderMode::Solid
                    };
                    ctx.push_rect(
                        PaintRect::new(
                            [(i % 512) as f32, (i / 512) as f32],
                            [18.0, 12.0],
                            [0.2, 0.5, 0.9, 1.0],
                        )
                        .radius((i % 8) as f32)
                        .mode(mode),
                    );
                }
            }
            frame.instances().len() as u64 ^ frame.batches().len() as u64
        },
    )
}

fn bench_paint_clip_cull(config: &BenchConfig) -> BenchResult {
    let mut frame = FramePaint::with_capacity(PAINT_COUNT, 256);
    let clip = ClipRect {
        x: 0.0,
        y: 0.0,
        width: 512.0,
        height: 512.0,
    };
    run_bench(
        "paint_clip_cull",
        if config.samples <= 3 { 80 } else { 220 },
        PAINT_COUNT as u64,
        "rects/s",
        config,
        || {
            frame.clear();
            {
                let mut ctx = PaintCtx::new(&mut frame);
                let token = ctx.push_clip_rect(clip);
                for i in 0..PAINT_COUNT {
                    let x = if i % 2 == 0 {
                        (i % 512) as f32
                    } else {
                        2_000.0
                    };
                    ctx.push_rect(PaintRect::new(
                        [x, (i / 512) as f32],
                        [16.0, 16.0],
                        [0.8, 0.4, 0.2, 1.0],
                    ));
                }
                if let Some(token) = token {
                    ctx.pop_clip(token);
                }
            }
            frame.instances().len() as u64 ^ frame.stats().culled_by_clip as u64
        },
    )
}

fn bench_primitive_builder(config: &BenchConfig) -> BenchResult {
    let points: Vec<[f32; 2]> = (0..POINT_COUNT)
        .map(|i| {
            let t = i as f32 * 0.025;
            [t.cos() * 240.0 + 320.0, t.sin() * 180.0 + 240.0]
        })
        .collect();
    let camera = aethel_gui::primitives::Camera3d::new([320.0, 240.0])
        .scale(2.0)
        .perspective(0.001);
    let mut builder = PrimitiveBuilder::with_capacity(POINT_COUNT + 16);

    run_bench(
        "primitive_builder",
        if config.samples <= 3 { 120 } else { 320 },
        POINT_COUNT as u64,
        "segments/s",
        config,
        || {
            builder.clear();
            builder.polyline(&points, [0.7, 0.9, 1.0, 1.0], 2.0);
            builder.cube_wireframe(camera, Point3::new(0.0, 0.0, 0.0), 80.0, [1.0; 4], 2.0);
            builder.len() as u64
        },
    )
}

fn bench_scratch_reuse(config: &BenchConfig) -> BenchResult {
    let mut scratch = FrameScratch::new();
    run_bench(
        "scratch_reuse",
        if config.samples <= 3 { 5_000 } else { 18_000 },
        1,
        "frames/s",
        config,
        || {
            scratch.begin_frame();
            {
                let mut ids = scratch.widget_indices(256);
                ids.extend(0..256);
            }
            {
                let mut dirty = scratch.dirty_widgets(128);
                dirty.extend((0..128).map(|i| i * 2));
            }
            {
                let mut text = scratch.string(512);
                text.push_str("AethelGUI retained scratch benchmark");
            }
            let snapshot = scratch.snapshot();
            snapshot.vec_capacity as u64 ^ snapshot.string_capacity as u64
        },
    )
}

fn bench_command_queue(config: &BenchConfig) -> BenchResult {
    let mut queue = CommandQueue::with_capacity(COMMAND_COUNT);
    run_bench(
        "command_queue",
        if config.samples <= 3 { 600 } else { 1_800 },
        COMMAND_COUNT as u64,
        "commands/s",
        config,
        || {
            queue.clear();
            for i in 0..COMMAND_COUNT {
                queue.emit_f32(CommandId::<f32>::from_raw(i as u64 + 1), i as f32 * 0.25);
            }
            queue.len() as u64 ^ queue.capacity() as u64
        },
    )
}

fn make_instances(count: usize) -> Vec<WidgetInstance> {
    (0..count)
        .map(|i| WidgetInstance {
            pos: [(i % 512) as f32, (i / 512) as f32],
            size: [12.0 + (i % 7) as f32, 10.0 + (i % 5) as f32],
            color: [
                (i % 255) as f32 / 255.0,
                0.45,
                1.2 - (i % 3) as f32 * 0.1,
                1.0,
            ],
            radius: 64.0,
            mode: if i % 31 == 0 {
                FIRST_CUSTOM_SHADER_MODE
            } else {
                0.0
            },
            ..Default::default()
        })
        .collect()
}

fn print_results(results: &[BenchResult], baseline: Option<&HashMap<String, f64>>) {
    println!("AethelGUI CPU benchmarks");
    println!(
        "{:<24} {:>10} {:>14} {:>14} {:>16} {:>10}",
        "benchmark", "iters", "best ns/it", "mean ns/it", "throughput", "speedup"
    );

    for result in results {
        let best = result.best_ns_per_iter();
        let mean = result.mean_ns_per_iter();
        let throughput = result.best_items_per_second();
        let speedup = baseline
            .and_then(|base| base.get(result.name))
            .map(|baseline_ns| format!("{:.2}x", baseline_ns / best))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:<24} {:>10} {:>14.1} {:>14.1} {:>10.2}M {:<5} {:>10}",
            result.name,
            result.iterations,
            best,
            mean,
            throughput / 1_000_000.0,
            result.unit,
            speedup
        );
        black_box(result.checksum);
    }
}

fn save_results(path: &PathBuf, results: &[BenchResult]) -> std::io::Result<()> {
    let mut out = String::from("name\titerations\tbest_ns_per_iter\tmean_ns_per_iter\n");
    for result in results {
        out.push_str(result.name);
        out.push('\t');
        out.push_str(&result.iterations.to_string());
        out.push('\t');
        out.push_str(&format!("{:.3}", result.best_ns_per_iter()));
        out.push('\t');
        out.push_str(&format!("{:.3}", result.mean_ns_per_iter()));
        out.push('\n');
    }
    fs::write(path, out)
}

fn load_baseline(path: &std::path::Path) -> std::io::Result<HashMap<String, f64>> {
    let text = fs::read_to_string(path)?;
    let mut baseline = HashMap::new();
    for line in text.lines().skip(1) {
        let mut parts = line.split('\t');
        let Some(name) = parts.next() else {
            continue;
        };
        let _iterations = parts.next();
        let Some(best_ns) = parts.next() else {
            continue;
        };
        if let Ok(value) = best_ns.parse::<f64>() {
            baseline.insert(name.to_string(), value);
        }
    }
    Ok(baseline)
}
