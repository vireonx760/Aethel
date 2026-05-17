use crate::core::input::InputManager;
use crate::core::renderer::Renderer;
use crate::core::scheduler::{FrameScheduler, RedrawReason};
use crate::gui::widget::GuiManager;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder, WindowLevel},
};

pub struct AethelGui {
    overlay: bool,
    title: String,
    width: u32,
    height: u32,
    target_fps: Option<u32>,
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

    pub fn run<F>(self, build_gui: F)
    where
        F: FnOnce(&mut GuiManager) + 'static,
    {
        let event_loop = EventLoop::new().unwrap();
        let mut builder = WindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height))
            .with_transparent(self.overlay)
            .with_decorations(!self.overlay);

        if self.overlay {
            builder = builder.with_window_level(WindowLevel::AlwaysOnTop);
        }

        let window: Arc<Window> = Arc::new(builder.build(&event_loop).unwrap());
        let mut renderer = pollster::block_on(Renderer::new(Arc::clone(&window)));

        let mut input = InputManager::new();
        let mut gui = GuiManager::new();
        build_gui(&mut gui);
        gui.for_each_custom_shader(|shader| renderer.register_custom_shader(shader));

        let window_id = window.id();
        let mut last_frame = Instant::now();
        let mut scheduler = FrameScheduler::new(last_frame);
        let frame_budget = self
            .target_fps
            .map(|fps| Duration::from_nanos(1_000_000_000 / fps as u64));

        let mut text_buffers: Vec<glyphon::Buffer> = Vec::with_capacity(128);

        event_loop
            .run(move |event, elwt| match event {
                Event::AboutToWait => {
                    let now = Instant::now();
                    let mouse_active = input.lmb.held || input.rmb.held;
                    scheduler.set_repaint_interval(gui.next_repaint_interval(), now);

                    if scheduler.wants_redraw(now) || mouse_active {
                        window.request_redraw();
                    }

                    elwt.set_control_flow(scheduler.control_flow(now, mouse_active));
                }

                Event::WindowEvent {
                    window_id: wid,
                    event,
                } if wid == window_id => {
                    input.process_event(&event);

                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(size) => {
                            renderer.resize(size);
                            scheduler.mark_dirty(RedrawReason::Resize, Instant::now());
                            window.request_redraw();
                        }
                        WindowEvent::RedrawRequested => {
                            let now = Instant::now();
                            if let Some(budget) = frame_budget
                                && now.duration_since(last_frame) < budget
                            {
                                return;
                            }

                            let dt = now.duration_since(last_frame).as_secs_f32().min(0.5);
                            last_frame = now;

                            gui.update(dt, &input);
                            input.end_frame();

                            gui.collect_paint();
                            let mut text_areas = Vec::with_capacity(text_buffers.len().max(128));
                            let text_layers = gui.prepare_text_layers(
                                renderer.font_system(),
                                &mut text_buffers,
                                &mut text_areas,
                            );

                            let frame_paint = gui.frame_paint();
                            if !frame_paint.is_empty() {
                                renderer
                                    .prepare_regular_text_layers(&text_areas, &text_layers.regular);
                                renderer
                                    .prepare_overlay_text(&text_areas[text_layers.overlay_start..]);
                                renderer.render_frame_layered(frame_paint, &text_layers.regular);
                            }

                            scheduler.after_redraw_with_interval(now, gui.next_repaint_interval());
                        }
                        WindowEvent::CursorMoved { .. } => {
                            scheduler.mark_dirty(RedrawReason::Input, Instant::now());
                            window.request_redraw();
                        }
                        WindowEvent::MouseInput { .. }
                        | WindowEvent::KeyboardInput { .. }
                        | WindowEvent::MouseWheel { .. } => {
                            scheduler.mark_dirty(RedrawReason::Input, Instant::now());
                            window.request_redraw();
                        }
                        _ => {}
                    }
                }

                _ => {}
            })
            .unwrap();
    }
}

impl Default for AethelGui {
    fn default() -> Self {
        Self::new()
    }
}
