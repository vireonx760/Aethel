struct Uniforms {
    screen_size: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: vec2<f32>,
    @location(3) radius: f32,
    @location(4) mode: f32,
    @location(5) world_pos: vec2<f32>,
    @location(6) clip_min: vec2<f32>,
    @location(7) clip_max: vec2<f32>,
    @location(8) use_clip: f32,
}

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) radius: f32,
    @location(4) mode: f32,
    @location(5) clip_min: vec2<f32>,
    @location(6) clip_max: vec2<f32>,
    @location(7) use_clip: f32,
    @location(8) rotation: f32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let local_uv = vec2<f32>(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
    );

    var world_pos = instance.pos + local_uv * instance.size;
    if (abs(instance.rotation) > 0.000001) {
        let local_centered = (local_uv - vec2<f32>(0.5)) * instance.size;
        let c = cos(instance.rotation);
        let s = sin(instance.rotation);
        let rotated = vec2<f32>(
            local_centered.x * c - local_centered.y * s,
            local_centered.x * s + local_centered.y * c,
        );
        world_pos = instance.pos + instance.size * 0.5 + rotated;
    }

    let ndc_x = (world_pos.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (world_pos.y / uniforms.screen_size.y) * 2.0;

    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = local_uv;
    out.color = instance.color;
    out.size = instance.size;
    out.radius = instance.radius;
    out.mode = instance.mode;
    out.world_pos = world_pos;
    out.clip_min = instance.clip_min;
    out.clip_max = instance.clip_max;
    out.use_clip = instance.use_clip;

    return out;
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let radius = clamp(r, 0.0, min(b.x, b.y));
    let q = abs(p) - b + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var clip_alpha = 1.0;
    if (in.use_clip > 0.5) {
        let cmin = min(in.clip_min, in.clip_max);
        let cmax = max(in.clip_min, in.clip_max);
        if (in.world_pos.x < cmin.x || in.world_pos.y < cmin.y ||
            in.world_pos.x > cmax.x || in.world_pos.y > cmax.y) {
            discard;
        }
        let edge_dist = min(
            min(in.world_pos.x - cmin.x, cmax.x - in.world_pos.x),
            min(in.world_pos.y - cmin.y, cmax.y - in.world_pos.y),
        );
        clip_alpha = clamp(edge_dist + 0.5, 0.0, 1.0);
    }

    let safe_size = max(in.size, vec2<f32>(0.0001));
    let safe_radius = clamp(in.radius, 0.0, min(safe_size.x, safe_size.y) * 0.5);
    let center_rel = (in.uv - 0.5) * safe_size;
    let dist = sd_rounded_box(center_rel, safe_size * 0.5, safe_radius);

    if (dist > 0.0) {
        discard;
    }

    var final_color = in.color;

    if (in.mode > 0.5 && in.mode < 1.5) {
        let hue = in.color.r;
        let rgb = hsv2rgb(vec3<f32>(hue, in.uv.x, 1.0 - in.uv.y));
        final_color = vec4<f32>(rgb, 1.0);
    } else if (in.mode > 1.5 && in.mode < 2.5) {
        let rgb = hsv2rgb(vec3<f32>(in.uv.x, 1.0, 1.0));
        final_color = vec4<f32>(rgb, 1.0);
    }

    let edge_softness = max(fwidth(dist), 0.75);
    let alpha = 1.0 - smoothstep(-edge_softness, edge_softness, dist);

    return vec4<f32>(final_color.rgb, final_color.a * alpha * clip_alpha);
}
