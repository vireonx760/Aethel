use crate::render::{SpaceRenderContext, SpaceRenderer};
use crate::sim::{Camera3D, Simulation, SimulationSettings};
use crate::ui::{
    EditorActions, EditorSignals, SceneLabelStore, SelectionTelemetry, UiTelemetry, build_editor,
};
use aethel_gui::core::input::InputManager;
use aethel_gui::core::renderer::Renderer;
use aethel_gui::core::scheduler::{FrameScheduler, RedrawReason};
use aethel_gui::gui::geometry::Point;
use aethel_gui::gui::widget::GuiManager;
use std::error::Error;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const WIDTH: u32 = 1440;
const HEIGHT: u32 = 900;
const GUI_GUARD_WIDTH: f32 = 430.0;
const GAME_TICK: Duration = Duration::from_millis(16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CameraMode {
    Free,
    Follow,
    Tactical,
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = DemoApp::new();
    let run_result = event_loop.run_app(&mut app);

    if let Some(error) = app.fatal_error {
        return Err(io::Error::other(error).into());
    }

    run_result?;
    Ok(())
}

struct DemoApp {
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    renderer: Option<Renderer>,
    space_renderer: Option<SpaceRenderer>,
    input: InputManager,
    gui: GuiManager,
    signals: EditorSignals,
    actions: EditorActions,
    telemetry: Arc<UiTelemetry>,
    selection: Arc<SelectionTelemetry>,
    labels: Arc<SceneLabelStore>,
    sim: Simulation,
    camera: Camera3D,
    last_selected: Option<usize>,
    last_asteroid_count: usize,
    text_buffers: Vec<glyphon::Buffer>,
    last_frame: Instant,
    scheduler: FrameScheduler,
    fatal_error: Option<String>,
}

impl DemoApp {
    fn new() -> Self {
        let mut gui = GuiManager::new();
        let signals = EditorSignals::new();
        let actions = EditorActions::default();
        let telemetry = Arc::new(UiTelemetry::default());
        let selection = Arc::new(SelectionTelemetry::default());
        let labels = Arc::new(SceneLabelStore::new());
        build_editor(
            &mut gui,
            &signals,
            &actions,
            Arc::clone(&telemetry),
            Arc::clone(&selection),
            Arc::clone(&labels),
        );

        let sim = Simulation::new();
        let now = Instant::now();
        Self {
            window: None,
            window_id: None,
            renderer: None,
            space_renderer: None,
            input: InputManager::new(),
            gui,
            signals,
            actions,
            telemetry,
            selection,
            labels,
            camera: Camera3D::default(),
            last_selected: sim.selected_index(),
            last_asteroid_count: 1_600,
            sim,
            text_buffers: Vec::with_capacity(192),
            last_frame: now,
            scheduler: FrameScheduler::new(now),
            fatal_error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl ToString) {
        self.fatal_error = Some(error.to_string());
        event_loop.exit();
    }

    fn request_frame_if_needed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let now = Instant::now();
        let game_interval = game_interval(&self.signals, self.sim.launch().is_some());
        let repaint_interval = min_interval(self.gui.next_repaint_interval(), game_interval);
        self.scheduler.set_repaint_interval(repaint_interval, now);

        let mouse_active = self.input.lmb.held || self.input.rmb.held;
        if self.scheduler.wants_redraw(now) || mouse_active {
            window.request_redraw();
        }
        event_loop.set_control_flow(self.scheduler.control_flow(now, mouse_active));
    }

    fn redraw(&mut self) {
        let (Some(renderer), Some(space_renderer)) =
            (self.renderer.as_mut(), self.space_renderer.as_mut())
        else {
            return;
        };

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        self.gui.update(dt, &self.input);
        let camera_mode = camera_mode_from_u32(self.actions.camera_mode());
        update_camera(&mut self.camera, &self.input, camera_mode);

        let settings = settings_from_signals(&self.signals);
        if self.actions.take_reset() {
            self.sim.reset();
            self.last_asteroid_count = settings.asteroid_count;
        }
        if self.actions.take_stabilize() {
            self.sim.stabilize();
        }
        if self.actions.take_rebuild_rings() || settings.asteroid_count != self.last_asteroid_count
        {
            self.sim.rebuild_asteroids(settings.asteroid_count);
            self.last_asteroid_count = settings.asteroid_count;
        }

        let viewport = renderer.surface_size();
        let pointer_over_gui = self
            .gui
            .captures_pointer_at(Point::new(self.input.mouse_pos.x, self.input.mouse_pos.y));
        let gui_guard_width = if pointer_over_gui {
            f32::INFINITY
        } else {
            GUI_GUARD_WIDTH
        };
        self.sim.update(
            dt,
            &settings,
            self.camera,
            &self.input,
            viewport,
            gui_guard_width,
        );
        sync_selected_body(
            &mut self.sim,
            &self.signals,
            &self.selection,
            &mut self.last_selected,
        );
        update_follow_camera(&mut self.camera, &self.sim, camera_mode, dt);
        let sim_stats = self.sim.stats();
        self.signals.stability.set(sim_stats.stable_score);
        self.labels.update(&self.sim, self.camera, viewport);
        self.input.end_frame();

        self.gui.collect_paint();
        let mut text_areas = Vec::with_capacity(self.text_buffers.len().max(192));
        let text_layers = self.gui.prepare_text_layers(
            renderer.font_system(),
            &mut self.text_buffers,
            &mut text_areas,
        );
        renderer.prepare_regular_text_layers(&text_areas, &text_layers.regular);
        renderer.prepare_overlay_text(&text_areas[text_layers.overlay_start..]);

        let frame_paint = self.gui.frame_paint();
        renderer.render_frame_layered_with_prepass(
            frame_paint,
            &text_layers.regular,
            |device, queue, encoder, view| {
                space_renderer.render(SpaceRenderContext {
                    device,
                    queue,
                    encoder,
                    view,
                    sim: &self.sim,
                    settings: &settings,
                    camera: self.camera,
                    viewport,
                    time: sim_stats.elapsed,
                });
            },
        );
        self.telemetry.set(sim_stats, space_renderer.stats());

        let repaint_interval = min_interval(
            self.gui.next_repaint_interval(),
            game_interval(&self.signals, self.sim.launch().is_some()),
        );
        self.scheduler
            .after_redraw_with_interval(now, repaint_interval);
    }
}

impl ApplicationHandler for DemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("Interstellar Architect - AethelGUI GPU Demo")
            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT));
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                self.fail(event_loop, err);
                return;
            }
        };

        let mut renderer = match pollster::block_on(Renderer::new(Arc::clone(&window))) {
            Ok(renderer) => renderer,
            Err(err) => {
                self.fail(event_loop, err);
                return;
            }
        };
        let space_renderer = SpaceRenderer::new(renderer.device(), renderer.surface_format());
        self.gui
            .for_each_custom_shader(|shader| renderer.register_custom_shader(shader));

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.space_renderer = Some(space_renderer);
        self.renderer = Some(renderer);
        self.scheduler
            .mark_dirty(RedrawReason::Resize, Instant::now());
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.request_frame_if_needed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }

        self.input.process_event(&event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
                if let Some(window) = self.window.as_ref() {
                    self.scheduler
                        .mark_dirty(RedrawReason::Resize, Instant::now());
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::CursorMoved { .. } => {
                if let Some(window) = self.window.as_ref() {
                    self.scheduler
                        .mark_dirty(RedrawReason::Input, Instant::now());
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { .. }
            | WindowEvent::KeyboardInput { .. }
            | WindowEvent::MouseWheel { .. } => {
                if let Some(window) = self.window.as_ref() {
                    self.scheduler
                        .mark_dirty(RedrawReason::Input, Instant::now());
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn settings_from_signals(signals: &EditorSignals) -> SimulationSettings {
    SimulationSettings {
        gravity: signals.gravity.get().clamp(10.0, 140.0),
        time_scale: signals.time_scale.get().clamp(0.0, 3.0),
        launch_mass: signals.launch_mass.get().clamp(8.0, 180.0),
        prediction_steps: signals.prediction_steps.get().clamp(24, 720),
        asteroid_count: (signals.asteroid_count.get() as usize).min(6_000),
        show_prediction: signals.show_prediction.get(),
        show_rings: signals.show_rings.get(),
        paused: signals.paused.get(),
        ..SimulationSettings::default()
    }
}

fn game_interval(signals: &EditorSignals, launch_active: bool) -> Option<Duration> {
    (!signals.paused.get() || launch_active).then_some(GAME_TICK)
}

fn min_interval(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn camera_mode_from_u32(mode: u32) -> CameraMode {
    match mode {
        1 => CameraMode::Follow,
        2 => CameraMode::Tactical,
        _ => CameraMode::Free,
    }
}

fn update_camera(camera: &mut Camera3D, input: &InputManager, mode: CameraMode) {
    if input.mouse_pos.x <= GUI_GUARD_WIDTH {
        return;
    }

    if input.rmb.held {
        match mode {
            CameraMode::Tactical => {
                let scale = camera.distance * 0.0014;
                camera.target -= camera.right() * input.mouse_delta.x * scale;
                camera.target += camera.up() * input.mouse_delta.y * scale;
            }
            CameraMode::Free | CameraMode::Follow => {
                if input.shift {
                    let scale = camera.distance * 0.0017;
                    camera.target -= camera.right() * input.mouse_delta.x * scale;
                    camera.target += camera.up() * input.mouse_delta.y * scale;
                } else {
                    camera.yaw -= input.mouse_delta.x * 0.006;
                    camera.pitch =
                        (camera.pitch + input.mouse_delta.y * 0.0045).clamp(-1.35, -0.18);
                }
            }
        }
    }

    if input.scroll_delta.abs() > 0.001 {
        let factor = (1.0 - input.scroll_delta * 0.10).clamp(0.58, 1.72);
        camera.distance = (camera.distance * factor).clamp(360.0, 3_600.0);
    }

    if mode == CameraMode::Tactical {
        camera.pitch = lerp(camera.pitch, -1.30, 0.08);
        camera.yaw = lerp(camera.yaw, -0.02, 0.08);
    }
}

fn update_follow_camera(camera: &mut Camera3D, sim: &Simulation, mode: CameraMode, dt: f32) {
    if mode != CameraMode::Follow {
        return;
    }
    if let Some((_index, body)) = sim.selected_body() {
        let blend = (dt * 7.5).clamp(0.0, 1.0);
        camera.target = camera.target.lerp(body.pos, blend);
        camera.distance = lerp(
            camera.distance,
            (body.radius * 24.0).clamp(520.0, 1_800.0),
            blend * 0.35,
        );
    }
}

fn sync_selected_body(
    sim: &mut Simulation,
    signals: &EditorSignals,
    selection: &SelectionTelemetry,
    last_selected: &mut Option<usize>,
) {
    let selected = sim.selected_index();
    if selected != *last_selected {
        if let Some((_index, body)) = sim.selected_body() {
            signals.selected_radius.set(body.radius);
            signals.selected_mass.set(body.mass);
            signals.selected_spin.set(body.spin_rate);
            signals.selected_atmosphere.set(body.atmosphere);
            signals.selected_roughness.set(body.roughness);
            selection.set_name(body.name);
        } else {
            selection.set_name("");
        }
        *last_selected = selected;
    } else if selected.is_some() {
        sim.apply_selected_body_controls(
            signals.selected_radius.get(),
            signals.selected_mass.get(),
            signals.selected_spin.get(),
            signals.selected_atmosphere.get(),
            signals.selected_roughness.get(),
        );
        if let Some((_index, body)) = sim.selected_body() {
            selection.set_name(body.name);
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_prefers_shorter_deadline() {
        assert_eq!(
            min_interval(
                Some(Duration::from_millis(32)),
                Some(Duration::from_millis(16))
            ),
            Some(Duration::from_millis(16))
        );
    }

    #[test]
    fn paused_without_launch_can_idle() {
        let signals = EditorSignals::new();
        signals.paused.set(true);
        assert_eq!(game_interval(&signals, false), None);
        assert_eq!(game_interval(&signals, true), Some(GAME_TICK));
    }
}
