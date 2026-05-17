use aethel_gui::core::input::InputManager;
use glam::{Mat4, Vec2, Vec3};

const MAX_BODIES: usize = 96;
const MAX_ASTEROIDS: usize = 6_000;
const MAX_PREDICTION: usize = 1_024;
const TRAIL_CAPACITY: usize = 192;
const STAR_COUNT: usize = 3;
const WORLD_EDGE: f32 = 2_400.0;
const FIXED_DT: f32 = 1.0 / 90.0;
const SELECT_DRAG_PX: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Star,
    Planet,
    Projectile,
    Debris,
}

impl BodyKind {
    #[inline]
    pub fn shader_code(self) -> f32 {
        match self {
            Self::Star => 0.0,
            Self::Planet => 1.0,
            Self::Projectile => 2.0,
            Self::Debris => 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub name: &'static str,
    pub pos: Vec3,
    pub vel: Vec3,
    pub radius: f32,
    pub mass: f32,
    pub color: Vec3,
    pub kind: BodyKind,
    pub seed: f32,
    pub spin_axis: Vec3,
    pub spin_rate: f32,
    pub rotation: f32,
    pub atmosphere: f32,
    pub roughness: f32,
}

struct PlanetInit {
    name: &'static str,
    pos: Vec3,
    vel: Vec3,
    radius: f32,
    mass: f32,
    color: Vec3,
    seed: f32,
    spin_axis: Vec3,
    spin_rate: f32,
}

impl Body {
    #[inline]
    fn star(name: &'static str, pos: Vec3, radius: f32, mass: f32, color: Vec3, seed: f32) -> Self {
        Self {
            name,
            pos,
            vel: Vec3::ZERO,
            radius,
            mass,
            color,
            kind: BodyKind::Star,
            seed,
            spin_axis: Vec3::new(0.0, 1.0, 0.0),
            spin_rate: 0.04,
            rotation: seed * std::f32::consts::TAU,
            atmosphere: 0.0,
            roughness: 0.0,
        }
    }

    #[inline]
    fn planet(init: PlanetInit) -> Self {
        Self {
            name: init.name,
            pos: init.pos,
            vel: init.vel,
            radius: init.radius,
            mass: init.mass,
            color: init.color,
            kind: BodyKind::Planet,
            seed: init.seed,
            spin_axis: init.spin_axis.normalize_or_zero(),
            spin_rate: init.spin_rate,
            rotation: init.seed * std::f32::consts::TAU,
            atmosphere: 0.68,
            roughness: 0.46,
        }
    }

    #[inline]
    fn projectile(
        name: &'static str,
        pos: Vec3,
        vel: Vec3,
        radius: f32,
        mass: f32,
        color: Vec3,
        seed: f32,
    ) -> Self {
        Self {
            name,
            pos,
            vel,
            radius,
            mass,
            color,
            kind: BodyKind::Projectile,
            seed,
            spin_axis: Vec3::new(0.33, 0.85, 0.41).normalize(),
            spin_rate: 0.45 + seed * 1.5,
            rotation: seed * std::f32::consts::TAU,
            atmosphere: 0.24,
            roughness: 0.70,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Asteroid {
    pub pos: Vec3,
    pub vel: Vec3,
    pub radius: f32,
    pub color: Vec3,
    pub seed: f32,
    pub spin_axis: Vec3,
    pub spin_rate: f32,
    pub rotation: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LaunchPreview {
    pub start: Vec3,
    pub current: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct OrbitTrail {
    points: [Vec3; TRAIL_CAPACITY],
    len: usize,
    cursor: usize,
    stride: u8,
}

impl OrbitTrail {
    fn new(start: Vec3) -> Self {
        let mut points = [Vec3::ZERO; TRAIL_CAPACITY];
        points[0] = start;
        Self {
            points,
            len: 1,
            cursor: 1,
            stride: 0,
        }
    }

    fn push(&mut self, point: Vec3) {
        self.stride = self.stride.wrapping_add(1);
        if self.stride & 0b11 != 0 {
            return;
        }
        self.points[self.cursor] = point;
        self.cursor = (self.cursor + 1) % TRAIL_CAPACITY;
        self.len = (self.len + 1).min(TRAIL_CAPACITY);
    }

    pub fn for_each_segment(&self, mut f: impl FnMut(Vec3, Vec3, f32)) {
        if self.len < 2 {
            return;
        }
        let start = if self.len == TRAIL_CAPACITY {
            self.cursor
        } else {
            0
        };
        let mut prev = self.points[start];
        for i in 1..self.len {
            let index = (start + i) % TRAIL_CAPACITY;
            let next = self.points[index];
            let alpha = i as f32 / self.len as f32;
            f(prev, next, alpha);
            prev = next;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SimulationSettings {
    pub gravity: f32,
    pub time_scale: f32,
    pub launch_mass: f32,
    pub prediction_steps: u32,
    pub softening: f32,
    pub asteroid_count: usize,
    pub show_prediction: bool,
    pub show_rings: bool,
    pub paused: bool,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            gravity: 72.0,
            time_scale: 1.0,
            launch_mass: 42.0,
            prediction_steps: 260,
            softening: 38.0,
            asteroid_count: 2_800,
            show_prediction: true,
            show_rings: true,
            paused: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SimulationStats {
    pub bodies: usize,
    pub asteroids: usize,
    pub prediction_points: usize,
    pub stable_score: f32,
    pub elapsed: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera3D {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, -18.0, 0.0),
            yaw: -0.34,
            pitch: -0.82,
            distance: 1_260.0,
            fov_y: 48.0_f32.to_radians(),
        }
    }
}

impl Camera3D {
    #[inline]
    pub fn eye(self) -> Vec3 {
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        let dir = Vec3::new(sy * cp, sp, cy * cp).normalize_or_zero();
        self.target - dir * self.distance.max(80.0)
    }

    #[inline]
    pub fn forward(self) -> Vec3 {
        (self.target - self.eye()).normalize_or_zero()
    }

    #[inline]
    pub fn right(self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or_zero()
    }

    #[inline]
    pub fn up(self) -> Vec3 {
        self.right().cross(self.forward()).normalize_or_zero()
    }

    #[inline]
    pub fn view_proj(self, viewport: [u32; 2]) -> Mat4 {
        let aspect = viewport[0].max(1) as f32 / viewport[1].max(1) as f32;
        let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, aspect, 8.0, 8_000.0);
        proj * view
    }

    #[inline]
    pub fn screen_to_world(self, mouse: Vec2, viewport: [u32; 2]) -> Vec3 {
        let w = viewport[0].max(1) as f32;
        let h = viewport[1].max(1) as f32;
        let aspect = w / h;
        let tan_half = (self.fov_y * 0.5).tan();
        let ndc = Vec2::new((mouse.x / w) * 2.0 - 1.0, 1.0 - (mouse.y / h) * 2.0);
        let dir = (self.forward()
            + self.right() * ndc.x * aspect * tan_half
            + self.up() * ndc.y * tan_half)
            .normalize_or_zero();
        let eye = self.eye();
        if dir.z.abs() < 1e-4 {
            return self.target;
        }
        let t = -eye.z / dir.z;
        eye + dir * t.max(0.0)
    }

    #[inline]
    pub fn world_to_screen(self, world: Vec3, viewport: [u32; 2]) -> Option<(Vec2, f32)> {
        let clip = self.view_proj(viewport) * world.extend(1.0);
        if clip.w <= 1e-5 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        if ndc.z < -1.0 || ndc.z > 1.0 {
            return None;
        }
        let w = viewport[0].max(1) as f32;
        let h = viewport[1].max(1) as f32;
        Some((
            Vec2::new((ndc.x * 0.5 + 0.5) * w, (0.5 - ndc.y * 0.5) * h),
            ndc.z,
        ))
    }
}

pub struct Simulation {
    bodies: Vec<Body>,
    trails: Vec<OrbitTrail>,
    asteroids: Vec<Asteroid>,
    prediction: Vec<Vec3>,
    launch: Option<LaunchPreview>,
    selected: Option<usize>,
    drag_start_mouse: Vec2,
    rng: u64,
    accumulator: f32,
    elapsed: f32,
    stable_score: f32,
}

impl Simulation {
    pub fn new() -> Self {
        let mut sim = Self {
            bodies: Vec::with_capacity(MAX_BODIES),
            trails: Vec::with_capacity(MAX_BODIES),
            asteroids: Vec::with_capacity(MAX_ASTEROIDS),
            prediction: Vec::with_capacity(MAX_PREDICTION),
            launch: None,
            selected: None,
            drag_start_mouse: Vec2::ZERO,
            rng: 0x9e37_79b9_7f4a_7c15,
            accumulator: 0.0,
            elapsed: 0.0,
            stable_score: 0.0,
        };
        sim.reset();
        sim
    }

    pub fn reset(&mut self) {
        self.bodies.clear();
        self.trails.clear();
        self.asteroids.clear();
        self.prediction.clear();
        self.launch = None;
        self.selected = None;
        self.accumulator = 0.0;
        self.elapsed = 0.0;
        self.stable_score = 0.0;
        self.rng = 0x7151_aa91_31c8_f911;

        self.push_body(Body::star(
            "Aster Helios",
            Vec3::new(-360.0, 110.0, -170.0),
            82.0,
            5_400.0,
            Vec3::new(1.0, 0.52, 0.22),
            0.13,
        ));
        self.push_body(Body::star(
            "Blue Vela",
            Vec3::new(350.0, 120.0, 125.0),
            70.0,
            4_500.0,
            Vec3::new(0.55, 0.78, 1.0),
            0.43,
        ));
        self.push_body(Body::star(
            "Gold Solenne",
            Vec3::new(10.0, -330.0, 60.0),
            76.0,
            4_900.0,
            Vec3::new(1.0, 0.86, 0.44),
            0.78,
        ));

        self.add_orbital_planet("Eos Prime", Vec3::new(-42.0, 356.0, 84.0), 29.0, 220.0, 0.0);
        self.selected = Some(STAR_COUNT);

        self.rebuild_asteroids(1_600);
    }

    pub fn stabilize(&mut self) {
        let stars = self.star_snapshot();
        for body in &mut self.bodies {
            if body.kind != BodyKind::Planet && body.kind != BodyKind::Projectile {
                continue;
            }
            let Some((nearest, dist)) = nearest_star(body.pos, &stars) else {
                continue;
            };
            let tangent = orbital_tangent((body.pos - nearest.pos).normalize_or_zero());
            let speed = (nearest.mass / dist.max(80.0)).sqrt() * 5.9;
            body.vel = tangent * speed;
        }
    }

    pub fn rebuild_asteroids(&mut self, count: usize) {
        self.asteroids.clear();
        let count = count.min(MAX_ASTEROIDS);
        if count == 0 || self.bodies.len() <= STAR_COUNT {
            return;
        }

        let planets: Vec<(Vec3, Vec3, f32, Vec3)> = self
            .bodies
            .iter()
            .filter(|body| body.kind == BodyKind::Planet)
            .map(|body| (body.pos, body.vel, body.radius, body.color))
            .collect();
        if planets.is_empty() {
            return;
        }

        for i in 0..count {
            let planet_index = i % planets.len();
            let (planet_pos, planet_vel, planet_radius, planet_color) = planets[planet_index];
            let a = self.rand01() * std::f32::consts::TAU;
            let orbit = planet_radius * (3.2 + self.rand01() * 8.5);
            let eccentric = 1.0 + (self.rand01() - 0.5) * 0.23;
            let ring_axis =
                (planet_pos.normalize_or_zero() + Vec3::new(0.18, 0.72, 0.34)).normalize_or_zero();
            let ring_right = ring_axis.cross(Vec3::Y).normalize_or_zero();
            let ring_right = if ring_right.length_squared() > 1e-5 {
                ring_right
            } else {
                Vec3::X
            };
            let ring_up = ring_axis.cross(ring_right).normalize_or_zero();
            let pos = planet_pos
                + ring_right * (a.cos() * orbit * eccentric)
                + ring_up * (a.sin() * orbit)
                + ring_axis * self.rand_signed() * 18.0;
            let tangent = (-ring_right * a.sin() + ring_up * a.cos()).normalize_or_zero();
            let local_speed = (16.0 + self.rand01() * 42.0) / orbit.sqrt().max(2.0);
            let color = planet_color * (0.34 + self.rand01() * 0.22)
                + Vec3::splat(0.15 + self.rand01() * 0.18);
            let radius = 0.75 + self.rand01() * 2.4;
            let seed = self.rand01();
            let spin_axis = Vec3::new(self.rand_signed(), self.rand_signed(), self.rand_signed())
                .normalize_or_zero();
            let spin_rate = 0.7 + self.rand01() * 4.0;
            self.asteroids.push(Asteroid {
                pos,
                vel: planet_vel + tangent * local_speed,
                radius,
                color,
                seed,
                spin_axis,
                spin_rate,
                rotation: seed * std::f32::consts::TAU,
            });
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        settings: &SimulationSettings,
        camera: Camera3D,
        input: &InputManager,
        viewport: [u32; 2],
        gui_guard_width: f32,
    ) -> bool {
        let pointer_changed =
            self.handle_pointer(input, settings, camera, viewport, gui_guard_width);
        let active = pointer_changed || !settings.paused;

        if !settings.paused {
            self.accumulator += (dt * settings.time_scale).min(0.25);
            while self.accumulator >= FIXED_DT {
                self.step(FIXED_DT, settings);
                self.accumulator -= FIXED_DT;
            }
            self.elapsed += dt * settings.time_scale;
        }

        self.update_prediction(settings);
        self.update_stability();
        active
    }

    pub fn bodies(&self) -> &[Body] {
        &self.bodies
    }

    pub fn asteroids(&self) -> &[Asteroid] {
        &self.asteroids
    }

    pub fn prediction(&self) -> &[Vec3] {
        &self.prediction
    }

    pub fn trails(&self) -> &[OrbitTrail] {
        &self.trails
    }

    pub fn launch(&self) -> Option<LaunchPreview> {
        self.launch
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_body(&self) -> Option<(usize, &Body)> {
        self.selected
            .and_then(|index| self.bodies.get(index).map(|body| (index, body)))
    }

    pub fn selected_body_mut(&mut self) -> Option<(usize, &mut Body)> {
        self.selected
            .and_then(|index| self.bodies.get_mut(index).map(|body| (index, body)))
    }

    pub fn apply_selected_body_controls(
        &mut self,
        radius: f32,
        mass: f32,
        spin_rate: f32,
        atmosphere: f32,
        roughness: f32,
    ) {
        if let Some((_index, body)) = self.selected_body_mut() {
            if body.kind == BodyKind::Star {
                return;
            }
            body.radius = radius.clamp(6.0, 96.0);
            body.mass = mass.clamp(8.0, 900.0);
            body.spin_rate = spin_rate.clamp(-4.0, 4.0);
            body.atmosphere = atmosphere.clamp(0.0, 1.0);
            body.roughness = roughness.clamp(0.0, 1.0);
        }
    }

    pub fn stats(&self) -> SimulationStats {
        SimulationStats {
            bodies: self.bodies.len(),
            asteroids: self.asteroids.len(),
            prediction_points: self.prediction.len(),
            stable_score: self.stable_score,
            elapsed: self.elapsed,
        }
    }

    fn push_body(&mut self, body: Body) {
        self.trails.push(OrbitTrail::new(body.pos));
        self.bodies.push(body);
    }

    fn add_orbital_planet(
        &mut self,
        name: &'static str,
        pos: Vec3,
        radius: f32,
        mass: f32,
        hue: f32,
    ) {
        let stars = self.star_snapshot();
        let (nearest, dist) = nearest_star(pos, &stars).expect("seed stars");
        let tangent = orbital_tangent((pos - nearest.pos).normalize_or_zero());
        let speed = (nearest.mass / dist.max(90.0)).sqrt() * 5.6;
        let color = palette(hue);
        let seed = self.rand01();
        let spin_axis = Vec3::new(self.rand_signed() * 0.7, 1.0, self.rand_signed() * 0.7);
        let spin_rate = 0.22 + self.rand01() * 1.4;
        self.push_body(Body::planet(PlanetInit {
            name,
            pos,
            vel: tangent * speed,
            radius,
            mass,
            color,
            seed,
            spin_axis,
            spin_rate,
        }));
    }

    fn handle_pointer(
        &mut self,
        input: &InputManager,
        settings: &SimulationSettings,
        camera: Camera3D,
        viewport: [u32; 2],
        gui_guard_width: f32,
    ) -> bool {
        if input.mouse_pos.x <= gui_guard_width {
            if self.launch.is_some() && input.lmb.just_released {
                self.launch = None;
                self.prediction.clear();
                return true;
            }
            return false;
        }

        if input.lmb.just_pressed {
            self.drag_start_mouse = input.mouse_pos;
            if let Some(index) = self.pick_body(input.mouse_pos, camera, viewport) {
                self.selected = Some(index);
                self.launch = None;
                self.prediction.clear();
                return true;
            }

            let start = camera.screen_to_world(input.mouse_pos, viewport);
            self.launch = Some(LaunchPreview {
                start,
                current: start,
                velocity: Vec3::ZERO,
                mass: settings.launch_mass,
            });
            return true;
        }

        if let Some(mut launch) = self.launch {
            if input.lmb.held {
                launch.current = camera.screen_to_world(input.mouse_pos, viewport);
                launch.mass = settings.launch_mass;
                launch.velocity = (launch.start - launch.current) * 0.75;
                self.launch = Some(launch);
                return true;
            }

            if input.lmb.just_released {
                let drag_px = input.mouse_pos.distance(self.drag_start_mouse);
                if drag_px < SELECT_DRAG_PX {
                    self.launch = None;
                    self.prediction.clear();
                    return true;
                }
                let speed = launch.velocity.length();
                if speed > 8.0 && self.bodies.len() < MAX_BODIES {
                    let radius = (settings.launch_mass.sqrt() * 2.0).clamp(8.0, 32.0);
                    let color = palette(self.rand01());
                    let seed = self.rand01();
                    self.push_body(Body::projectile(
                        "Launched Body",
                        launch.start,
                        launch.velocity,
                        radius,
                        settings.launch_mass.max(8.0),
                        color,
                        seed,
                    ));
                    self.selected = Some(self.bodies.len() - 1);
                }
                self.launch = None;
                return true;
            }
        }

        false
    }

    fn pick_body(&self, mouse: Vec2, camera: Camera3D, viewport: [u32; 2]) -> Option<usize> {
        let mut best = None;
        let mut best_depth = f32::MAX;
        for (index, body) in self.bodies.iter().enumerate() {
            let Some((screen, depth)) = camera.world_to_screen(body.pos, viewport) else {
                continue;
            };
            let Some((edge, _)) =
                camera.world_to_screen(body.pos + camera.right() * body.radius, viewport)
            else {
                continue;
            };
            let radius_px = screen.distance(edge).max(10.0);
            if mouse.distance(screen) <= radius_px + 8.0 && depth < best_depth {
                best = Some(index);
                best_depth = depth;
            }
        }
        best
    }

    fn step(&mut self, dt: f32, settings: &SimulationSettings) {
        let snapshot = self.bodies.clone();
        for body in &mut self.bodies {
            body.rotation = (body.rotation + body.spin_rate * dt).rem_euclid(std::f32::consts::TAU);
            if body.kind == BodyKind::Star {
                continue;
            }
            let acc = acceleration_at(body.pos, &snapshot, settings.gravity, settings.softening);
            body.vel += acc * dt;
            body.vel *= 0.9995;
            body.pos += body.vel * dt;
        }

        self.retain_live_bodies();
        for (body, trail) in self.bodies.iter().zip(self.trails.iter_mut()) {
            trail.push(body.pos);
        }

        let body_snapshot = self.bodies.clone();
        for asteroid in &mut self.asteroids {
            asteroid.rotation =
                (asteroid.rotation + asteroid.spin_rate * dt).rem_euclid(std::f32::consts::TAU);
            let acc = acceleration_at(
                asteroid.pos,
                &body_snapshot,
                settings.gravity * 0.82,
                settings.softening,
            );
            asteroid.vel += acc * dt;
            asteroid.vel *= 0.999;
            asteroid.pos += asteroid.vel * dt;
            if asteroid.pos.x.abs() > WORLD_EDGE || asteroid.pos.y.abs() > WORLD_EDGE {
                wrap_asteroid(asteroid);
            }
        }
    }

    fn retain_live_bodies(&mut self) {
        let mut write = 0usize;
        for read in 0..self.bodies.len() {
            let body = self.bodies[read];
            let keep = body.kind == BodyKind::Star
                || (body.pos.x.abs() < WORLD_EDGE
                    && body.pos.y.abs() < WORLD_EDGE
                    && body.pos.z.abs() < WORLD_EDGE
                    && body.vel.length_squared() < 220_000.0);
            if keep {
                if write != read {
                    self.bodies[write] = self.bodies[read];
                    self.trails[write] = self.trails[read];
                }
                if self.selected == Some(read) {
                    self.selected = Some(write);
                }
                write += 1;
            } else if self.selected == Some(read) {
                self.selected = None;
            }
        }
        self.bodies.truncate(write);
        self.trails.truncate(write);
    }

    fn update_prediction(&mut self, settings: &SimulationSettings) {
        self.prediction.clear();
        if !settings.show_prediction {
            return;
        }
        let Some(launch) = self.launch else {
            return;
        };

        let mut pos = launch.start;
        let mut vel = launch.velocity;
        let steps = settings.prediction_steps.min(MAX_PREDICTION as u32);
        let bodies = self.star_and_planet_snapshot();
        for _ in 0..steps {
            let acc = acceleration_at(pos, &bodies, settings.gravity, settings.softening);
            vel += acc * FIXED_DT;
            pos += vel * FIXED_DT;
            self.prediction.push(pos);
            if pos.x.abs() > WORLD_EDGE || pos.y.abs() > WORLD_EDGE {
                break;
            }
        }
    }

    fn update_stability(&mut self) {
        let mut score = 0.0;
        let mut count = 0.0;
        for body in &self.bodies {
            if body.kind != BodyKind::Planet && body.kind != BodyKind::Projectile {
                continue;
            }
            let Some((nearest, dist)) = nearest_star(body.pos, &self.bodies[..STAR_COUNT]) else {
                continue;
            };
            let radial = (body.pos - nearest.pos).normalize_or_zero();
            let tangent_speed = body.vel.dot(orbital_tangent(radial)).abs();
            let target = (nearest.mass / dist.max(80.0)).sqrt() * 5.7;
            let speed_score = 1.0 - ((tangent_speed - target).abs() / target.max(1.0)).min(1.0);
            let distance_score = (dist / 680.0).clamp(0.0, 1.0);
            score += speed_score * distance_score;
            count += 1.0;
        }
        self.stable_score = if count > 0.0 { score / count } else { 0.0 };
    }

    fn star_snapshot(&self) -> Vec<Body> {
        self.bodies.iter().take(STAR_COUNT).copied().collect()
    }

    fn star_and_planet_snapshot(&self) -> Vec<Body> {
        self.bodies
            .iter()
            .copied()
            .filter(|body| body.kind == BodyKind::Star || body.kind == BodyKind::Planet)
            .collect()
    }

    #[inline]
    fn rand01(&mut self) -> f32 {
        self.rng ^= self.rng << 7;
        self.rng ^= self.rng >> 9;
        self.rng ^= self.rng << 8;
        let value = (self.rng & 0x00ff_ffff) as f32 / 0x0100_0000 as f32;
        value.clamp(0.0, 0.999_999)
    }

    #[inline]
    fn rand_signed(&mut self) -> f32 {
        self.rand01() * 2.0 - 1.0
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn acceleration_at(pos: Vec3, bodies: &[Body], gravity: f32, softening: f32) -> Vec3 {
    let mut acc = Vec3::ZERO;
    let soft2 = softening * softening;
    for body in bodies {
        let delta = body.pos - pos;
        let dist2 = delta.length_squared() + soft2;
        if dist2 <= 0.001 {
            continue;
        }
        let inv_dist = dist2.sqrt().recip();
        let strength = gravity * body.mass * inv_dist * inv_dist * inv_dist;
        acc += delta * strength;
    }
    acc
}

#[inline]
fn nearest_star(pos: Vec3, stars: &[Body]) -> Option<(Body, f32)> {
    let mut best = None;
    let mut best_dist = f32::MAX;
    for star in stars.iter().filter(|body| body.kind == BodyKind::Star) {
        let dist = pos.distance(star.pos);
        if dist < best_dist {
            best = Some(*star);
            best_dist = dist;
        }
    }
    best.map(|star| (star, best_dist))
}

#[inline]
fn orbital_tangent(radial: Vec3) -> Vec3 {
    let primary_axis = Vec3::new(0.22, 0.93, 0.31).normalize();
    let tangent = primary_axis.cross(radial).normalize_or_zero();
    if tangent.length_squared() > 1e-5 {
        tangent
    } else {
        Vec3::Y.cross(radial).normalize_or_zero()
    }
}

fn wrap_asteroid(asteroid: &mut Asteroid) {
    if asteroid.pos.x > WORLD_EDGE {
        asteroid.pos.x = -WORLD_EDGE;
    } else if asteroid.pos.x < -WORLD_EDGE {
        asteroid.pos.x = WORLD_EDGE;
    }
    if asteroid.pos.y > WORLD_EDGE {
        asteroid.pos.y = -WORLD_EDGE;
    } else if asteroid.pos.y < -WORLD_EDGE {
        asteroid.pos.y = WORLD_EDGE;
    }
    asteroid.vel *= 0.72;
}

#[inline]
pub fn palette(t: f32) -> Vec3 {
    let phase = t.fract() * std::f32::consts::TAU;
    Vec3::new(
        0.52 + phase.cos() * 0.34,
        0.54 + (phase + 2.1).cos() * 0.32,
        0.56 + (phase + 4.2).cos() * 0.36,
    )
    .clamp(Vec3::splat(0.08), Vec3::splat(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_system_has_stars_planets_and_asteroids() {
        let sim = Simulation::new();
        assert_eq!(
            sim.bodies[..STAR_COUNT]
                .iter()
                .filter(|b| b.kind == BodyKind::Star)
                .count(),
            STAR_COUNT
        );
        assert_eq!(sim.bodies.len(), STAR_COUNT + 1);
        assert!(sim.asteroids.len() >= 1_000);
    }

    #[test]
    fn camera_mapping_keeps_center_stable() {
        let camera = Camera3D::default();
        let world = camera.screen_to_world(Vec2::new(640.0, 360.0), [1280, 720]);
        assert!((world.x - camera.target.x).abs() < 0.01);
        assert!((world.y - camera.target.y).abs() < 0.01);
        assert!(world.z.abs() < 0.01);
    }

    #[test]
    fn acceleration_is_finite_near_mass() {
        let bodies = [Body::star(
            "Test Star",
            Vec3::ZERO,
            20.0,
            400.0,
            Vec3::ONE,
            0.0,
        )];
        let acc = acceleration_at(Vec3::new(0.001, 0.0, 0.0), &bodies, 80.0, 32.0);
        assert!(acc.is_finite());
    }
}
