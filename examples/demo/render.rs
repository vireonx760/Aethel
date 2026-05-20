use crate::sim::{BodyKind, Camera3D, Simulation, SimulationSettings};
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use std::mem;
use wgpu::util::DeviceExt;

const INITIAL_BODY_CAPACITY: usize = 4_096;
const INITIAL_LINE_CAPACITY: usize = 4_096;

pub const SPACE_WGSL: &str = r#"
struct SceneUniform {
    view_proj: mat4x4<f32>,
    camera_right_time: vec4<f32>,
    camera_up_pad: vec4<f32>,
    camera_forward_pad: vec4<f32>,
    camera_pos_pad: vec4<f32>,
    sun_pos_radius: array<vec4<f32>, 3>,
    sun_color_power: array<vec4<f32>, 3>,
    viewport: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> scene: SceneUniform;

struct BodyInstance {
    @location(0) pos_radius: vec4<f32>,
    @location(1) color_kind: vec4<f32>,
    @location(2) spin_axis_rate: vec4<f32>,
    @location(3) material: vec4<f32>,
}

struct BodyOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
    @location(3) kind: f32,
    @location(4) world: vec3<f32>,
    @location(5) spin_axis: vec3<f32>,
    @location(6) radius: f32,
    @location(7) spin_rate: f32,
    @location(8) rotation: f32,
    @location(9) atmosphere: f32,
    @location(10) roughness: f32,
    @location(11) seed: f32,
}

fn project_world(world: vec3<f32>) -> vec4<f32> {
    return scene.view_proj * vec4<f32>(world, 1.0);
}

fn hash31(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.1031);
    let r = q + dot(q, q.yzx + 33.33);
    return fract((r.x + r.y) * r.z);
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n000 = hash31(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash31(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash31(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash31(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash31(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash31(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash31(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash31(i + vec3<f32>(1.0, 1.0, 1.0));
    let nx00 = mix(n000, n100, u.x);
    let nx10 = mix(n010, n110, u.x);
    let nx01 = mix(n001, n101, u.x);
    let nx11 = mix(n011, n111, u.x);
    let nxy0 = mix(nx00, nx10, u.y);
    let nxy1 = mix(nx01, nx11, u.y);
    return mix(nxy0, nxy1, u.z);
}

fn fbm(p0: vec3<f32>) -> f32 {
    var p = p0;
    var amp = 0.5;
    var sum = 0.0;
    for (var i = 0u; i < 5u; i = i + 1u) {
        sum = sum + noise3(p) * amp;
        p = p * 2.03 + vec3<f32>(17.1, 9.2, 5.7);
        amp = amp * 0.52;
    }
    return sum;
}

@vertex
fn body_vs(@builtin(vertex_index) vertex_index: u32, instance: BodyInstance) -> BodyOut {
    let right = vertex_index == 1u || vertex_index == 4u || vertex_index == 5u;
    let top = vertex_index == 2u || vertex_index == 3u || vertex_index == 5u;
    let local = vec2<f32>(select(-1.0, 1.0, right), select(-1.0, 1.0, top));
    let radius = max(instance.pos_radius.w, 0.5);
    let world = instance.pos_radius.xyz;
    let billboard_world = world
        + scene.camera_right_time.xyz * local.x * radius
        + scene.camera_up_pad.xyz * local.y * radius;

    var out: BodyOut;
    out.clip_pos = project_world(billboard_world);
    out.local = local;
    out.uv = local * 0.5 + vec2<f32>(0.5);
    out.color = instance.color_kind.rgb;
    out.kind = instance.color_kind.a;
    out.world = world;
    out.spin_axis = normalize(instance.spin_axis_rate.xyz + vec3<f32>(0.0001, 0.0, 0.0));
    out.spin_rate = instance.spin_axis_rate.w;
    out.rotation = instance.material.x;
    out.atmosphere = instance.material.y;
    out.roughness = instance.material.z;
    out.seed = instance.material.w;
    out.radius = radius;
    return out;
}

fn surface_normal(local: vec2<f32>) -> vec3<f32> {
    let d2 = dot(local, local);
    let z = sqrt(max(0.0, 1.0 - d2));
    return normalize(vec3<f32>(local.x, local.y, z));
}

fn surface_normal_world(local: vec2<f32>) -> vec3<f32> {
    let n = surface_normal(local);
    return normalize(
        scene.camera_right_time.xyz * n.x +
        scene.camera_up_pad.xyz * n.y -
        scene.camera_forward_pad.xyz * n.z
    );
}

fn star_color(in: BodyOut, d: f32) -> vec4<f32> {
    let time = scene.camera_right_time.w;
    let safe_d = clamp(d, 0.0, 1.0);
    let z = sqrt(max(0.0, 1.0 - safe_d * safe_d));
    let flow = vec2<f32>(time * (0.045 + in.seed * 0.025), -time * 0.031);
    let granulation = fbm(vec3<f32>(in.local * 8.5 + flow, in.seed * 17.0));
    let cells = fbm(vec3<f32>(in.local * 21.0 - flow.yx * 1.7, in.seed * 41.0));
    let magnetic = pow(abs(sin((in.local.x * 4.0 + in.local.y * 2.7 + time * 0.12) * 3.14159)), 8.0);
    let limb = 0.38 + 0.62 * pow(z, 0.42);
    let photosphere = 1.0 - smoothstep(0.86, 1.0, safe_d);
    let core = smoothstep(0.62, 0.04, safe_d);
    let corona = pow(max(0.0, 1.0 - safe_d), 0.34) * (1.0 - smoothstep(0.72, 1.0, safe_d));
    let heat = limb * (0.82 + granulation * 0.38 + cells * 0.10) + magnetic * 0.20;
    let color = in.color * heat * (0.78 + photosphere * 0.95)
        + vec3<f32>(1.0, 0.94, 0.72) * core * 0.92
        + in.color * corona * 0.42;
    let alpha = clamp(photosphere * (0.86 + core * 0.12) + corona * 0.18, 0.0, 1.0);
    return vec4<f32>(color, alpha);
}

fn rotate2(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn planet_color(in: BodyOut, d: f32) -> vec4<f32> {
    let local_n = surface_normal(in.local);
    let n = surface_normal_world(in.local);
    let world = in.world + n * in.radius;
    var light = vec3<f32>(0.035, 0.045, 0.065);
    for (var i = 0u; i < 3u; i = i + 1u) {
        let sun_dir = normalize(scene.sun_pos_radius[i].xyz - world);
        let diffuse = max(dot(n, sun_dir), 0.0);
        let dist = max(distance(scene.sun_pos_radius[i].xyz, world), 1.0);
        let power = scene.sun_color_power[i].w / (0.016 * dist + 80.0);
        light = light + scene.sun_color_power[i].rgb * diffuse * power;
    }

    let time = scene.camera_right_time.w;
    let rotated = rotate2(in.local, in.rotation + time * in.spin_rate * 0.08);
    let uv_rot = rotated * 0.5 + vec2<f32>(0.5);
    let bands = sin((uv_rot.y + fbm(vec3<f32>(uv_rot * 2.5, in.seed)) * 0.08) * 46.0 + in.seed * 12.0);
    let terrain = fbm(vec3<f32>(rotated * 4.8, in.seed * 23.0 + time * 0.015));
    let cloud = fbm(vec3<f32>(rotated * 9.0 + vec2<f32>(time * 0.018, 0.0), in.seed * 41.0));
    let land = mix(
        in.color * (0.44 + in.roughness * 0.20),
        in.color * 1.18 + vec3<f32>(0.10, 0.08, 0.04),
        smoothstep(0.32, 0.74, terrain)
    );
    let banded = land + vec3<f32>(bands * 0.045);
    let clouds = vec3<f32>(smoothstep(0.64, 0.93, cloud)) * 0.22 * in.atmosphere;
    let rim = pow(1.0 - clamp(local_n.z, 0.0, 1.0), 2.5);
    let view_dir = normalize(scene.camera_pos_pad.xyz - world);
    var scatter = vec3<f32>(0.0);
    for (var i = 0u; i < 3u; i = i + 1u) {
        let sun_dir = normalize(scene.sun_pos_radius[i].xyz - world);
        let mu = clamp(dot(view_dir, sun_dir), -1.0, 1.0);
        let rayleigh = 0.0597 * (1.0 + mu * mu);
        let mie = 0.018 * pow(max(0.0, 1.0 - mu), 2.0);
        let horizon = pow(max(0.0, 1.0 - abs(dot(n, view_dir))), 2.2);
        scatter = scatter + scene.sun_color_power[i].rgb * (rayleigh + mie) * horizon;
    }
    let atmosphere = (vec3<f32>(0.24, 0.52, 1.0) * rim * 0.34 + scatter) * in.atmosphere;
    let color = (banded + clouds) * light + atmosphere;
    let alpha = 1.0 - smoothstep(0.985, 1.0, d);
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.35)), alpha);
}

fn rock_color(in: BodyOut, d: f32) -> vec4<f32> {
    let rotated = rotate2(in.local, in.rotation + scene.camera_right_time.w * in.spin_rate * 0.12);
    let rough = fbm(vec3<f32>(rotated * 6.0, in.seed * 17.0));
    let n = surface_normal_world(in.local + (rough - 0.5) * 0.12);
    var light = 0.08;
    for (var i = 0u; i < 3u; i = i + 1u) {
        let dir = normalize(scene.sun_pos_radius[i].xyz - in.world);
        light = light + max(dot(n, dir), 0.0) * 0.28;
    }
    let chipped = smoothstep(0.76, 0.22, d + rough * 0.08);
    return vec4<f32>(in.color * light * (0.7 + rough * 0.6), chipped);
}

@fragment
fn body_fs(in: BodyOut) -> @location(0) vec4<f32> {
    let d = length(in.local);
    let feather = fwidth(d) * 1.5;
    if (d > 1.0 + feather) {
        discard;
    }

    if (in.kind < 0.5) {
        return star_color(in, d);
    }
    if (in.kind < 1.5) {
        return planet_color(in, d);
    }
    return rock_color(in, d);
}

struct LineIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct LineOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn line_vs(in: LineIn) -> LineOut {
    var out: LineOut;
    out.clip_pos = project_world(in.pos);
    out.color = in.color;
    return out;
}

@fragment
fn line_fs(in: LineOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    view_proj: [[f32; 4]; 4],
    camera_right_time: [f32; 4],
    camera_up_pad: [f32; 4],
    camera_forward_pad: [f32; 4],
    camera_pos_pad: [f32; 4],
    sun_pos_radius: [[f32; 4]; 3],
    sun_color_power: [[f32; 4]; 3],
    viewport: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BodyInstance {
    pos_radius: [f32; 4],
    color_kind: [f32; 4],
    spin_axis_rate: [f32; 4],
    material: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineVertex {
    pos: [f32; 3],
    color: [f32; 4],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SpaceRenderStats {
    pub instances: usize,
    pub line_vertices: usize,
    pub body_capacity: usize,
    pub line_capacity: usize,
}

pub struct SpaceRenderContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub sim: &'a Simulation,
    pub settings: &'a SimulationSettings,
    pub camera: Camera3D,
    pub viewport: [u32; 2],
    pub time: f32,
}

pub struct SpaceRenderer {
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    body_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    body_buffer: wgpu::Buffer,
    line_buffer: wgpu::Buffer,
    body_capacity: usize,
    line_capacity: usize,
    body_instances: Vec<BodyInstance>,
    line_vertices: Vec<LineVertex>,
    stats: SpaceRenderStats,
}

impl SpaceRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Interstellar Architect Space Shader"),
            source: wgpu::ShaderSource::Wgsl(SPACE_WGSL.into()),
        });

        let uniform = SceneUniform {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            camera_right_time: [1.0, 0.0, 0.0, 0.0],
            camera_up_pad: [0.0, 1.0, 0.0, 0.0],
            camera_forward_pad: [0.0, 0.0, -1.0, 0.0],
            camera_pos_pad: [0.0, 0.0, 0.0, 0.0],
            sun_pos_radius: [[0.0; 4]; 3],
            sun_color_power: [[0.0; 4]; 3],
            viewport: [1280.0, 720.0, 0.0, 0.0],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Interstellar Architect Scene Uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Interstellar Architect Uniform Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Interstellar Architect Uniform Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Interstellar Architect Pipeline Layout"),
            bind_group_layouts: &[Some(&uniform_layout)],
            immediate_size: 0,
        });

        let body_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Interstellar Architect Body Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("body_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[body_instance_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("body_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Interstellar Architect Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("line_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[line_vertex_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("line_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let body_capacity = INITIAL_BODY_CAPACITY;
        let line_capacity = INITIAL_LINE_CAPACITY;
        let body_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Interstellar Architect Body Instances"),
            size: buffer_size::<BodyInstance>(body_capacity),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let line_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Interstellar Architect Line Vertices"),
            size: buffer_size::<LineVertex>(line_capacity),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniform_buffer,
            uniform_bind_group,
            body_pipeline,
            line_pipeline,
            body_buffer,
            line_buffer,
            body_capacity,
            line_capacity,
            body_instances: Vec::with_capacity(body_capacity),
            line_vertices: Vec::with_capacity(line_capacity),
            stats: SpaceRenderStats {
                body_capacity,
                line_capacity,
                ..SpaceRenderStats::default()
            },
        }
    }

    pub fn render(&mut self, ctx: SpaceRenderContext<'_>) {
        self.populate_instances(ctx.sim, ctx.settings);
        self.populate_lines(ctx.sim, ctx.settings);
        self.ensure_body_capacity(ctx.device, self.body_instances.len());
        self.ensure_line_capacity(ctx.device, self.line_vertices.len());

        if !self.body_instances.is_empty() {
            ctx.queue.write_buffer(
                &self.body_buffer,
                0,
                bytemuck::cast_slice(&self.body_instances),
            );
        }
        if !self.line_vertices.is_empty() {
            ctx.queue.write_buffer(
                &self.line_buffer,
                0,
                bytemuck::cast_slice(&self.line_vertices),
            );
        }

        let uniform = build_uniform(ctx.sim, ctx.camera, ctx.viewport, ctx.time);
        ctx.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        {
            let mut pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Interstellar Architect Space Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ctx.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.007,
                            g: 0.009,
                            b: 0.015,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if !self.body_instances.is_empty() {
                pass.set_pipeline(&self.body_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.body_buffer.slice(..));
                pass.draw(0..6, 0..self.body_instances.len() as u32);
            }

            if !self.line_vertices.is_empty() {
                pass.set_pipeline(&self.line_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.line_buffer.slice(..));
                pass.draw(0..self.line_vertices.len() as u32, 0..1);
            }
        }

        self.stats = SpaceRenderStats {
            instances: self.body_instances.len(),
            line_vertices: self.line_vertices.len(),
            body_capacity: self.body_capacity,
            line_capacity: self.line_capacity,
        };
    }

    pub fn stats(&self) -> SpaceRenderStats {
        self.stats
    }

    fn populate_instances(&mut self, sim: &Simulation, settings: &SimulationSettings) {
        self.body_instances.clear();

        if settings.show_rings {
            self.body_instances
                .extend(sim.asteroids().iter().map(|asteroid| BodyInstance {
                    pos_radius: [
                        asteroid.pos.x,
                        asteroid.pos.y,
                        asteroid.pos.z - 8.0,
                        asteroid.radius,
                    ],
                    color_kind: [
                        asteroid.color.x,
                        asteroid.color.y,
                        asteroid.color.z,
                        BodyKind::Debris.shader_code(),
                    ],
                    spin_axis_rate: [
                        asteroid.spin_axis.x,
                        asteroid.spin_axis.y,
                        asteroid.spin_axis.z,
                        asteroid.spin_rate,
                    ],
                    material: [asteroid.rotation, 0.0, 0.82, asteroid.seed],
                }));
        }

        self.body_instances
            .extend(sim.bodies().iter().map(|body| BodyInstance {
                pos_radius: [body.pos.x, body.pos.y, body.pos.z, body.radius],
                color_kind: [
                    body.color.x,
                    body.color.y,
                    body.color.z,
                    body.kind.shader_code(),
                ],
                spin_axis_rate: [
                    body.spin_axis.x,
                    body.spin_axis.y,
                    body.spin_axis.z,
                    body.spin_rate,
                ],
                material: [body.rotation, body.atmosphere, body.roughness, body.seed],
            }));

        if let Some(launch) = sim.launch() {
            self.body_instances.push(BodyInstance {
                pos_radius: [
                    launch.start.x,
                    launch.start.y,
                    launch.start.z + 4.0,
                    (launch.mass.sqrt() * 2.0).clamp(8.0, 32.0),
                ],
                color_kind: [0.74, 0.96, 1.0, BodyKind::Projectile.shader_code()],
                spin_axis_rate: [0.33, 0.85, 0.41, 1.2],
                material: [0.0, 0.22, 0.74, 0.91],
            });
        }
    }

    fn populate_lines(&mut self, sim: &Simulation, settings: &SimulationSettings) {
        self.line_vertices.clear();

        if settings.show_prediction {
            let points = sim.prediction();
            for pair in points.windows(2) {
                let alpha = 0.16 + (pair[0].distance(pair[1]) * 0.004).min(0.28);
                self.push_line(pair[0], pair[1], Vec4::new(0.42, 0.86, 1.0, alpha));
            }
        }

        for (index, (body, trail)) in sim.bodies().iter().zip(sim.trails().iter()).enumerate() {
            if body.kind == BodyKind::Star {
                continue;
            }
            let selected = sim.selected_index() == Some(index);
            let color = if selected {
                Vec4::new(1.0, 0.88, 0.38, 0.68)
            } else {
                Vec4::new(body.color.x, body.color.y, body.color.z, 0.20)
            };
            trail.for_each_segment(|a, b, age| {
                self.push_line(a, b, Vec4::new(color.x, color.y, color.z, color.w * age));
            });
        }

        if let Some(launch) = sim.launch() {
            self.push_line(
                launch.current,
                launch.start,
                Vec4::new(1.0, 0.85, 0.35, 0.72),
            );
        }

        if let Some((_index, body)) = sim.selected_body() {
            self.push_spin_axis(body);
        }

        if settings.show_rings {
            for body in sim
                .bodies()
                .iter()
                .filter(|body| body.kind == BodyKind::Planet)
            {
                let ring_radius = body.radius * 5.0;
                let axis = (body.pos.normalize_or_zero() + Vec3::new(0.18, 0.72, 0.34))
                    .normalize_or_zero();
                let right = axis.cross(Vec3::Y).normalize_or_zero();
                let right = if right.length_squared() > 1e-5 {
                    right
                } else {
                    Vec3::X
                };
                let up = axis.cross(right).normalize_or_zero();
                let mut prev = body.pos + right * ring_radius;
                for i in 1..=96 {
                    let a = i as f32 / 96.0 * std::f32::consts::TAU;
                    let next =
                        body.pos + right * (a.cos() * ring_radius) + up * (a.sin() * ring_radius);
                    self.push_line(
                        prev,
                        next,
                        Vec4::new(body.color.x, body.color.y, body.color.z, 0.12),
                    );
                    prev = next;
                }
            }
        }
    }

    #[inline]
    fn push_spin_axis(&mut self, body: &crate::sim::Body) {
        if body.kind == BodyKind::Star {
            return;
        }
        let axis = body.spin_axis.normalize_or_zero();
        let start = body.pos - axis * body.radius * 1.25;
        let end = body.pos + axis * body.radius * 2.35;
        let color = Vec4::new(1.0, 0.72, 0.22, 0.88);
        self.push_line(start, end, color);

        let side = axis.cross(Vec3::Y).normalize_or_zero();
        let side = if side.length_squared() > 1e-5 {
            side
        } else {
            axis.cross(Vec3::X).normalize_or_zero()
        };
        let spin_sign = if body.spin_rate >= 0.0 { 1.0 } else { -1.0 };
        let head = body.radius * 0.42 * spin_sign;
        self.push_line(end, end - axis * body.radius * 0.36 + side * head, color);
        self.push_line(end, end - axis * body.radius * 0.36 - side * head, color);
    }

    #[inline]
    fn push_line(&mut self, a: Vec3, b: Vec3, color: Vec4) {
        self.line_vertices.push(LineVertex {
            pos: [a.x, a.y, a.z],
            color: [color.x, color.y, color.z, color.w],
        });
        self.line_vertices.push(LineVertex {
            pos: [b.x, b.y, b.z],
            color: [color.x, color.y, color.z, color.w],
        });
    }

    fn ensure_body_capacity(&mut self, device: &wgpu::Device, needed: usize) {
        if needed <= self.body_capacity {
            return;
        }
        self.body_capacity = needed.next_power_of_two();
        self.body_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Interstellar Architect Body Instances"),
            size: buffer_size::<BodyInstance>(self.body_capacity),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }

    fn ensure_line_capacity(&mut self, device: &wgpu::Device, needed: usize) {
        if needed <= self.line_capacity {
            return;
        }
        self.line_capacity = needed.next_power_of_two();
        self.line_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Interstellar Architect Line Vertices"),
            size: buffer_size::<LineVertex>(self.line_capacity),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }
}

fn build_uniform(
    sim: &Simulation,
    camera: Camera3D,
    viewport: [u32; 2],
    time: f32,
) -> SceneUniform {
    let mut sun_pos_radius = [[0.0; 4]; 3];
    let mut sun_color_power = [[0.0; 4]; 3];
    for (slot, body) in sim
        .bodies()
        .iter()
        .filter(|body| body.kind == BodyKind::Star)
        .take(3)
        .enumerate()
    {
        sun_pos_radius[slot] = [body.pos.x, body.pos.y, body.pos.z, body.radius];
        sun_color_power[slot] = [
            body.color.x,
            body.color.y,
            body.color.z,
            (body.mass / 4_800.0).clamp(0.6, 1.5),
        ];
    }

    SceneUniform {
        view_proj: camera.view_proj(viewport).to_cols_array_2d(),
        camera_right_time: [camera.right().x, camera.right().y, camera.right().z, time],
        camera_up_pad: [camera.up().x, camera.up().y, camera.up().z, 0.0],
        camera_forward_pad: [
            camera.forward().x,
            camera.forward().y,
            camera.forward().z,
            0.0,
        ],
        camera_pos_pad: [camera.eye().x, camera.eye().y, camera.eye().z, 0.0],
        sun_pos_radius,
        sun_color_power,
        viewport: [
            viewport[0].max(1) as f32,
            viewport[1].max(1) as f32,
            0.0,
            0.0,
        ],
    }
}

fn body_instance_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
    const ATTRS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<BodyInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRS,
    }
}

fn line_vertex_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<LineVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRS,
    }
}

#[inline]
fn buffer_size<T>(capacity: usize) -> wgpu::BufferAddress {
    (capacity.max(1) * mem::size_of::<T>()) as wgpu::BufferAddress
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_validates_as_wgsl() {
        let parsed = naga::front::wgsl::parse_str(SPACE_WGSL);
        assert!(parsed.is_ok(), "space shader must parse: {parsed:?}");
        let Ok(module) = parsed else {
            return;
        };
        let validated = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module);
        assert!(
            validated.is_ok(),
            "space shader must validate: {validated:?}"
        );
    }

    #[test]
    fn gpu_instance_layouts_are_compact() {
        assert_eq!(mem::size_of::<BodyInstance>(), 64);
        assert_eq!(mem::size_of::<LineVertex>(), 28);
        assert_eq!(mem::align_of::<BodyInstance>(), 4);
    }

    #[test]
    fn uniform_layout_is_multiple_of_sixteen() {
        assert_eq!(mem::size_of::<SceneUniform>() % 16, 0);
    }
}
