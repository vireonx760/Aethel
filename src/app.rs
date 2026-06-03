use crate::core::input::InputManager;
use crate::core::renderer::{Renderer, RendererInitError, RendererOptions};
use crate::core::scheduler::{FrameScheduler, RedrawReason};
use crate::gui::widget::GuiManager;
use crate::ui::{Ui, UiState};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

pub struct AethelGui {
    overlay: bool,
    title: String,
    width: u32,
    height: u32,
    target_fps: Option<u32>,
}

#[derive(Debug)]
pub enum AethelRunError {
    EventLoop(EventLoopError),
    Window(OsError),
    Renderer(RendererInitError),
}

pub type Result<T = ()> = std::result::Result<T, AethelRunError>;

impl fmt::Display for AethelRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventLoop(err) => write!(f, "event loop failed: {err}"),
            Self::Window(err) => write!(f, "window creation failed: {err}"),
            Self::Renderer(err) => write!(f, "renderer initialization failed: {err}"),
        }
    }
}

impl Error for AethelRunError {}

impl From<EventLoopError> for AethelRunError {
    fn from(value: EventLoopError) -> Self {
        Self::EventLoop(value)
    }
}

impl From<OsError> for AethelRunError {
    fn from(value: OsError) -> Self {
        Self::Window(value)
    }
}

impl From<RendererInitError> for AethelRunError {
    fn from(value: RendererInitError) -> Self {
        Self::Renderer(value)
    }
}

impl AethelGui {
    pub fn new() -> Self {
        Self {
            overlay: false,
            title: "AethelGUI".to_string(),
            width: 1280,
            height: 720,
            target_fps: None,
        }
    }

    pub fn overlay(mut self, yes: bool) -> Self {
        self.overlay = yes;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn max_fps(mut self, fps: u32) -> Self {
        self.target_fps = Some(fps.max(1));
        self
    }

    pub fn run<F>(self, build_gui: F) -> Result
    where
        F: FnOnce(&mut GuiManager) + 'static,
    {
        let event_loop = EventLoop::new()?;
        let mut gui = GuiManager::new();
        build_gui(&mut gui);

        let mut app = RuntimeApp::new(self, gui);
        let run_result = event_loop
            .run_app(&mut app)
            .map_err(AethelRunError::EventLoop);

        if let Some(err) = app.fatal_error {
            return Err(err);
        }

        run_result
    }

    pub fn run_ui<F>(self, build_ui: F) -> Result
    where
        F: FnMut(&mut Ui<'_>) + 'static,
    {
        let event_loop = EventLoop::new()?;
        let mut app = RuntimeApp::new_immediate(self, build_ui);
        let run_result = event_loop
            .run_app(&mut app)
            .map_err(AethelRunError::EventLoop);

        if let Some(err) = app.fatal_error {
            return Err(err);
        }

        run_result
    }
}

impl Default for AethelGui {
    fn default() -> Self {
        Self::new()
    }
}

struct RuntimeApp {
    config: AethelGui,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    renderer: Option<Renderer>,
    input: InputManager,
    gui: GuiManager,
    last_frame: Instant,
    scheduler: FrameScheduler,
    frame_budget: Option<Duration>,
    text_buffers: Vec<glyphon::Buffer>,
    fatal_error: Option<AethelRunError>,
    immediate: Option<ImmediateRuntime>,
}

struct ImmediateRuntime {
    state: UiState,
    build: Box<dyn FnMut(&mut Ui<'_>)>,
}

impl RuntimeApp {
    fn new(config: AethelGui, gui: GuiManager) -> Self {
        let now = Instant::now();
        let frame_budget = config
            .target_fps
            .map(|fps| Duration::from_nanos(1_000_000_000 / fps as u64));

        Self {
            config,
            window: None,
            window_id: None,
            renderer: None,
            input: InputManager::new(),
            gui,
            last_frame: now,
            scheduler: FrameScheduler::new(now),
            frame_budget,
            text_buffers: Vec::with_capacity(128),
            fatal_error: None,
            immediate: None,
        }
    }

    fn new_immediate<F>(config: AethelGui, build: F) -> Self
    where
        F: FnMut(&mut Ui<'_>) + 'static,
    {
        let mut app = Self::new(config, GuiManager::new());
        app.immediate = Some(ImmediateRuntime {
            state: UiState::new(),
            build: Box::new(build),
        });
        app
    }

    fn rebuild_immediate_ui(&mut self) {
        let Some(immediate) = self.immediate.as_mut() else {
            return;
        };
        let commands = self.gui.commands().to_vec();
        Ui::rebuild(&mut self.gui, &mut immediate.state, &commands, |ui| {
            (immediate.build)(ui)
        });
    }

    fn register_custom_shaders(&self, renderer: &mut Renderer) {
        self.gui
            .for_each_custom_shader(|shader| renderer.register_custom_shader(shader));
    }

    fn window_attributes(&self) -> WindowAttributes {
        let mut attributes = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .with_transparent(self.config.overlay)
            .with_decorations(!self.config.overlay);

        if self.config.overlay {
            attributes = attributes.with_window_level(WindowLevel::AlwaysOnTop);
        }

        attributes
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: AethelRunError) {
        self.fatal_error = Some(error);
        event_loop.exit();
    }

    fn request_frame_if_needed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let now = Instant::now();
        let mouse_active = self.input.lmb.held || self.input.rmb.held;
        self.scheduler
            .set_repaint_interval(self.gui.next_repaint_interval(), now);

        if self.scheduler.wants_redraw(now) || mouse_active {
            window.request_redraw();
        }

        event_loop.set_control_flow(self.scheduler.control_flow(now, mouse_active));
    }

    fn redraw(&mut self) {
        if self.renderer.is_none() {
            return;
        }

        let now = Instant::now();
        if let Some(budget) = self.frame_budget
            && now.duration_since(self.last_frame) < budget
        {
            return;
        }

        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.5);
        self.last_frame = now;

        self.gui.update(dt, &self.input);
        self.rebuild_immediate_ui();
        self.input.end_frame();

        self.gui.collect_paint();

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        self.gui
            .for_each_custom_shader(|shader| renderer.register_custom_shader(shader));

        let mut text_areas = Vec::with_capacity(self.text_buffers.len().max(128));
        let text_layers = self.gui.prepare_text_layers(
            renderer.font_system(),
            &mut self.text_buffers,
            &mut text_areas,
        );

        let frame_paint = self.gui.frame_paint();
        if !frame_paint.is_empty() {
            renderer.prepare_regular_text_layers(&text_areas, &text_layers.regular);
            renderer.prepare_overlay_text(&text_areas[text_layers.overlay_start..]);
            renderer.render_frame_layered(frame_paint, &text_layers.regular);
        }

        self.scheduler
            .after_redraw_with_interval(now, self.gui.next_repaint_interval());
    }
}

impl ApplicationHandler for RuntimeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match event_loop.create_window(self.window_attributes()) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                self.fail(event_loop, err.into());
                return;
            }
        };

        let renderer_options = if self.config.overlay {
            RendererOptions::overlay()
        } else {
            RendererOptions::default()
        };
        let mut renderer = match pollster::block_on(Renderer::new_with_options(
            Arc::clone(&window),
            renderer_options,
        )) {
            Ok(renderer) => renderer,
            Err(err) => {
                self.fail(event_loop, err.into());
                return;
            }
        };

        self.register_custom_shaders(&mut renderer);
        self.window_id = Some(window.id());
        self.window = Some(window);
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
