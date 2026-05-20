use crate::render::SpaceRenderStats;
use crate::sim::{Camera3D, Simulation, SimulationStats};
use aethel_gui::core::input::InputManager;
use aethel_gui::core::renderer::WidgetInstance;
use aethel_gui::gui::binding::{BoolSignal, F32Signal, U32Signal};
use aethel_gui::gui::geometry::{BoxConstraints, Point, Rect, Size};
use aethel_gui::gui::paint::PaintCtx;
use aethel_gui::gui::text::{set_buffer_size, set_buffer_text, shape_text, text_area};
use aethel_gui::gui::widget::{GuiManager, Widget};
use aethel_gui::widgets::{Button, Checkbox, Label, Panel, ProgressBar, SliderLabeled};
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, TextArea, TextBounds};
use std::any::Any;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

const PANEL_X: f32 = 20.0;
const PANEL_Y: f32 = 20.0;
const PANEL_W: f32 = 380.0;
const PANEL_H: f32 = 820.0;
const CONTENT_X: f32 = PANEL_X + 24.0;
const CONTENT_W: f32 = PANEL_W - 48.0;
const INSPECTOR_X: f32 = 1040.0;
const INSPECTOR_Y: f32 = 82.0;
const INSPECTOR_W: f32 = 360.0;
const INSPECTOR_H: f32 = 430.0;

#[derive(Clone)]
pub struct EditorSignals {
    pub gravity: F32Signal,
    pub time_scale: F32Signal,
    pub launch_mass: F32Signal,
    pub prediction_steps: U32Signal,
    pub asteroid_count: U32Signal,
    pub paused: BoolSignal,
    pub show_prediction: BoolSignal,
    pub show_rings: BoolSignal,
    pub stability: F32Signal,
    pub selected_radius: F32Signal,
    pub selected_mass: F32Signal,
    pub selected_spin: F32Signal,
    pub selected_atmosphere: F32Signal,
    pub selected_roughness: F32Signal,
}

impl EditorSignals {
    pub fn new() -> Self {
        Self {
            gravity: F32Signal::new(72.0),
            time_scale: F32Signal::new(1.0),
            launch_mass: F32Signal::new(42.0),
            prediction_steps: U32Signal::new(260),
            asteroid_count: U32Signal::new(1_600),
            paused: BoolSignal::new(false),
            show_prediction: BoolSignal::new(true),
            show_rings: BoolSignal::new(true),
            stability: F32Signal::new(0.0),
            selected_radius: F32Signal::new(24.0),
            selected_mass: F32Signal::new(200.0),
            selected_spin: F32Signal::new(0.8),
            selected_atmosphere: F32Signal::new(0.65),
            selected_roughness: F32Signal::new(0.45),
        }
    }
}

impl Default for EditorSignals {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default)]
pub struct EditorActions {
    reset: Arc<AtomicBool>,
    stabilize: Arc<AtomicBool>,
    rebuild_rings: Arc<AtomicBool>,
    camera_mode: Arc<AtomicU32>,
}

impl EditorActions {
    pub fn request_reset(&self) {
        self.reset.store(true, Ordering::Release);
    }

    pub fn request_stabilize(&self) {
        self.stabilize.store(true, Ordering::Release);
    }

    pub fn request_rebuild_rings(&self) {
        self.rebuild_rings.store(true, Ordering::Release);
    }

    pub fn set_camera_mode(&self, mode: u32) {
        self.camera_mode.store(mode.min(2), Ordering::Release);
    }

    pub fn camera_mode(&self) -> u32 {
        self.camera_mode.load(Ordering::Acquire)
    }

    pub fn take_reset(&self) -> bool {
        self.reset.swap(false, Ordering::AcqRel)
    }

    pub fn take_stabilize(&self) -> bool {
        self.stabilize.swap(false, Ordering::AcqRel)
    }

    pub fn take_rebuild_rings(&self) -> bool {
        self.rebuild_rings.swap(false, Ordering::AcqRel)
    }
}

#[derive(Clone, Copy, Debug)]
struct SceneLabel {
    x: f32,
    y: f32,
    text: &'static str,
    selected: bool,
}

#[derive(Default)]
pub struct SceneLabelStore {
    labels: Mutex<Vec<SceneLabel>>,
}

impl SceneLabelStore {
    pub fn new() -> Self {
        Self {
            labels: Mutex::new(Vec::with_capacity(64)),
        }
    }

    pub fn update(&self, sim: &Simulation, camera: Camera3D, viewport: [u32; 2]) {
        if let Ok(mut labels) = self.labels.lock() {
            labels.clear();
            for (index, body) in sim.bodies().iter().enumerate() {
                if let Some((screen, _depth)) = camera.world_to_screen(body.pos, viewport) {
                    labels.push(SceneLabel {
                        x: screen.x + 10.0,
                        y: screen.y - 16.0,
                        text: body.name,
                        selected: sim.selected_index() == Some(index),
                    });
                }
            }
        }
    }
}

pub struct SceneLabelOverlay {
    store: Arc<SceneLabelStore>,
}

impl SceneLabelOverlay {
    pub fn new(store: Arc<SceneLabelStore>) -> Self {
        Self { store }
    }
}

impl Widget for SceneLabelOverlay {
    fn update(&mut self, _dt: f32, _input: &InputManager) {}

    fn instances(&self) -> Vec<WidgetInstance> {
        Vec::new()
    }

    fn paint(&self, _ctx: &mut PaintCtx) {}

    fn prepare_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        if let Ok(labels) = self.store.labels.lock() {
            for label in labels.iter() {
                let scale = if label.selected { 15.0 } else { 12.0 };
                let mut buffer = Buffer::new(font_system, Metrics::new(scale, scale * 1.25));
                set_buffer_size(&mut buffer, font_system, 220.0, 28.0);
                let color = if label.selected {
                    Color::rgba(255, 226, 120, 255)
                } else {
                    Color::rgba(176, 204, 230, 220)
                };
                set_buffer_text(
                    &mut buffer,
                    font_system,
                    label.text,
                    Attrs::new().family(Family::SansSerif).color(color),
                );
                shape_text(&mut buffer, font_system);
                buffers.push(buffer);
            }
        }
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if let Ok(labels) = self.store.labels.lock() {
            for label in labels.iter() {
                if let Some(buffer) = buffers.get(*bi) {
                    areas.push(text_area(
                        buffer,
                        label.x,
                        label.y,
                        TextBounds {
                            left: 0,
                            top: 0,
                            right: 4096,
                            bottom: 4096,
                        },
                        if label.selected {
                            Color::rgba(255, 226, 120, 255)
                        } else {
                            Color::rgba(176, 204, 230, 220)
                        },
                    ));
                    *bi += 1;
                }
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
pub struct SelectionTelemetry {
    name: Mutex<String>,
}

impl SelectionTelemetry {
    pub fn set_name(&self, name: &str) {
        if let Ok(mut value) = self.name.lock() {
            value.clear();
            value.push_str(name);
        }
    }

    fn write_text(&self, out: &mut String) {
        out.clear();
        if let Ok(name) = self.name.lock() {
            if name.is_empty() {
                out.push_str("No object selected");
            } else {
                let _ = write!(out, "Selected: {name}");
            }
        }
    }
}

#[derive(Default)]
pub struct UiTelemetry {
    bodies: AtomicU32,
    asteroids: AtomicU32,
    prediction: AtomicU32,
    scene_instances: AtomicU32,
    line_vertices: AtomicU32,
    body_capacity: AtomicU32,
    line_capacity: AtomicU32,
    stable_bits: AtomicU32,
    elapsed_bits: AtomicU32,
}

impl UiTelemetry {
    pub fn set(&self, sim: SimulationStats, render: SpaceRenderStats) {
        self.bodies.store(sim.bodies as u32, Ordering::Relaxed);
        self.asteroids
            .store(sim.asteroids as u32, Ordering::Relaxed);
        self.prediction
            .store(sim.prediction_points as u32, Ordering::Relaxed);
        self.scene_instances
            .store(render.instances as u32, Ordering::Relaxed);
        self.line_vertices
            .store(render.line_vertices as u32, Ordering::Relaxed);
        self.body_capacity
            .store(render.body_capacity as u32, Ordering::Relaxed);
        self.line_capacity
            .store(render.line_capacity as u32, Ordering::Relaxed);
        self.stable_bits
            .store(sim.stable_score.to_bits(), Ordering::Relaxed);
        self.elapsed_bits
            .store(sim.elapsed.to_bits(), Ordering::Relaxed);
    }

    fn write_text(&self, out: &mut String) {
        out.clear();
        let stable = f32::from_bits(self.stable_bits.load(Ordering::Relaxed));
        let elapsed = f32::from_bits(self.elapsed_bits.load(Ordering::Relaxed));
        let _ = writeln!(out, "Bodies        {}", self.bodies.load(Ordering::Relaxed));
        let _ = writeln!(
            out,
            "Asteroids     {}",
            self.asteroids.load(Ordering::Relaxed)
        );
        let _ = writeln!(
            out,
            "Prediction    {} pts",
            self.prediction.load(Ordering::Relaxed)
        );
        let _ = writeln!(
            out,
            "GPU instances {}",
            self.scene_instances.load(Ordering::Relaxed)
        );
        let _ = writeln!(
            out,
            "Line vertices {}",
            self.line_vertices.load(Ordering::Relaxed)
        );
        let _ = writeln!(
            out,
            "Caps          {}/{}",
            self.body_capacity.load(Ordering::Relaxed),
            self.line_capacity.load(Ordering::Relaxed)
        );
        let _ = writeln!(out, "Stability     {:>3.0}%", stable * 100.0);
        let _ = write!(out, "Elapsed       {:>5.1}s", elapsed);
    }
}

pub struct TelemetryReadout {
    pos: [f32; 2],
    size: [f32; 2],
    rect: Rect,
    clip_rect: Option<Rect>,
    telemetry: Arc<UiTelemetry>,
    text: String,
}

impl TelemetryReadout {
    pub fn new(pos: [f32; 2], size: [f32; 2], telemetry: Arc<UiTelemetry>) -> Self {
        Self {
            pos,
            size,
            rect: Rect::new(pos[0], pos[1], size[0], size[1]),
            clip_rect: None,
            telemetry,
            text: String::with_capacity(220),
        }
    }
}

impl Widget for TelemetryReadout {
    fn update(&mut self, _dt: f32, _input: &InputManager) {}

    fn instances(&self) -> Vec<WidgetInstance> {
        Vec::new()
    }

    fn paint(&self, _ctx: &mut PaintCtx) {}

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let size = constraints.constrain(Size::new(self.size[0], self.size[1]));
        self.size = [size.width, size.height];
        self.rect.width = size.width;
        self.rect.height = size.height;
        size
    }

    fn set_position(&mut self, position: Point) {
        self.pos = [position.x, position.y];
        self.rect.x = position.x;
        self.rect.y = position.y;
    }

    fn set_clip_rect(&mut self, clip: Rect) {
        self.clip_rect = Some(clip);
    }

    fn get_rect(&self) -> Rect {
        self.rect
    }

    fn prepare_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        self.telemetry.write_text(&mut self.text);
        let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 19.0));
        set_buffer_size(&mut buffer, font_system, self.size[0], self.size[1]);
        set_buffer_text(
            &mut buffer,
            font_system,
            &self.text,
            Attrs::new()
                .family(Family::Monospace)
                .color(Color::rgba(214, 226, 240, 255)),
        );
        shape_text(&mut buffer, font_system);
        buffers.push(buffer);
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if let Some(buffer) = buffers.get(*bi) {
            let bounds = if let Some(clip) = self.clip_rect {
                TextBounds {
                    left: clip.x as i32,
                    top: clip.y as i32,
                    right: (clip.x + clip.width) as i32,
                    bottom: (clip.y + clip.height) as i32,
                }
            } else {
                TextBounds {
                    left: self.pos[0] as i32,
                    top: self.pos[1] as i32,
                    right: (self.pos[0] + self.size[0]) as i32,
                    bottom: (self.pos[1] + self.size[1]) as i32,
                }
            };
            areas.push(text_area(
                buffer,
                self.pos[0],
                self.pos[1],
                bounds,
                Color::rgba(214, 226, 240, 255),
            ));
            *bi += 1;
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct SelectionReadout {
    pos: [f32; 2],
    size: [f32; 2],
    rect: Rect,
    telemetry: Arc<SelectionTelemetry>,
    text: String,
}

impl SelectionReadout {
    pub fn new(pos: [f32; 2], size: [f32; 2], telemetry: Arc<SelectionTelemetry>) -> Self {
        Self {
            pos,
            size,
            rect: Rect::new(pos[0], pos[1], size[0], size[1]),
            telemetry,
            text: String::with_capacity(96),
        }
    }
}

impl Widget for SelectionReadout {
    fn update(&mut self, _dt: f32, _input: &InputManager) {}

    fn instances(&self) -> Vec<WidgetInstance> {
        Vec::new()
    }

    fn paint(&self, _ctx: &mut PaintCtx) {}

    fn set_position(&mut self, position: Point) {
        self.pos = [position.x, position.y];
        self.rect.x = position.x;
        self.rect.y = position.y;
    }

    fn get_rect(&self) -> Rect {
        self.rect
    }

    fn prepare_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        self.telemetry.write_text(&mut self.text);
        let mut buffer = Buffer::new(font_system, Metrics::new(15.0, 19.0));
        set_buffer_size(&mut buffer, font_system, self.size[0], self.size[1]);
        set_buffer_text(
            &mut buffer,
            font_system,
            &self.text,
            Attrs::new()
                .family(Family::SansSerif)
                .color(Color::rgba(230, 238, 248, 255)),
        );
        shape_text(&mut buffer, font_system);
        buffers.push(buffer);
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if let Some(buffer) = buffers.get(*bi) {
            areas.push(text_area(
                buffer,
                self.pos[0],
                self.pos[1],
                TextBounds {
                    left: self.pos[0] as i32,
                    top: self.pos[1] as i32,
                    right: (self.pos[0] + self.size[0]) as i32,
                    bottom: (self.pos[1] + self.size[1]) as i32,
                },
                Color::rgba(230, 238, 248, 255),
            ));
            *bi += 1;
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn build_editor(
    gui: &mut GuiManager,
    signals: &EditorSignals,
    actions: &EditorActions,
    telemetry: Arc<UiTelemetry>,
    selection: Arc<SelectionTelemetry>,
    labels: Arc<SceneLabelStore>,
) {
    let panel = gui.add(
        Panel::new([PANEL_X, PANEL_Y], [PANEL_W, PANEL_H])
            .draggable(true)
            .resizable(true)
            .min_size([340.0, 520.0])
            .radius(14.0)
            .color([0.055, 0.065, 0.088, 0.86]),
    );

    let mut children = Vec::with_capacity(30);
    children.push(
        gui.add(
            Label::new([CONTENT_X, 48.0], "Interstellar Architect")
                .scale(27.0)
                .color([0.74, 0.88, 1.0, 1.0]),
        ),
    );
    children.push(
        gui.add(
            Label::new([CONTENT_X, 84.0], "Stabilize a three-star system")
                .scale(14.0)
                .color([0.66, 0.72, 0.80, 1.0]),
        ),
    );

    let reset_actions = actions.clone();
    children.push(
        gui.add(
            Button::new([CONTENT_X, 118.0], [102.0, 36.0], "Reset")
                .colors(
                    [0.22, 0.26, 0.34, 0.96],
                    [0.30, 0.36, 0.46, 1.0],
                    [0.14, 0.17, 0.24, 1.0],
                )
                .on_click(move || reset_actions.request_reset()),
        ),
    );

    let stabilize_actions = actions.clone();
    children.push(
        gui.add(
            Button::new([CONTENT_X + 114.0, 118.0], [122.0, 36.0], "Stabilize")
                .colors(
                    [0.16, 0.34, 0.34, 0.96],
                    [0.20, 0.45, 0.45, 1.0],
                    [0.10, 0.24, 0.26, 1.0],
                )
                .on_click(move || stabilize_actions.request_stabilize()),
        ),
    );

    let rebuild_actions = actions.clone();
    children.push(
        gui.add(
            Button::new([CONTENT_X + 248.0, 118.0], [86.0, 36.0], "Rings")
                .colors(
                    [0.30, 0.24, 0.16, 0.96],
                    [0.42, 0.32, 0.19, 1.0],
                    [0.22, 0.16, 0.10, 1.0],
                )
                .on_click(move || rebuild_actions.request_rebuild_rings()),
        ),
    );

    children.push(
        gui.add(
            Label::new([CONTENT_X, 174.0], "SIMULATION")
                .scale(15.0)
                .color([0.40, 0.78, 1.0, 1.0]),
        ),
    );
    children.push(
        gui.add(
            SliderLabeled::new_f32(
                [CONTENT_X, 196.0],
                CONTENT_W,
                "Gravity",
                10.0,
                140.0,
                signals.gravity.get(),
            )
            .suffix("")
            .bind_f32_signal(signals.gravity.clone()),
        ),
    );
    children.push(
        gui.add(
            SliderLabeled::new_f32(
                [CONTENT_X, 258.0],
                CONTENT_W,
                "Time Scale",
                0.0,
                3.0,
                signals.time_scale.get(),
            )
            .suffix("x")
            .bind_f32_signal(signals.time_scale.clone()),
        ),
    );
    children.push(
        gui.add(
            SliderLabeled::new_f32(
                [CONTENT_X, 320.0],
                CONTENT_W,
                "Launch Mass",
                8.0,
                180.0,
                signals.launch_mass.get(),
            )
            .bind_f32_signal(signals.launch_mass.clone()),
        ),
    );
    children.push(
        gui.add(
            SliderLabeled::new_u32(
                [CONTENT_X, 382.0],
                CONTENT_W,
                "Prediction",
                24,
                720,
                signals.prediction_steps.get(),
            )
            .suffix(" pts")
            .bind_u32_signal(signals.prediction_steps.clone()),
        ),
    );
    children.push(
        gui.add(
            SliderLabeled::new_u32(
                [CONTENT_X, 444.0],
                CONTENT_W,
                "Asteroids",
                0,
                6000,
                signals.asteroid_count.get(),
            )
            .bind_u32_signal(signals.asteroid_count.clone()),
        ),
    );

    children.push(
        gui.add(
            Label::new([CONTENT_X, 518.0], "VIEW")
                .scale(15.0)
                .color([0.40, 0.78, 1.0, 1.0]),
        ),
    );
    children.push(
        gui.add(
            Checkbox::new([CONTENT_X, 544.0])
                .with_label("Pause simulation")
                .bind_signal(signals.paused.clone()),
        ),
    );
    children.push(
        gui.add(
            Checkbox::new([CONTENT_X, 578.0])
                .with_label("Show launch prediction")
                .bind_signal(signals.show_prediction.clone()),
        ),
    );
    children.push(
        gui.add(
            Checkbox::new([CONTENT_X, 612.0])
                .with_label("Draw asteroid rings")
                .bind_signal(signals.show_rings.clone()),
        ),
    );

    children.push(
        gui.add(
            ProgressBar::new([CONTENT_X + 148.0, 648.0], [CONTENT_W - 148.0, 13.0])
                .bind_signal(signals.stability.clone()),
        ),
    );
    children.push(
        gui.add(
            Label::new([CONTENT_X, 642.0], "Stability")
                .scale(14.0)
                .color([0.78, 0.86, 0.94, 1.0]),
        ),
    );

    children.push(gui.add(TelemetryReadout::new(
        [CONTENT_X + 4.0, 668.0],
        [CONTENT_W - 8.0, 150.0],
        telemetry,
    )));

    gui.add(SceneLabelOverlay::new(labels));

    let free_actions = actions.clone();
    gui.add(
        Button::new([1050.0, 22.0], [78.0, 32.0], "Free")
            .colors(
                [0.14, 0.18, 0.25, 0.92],
                [0.20, 0.28, 0.38, 1.0],
                [0.10, 0.14, 0.20, 1.0],
            )
            .on_click(move || free_actions.set_camera_mode(0)),
    );
    let follow_actions = actions.clone();
    gui.add(
        Button::new([1136.0, 22.0], [88.0, 32.0], "Follow")
            .colors(
                [0.14, 0.18, 0.25, 0.92],
                [0.20, 0.28, 0.38, 1.0],
                [0.10, 0.14, 0.20, 1.0],
            )
            .on_click(move || follow_actions.set_camera_mode(1)),
    );
    let tactical_actions = actions.clone();
    gui.add(
        Button::new([1232.0, 22.0], [94.0, 32.0], "Tactical")
            .colors(
                [0.14, 0.18, 0.25, 0.92],
                [0.20, 0.28, 0.38, 1.0],
                [0.10, 0.14, 0.20, 1.0],
            )
            .on_click(move || tactical_actions.set_camera_mode(2)),
    );

    gui.add(
        Label::new(
            [PANEL_X + PANEL_W + 24.0, 32.0],
            "Drag in space to launch a body",
        )
        .scale(16.0)
        .color([0.72, 0.80, 0.90, 0.82]),
    );

    gui.register_clip_group(panel, children);

    let inspector = gui.add(
        Panel::new([INSPECTOR_X, INSPECTOR_Y], [INSPECTOR_W, INSPECTOR_H])
            .draggable(true)
            .resizable(false)
            .radius(12.0)
            .color([0.045, 0.055, 0.076, 0.84]),
    );
    let ix = INSPECTOR_X + 22.0;
    let iw = INSPECTOR_W - 44.0;
    let inspector_children = vec![
        gui.add(
            Label::new([ix, INSPECTOR_Y + 22.0], "OBJECT INSPECTOR")
                .scale(17.0)
                .color([0.48, 0.82, 1.0, 1.0]),
        ),
        gui.add(SelectionReadout::new(
            [ix, INSPECTOR_Y + 54.0],
            [iw, 28.0],
            selection,
        )),
        gui.add(
            SliderLabeled::new_f32(
                [ix, INSPECTOR_Y + 92.0],
                iw,
                "Radius",
                6.0,
                96.0,
                signals.selected_radius.get(),
            )
            .bind_f32_signal(signals.selected_radius.clone()),
        ),
        gui.add(
            SliderLabeled::new_f32(
                [ix, INSPECTOR_Y + 154.0],
                iw,
                "Mass",
                8.0,
                900.0,
                signals.selected_mass.get(),
            )
            .bind_f32_signal(signals.selected_mass.clone()),
        ),
        gui.add(
            SliderLabeled::new_f32(
                [ix, INSPECTOR_Y + 216.0],
                iw,
                "Spin",
                -4.0,
                4.0,
                signals.selected_spin.get(),
            )
            .bind_f32_signal(signals.selected_spin.clone()),
        ),
        gui.add(
            SliderLabeled::new_f32(
                [ix, INSPECTOR_Y + 278.0],
                iw,
                "Atmosphere",
                0.0,
                1.0,
                signals.selected_atmosphere.get(),
            )
            .bind_f32_signal(signals.selected_atmosphere.clone()),
        ),
        gui.add(
            SliderLabeled::new_f32(
                [ix, INSPECTOR_Y + 340.0],
                iw,
                "Roughness",
                0.0,
                1.0,
                signals.selected_roughness.get(),
            )
            .bind_f32_signal(signals.selected_roughness.clone()),
        ),
    ];
    gui.register_clip_group(inspector, inspector_children);
}
