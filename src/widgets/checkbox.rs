use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::binding::BoolSignal;
use crate::gui::command::{CommandId, UpdateCtx};
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use glam::Vec2;
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, TextArea, TextBounds};
use std::any::Any;
use std::sync::{Arc, Mutex};

pub type CheckboxCallback = Arc<Mutex<dyn FnMut(bool) + Send + Sync>>;

pub struct Checkbox {
    pub pos: [f32; 2],
    pub size: f32,
    natural_size: f32,
    pub checked: bool,
    pub label: String,
    pub spacing: f32,
    rect: Rect,
    clip_rect: Option<Rect>,

    binding: Option<Arc<Mutex<bool>>>,
    signal: Option<BoolSignal>,
    on_change: Option<CheckboxCallback>,
    on_change_cmd: Option<CommandId<bool>>,
}

impl Checkbox {
    pub fn new(pos: [f32; 2]) -> Self {
        Self {
            pos,
            size: 24.0,
            natural_size: 24.0,
            checked: false,
            label: String::new(),
            spacing: 10.0,
            rect: Rect::new(pos[0], pos[1], 24.0, 24.0),
            clip_rect: None,
            binding: None,
            signal: None,
            on_change: None,
            on_change_cmd: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn bind(mut self, target: Arc<Mutex<bool>>) -> Self {
        if let Ok(val) = target.lock() {
            self.checked = *val;
        }
        self.binding = Some(target);
        self
    }

    pub fn bind_signal(mut self, target: BoolSignal) -> Self {
        self.checked = target.get();
        self.signal = Some(target);
        self
    }

    pub fn on_change<F: FnMut(bool) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Arc::new(Mutex::new(f)));
        self
    }

    pub fn on_change_cmd(mut self, command: CommandId<bool>) -> Self {
        self.on_change_cmd = Some(command);
        self
    }

    pub fn set_checked(&mut self, checked: bool) {
        if self.checked != checked {
            self.checked = checked;
            self.sync_binding();
            self.trigger_callback();
        }
    }
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    fn sync_binding(&self) {
        if let Some(signal) = &self.signal {
            signal.set(self.checked);
        }
        if let Some(b) = &self.binding
            && let Ok(mut v) = b.lock()
        {
            *v = self.checked;
        }
    }

    fn trigger_callback(&self) {
        if let Some(cb) = &self.on_change
            && let Ok(mut f) = cb.lock()
        {
            f(self.checked);
        }
    }

    fn get_total_width(&self) -> f32 {
        if self.label.is_empty() {
            self.size
        } else {
            self.size + self.spacing + self.label.chars().count() as f32 * 9.0
        }
    }
}

impl Widget for Checkbox {
    fn update(&mut self, _dt: f32, input: &InputManager) {
        if let Some(signal) = &self.signal {
            self.checked = signal.get();
        }
        if let Some(b) = &self.binding
            && let Ok(v) = b.lock()
        {
            self.checked = *v;
        }

        let total_width = self.get_total_width();
        let min = Vec2::from_array(self.pos);
        let max = min + Vec2::new(total_width, self.size);
        let mut mouse_in = input.mouse_pos.cmpge(min).all() && input.mouse_pos.cmplt(max).all();

        if mouse_in && let Some(clip) = self.clip_rect {
            let cmin = Vec2::new(clip.x, clip.y);
            let cmax = Vec2::new(clip.x + clip.width, clip.y + clip.height);
            if !input.mouse_pos.cmpge(cmin).all() || !input.mouse_pos.cmplt(cmax).all() {
                mouse_in = false;
            }
        }

        if mouse_in && input.lmb.just_pressed {
            self.checked = !self.checked;
            self.sync_binding();
            self.trigger_callback();
        }
    }

    fn update_ctx(&mut self, dt: f32, input: &InputManager, ctx: &mut UpdateCtx) {
        let before = self.checked;
        self.update(dt, input);
        if before != self.checked
            && let Some(command) = self.on_change_cmd
        {
            ctx.emit_bool(command, self.checked);
        }
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let (clip_min, clip_max, use_clip) = clip_info(self.clip_rect);

        let outer = WidgetInstance {
            pos: self.pos,
            size: [self.size, self.size],
            color: [0.22, 0.22, 0.25, 1.0],
            radius: 4.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        };

        let mut instances = vec![outer];

        if self.checked {
            let inset = 3.0;
            instances.push(WidgetInstance {
                pos: [self.pos[0] + inset, self.pos[1] + inset],
                size: [self.size - inset * 2.0, self.size - inset * 2.0],
                color: [0.0, 0.7, 0.4, 1.0],
                radius: 2.0,
                clip_min,
                clip_max,
                use_clip,
                ..Default::default()
            });
        }

        instances
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let (clip_min, clip_max, use_clip) = clip_info(self.clip_rect);
        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: [self.size, self.size],
            color: [0.22, 0.22, 0.25, 1.0],
            radius: 4.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });

        if self.checked {
            let inset = 3.0;
            ctx.push_instance(WidgetInstance {
                pos: [self.pos[0] + inset, self.pos[1] + inset],
                size: [self.size - inset * 2.0, self.size - inset * 2.0],
                color: [0.0, 0.7, 0.4, 1.0],
                radius: 2.0,
                clip_min,
                clip_max,
                use_clip,
                ..Default::default()
            });
        }
    }

    fn prepare_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        if self.label.is_empty() {
            return;
        }

        let mut buffer = Buffer::new(font_system, Metrics::new(16.0, 20.0));
        buffer.set_size(font_system, 500.0, 30.0);
        buffer.set_text(
            font_system,
            &self.label,
            Attrs::new()
                .family(Family::SansSerif)
                .color(Color::rgb(230, 230, 240)),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(font_system);
        buffers.push(buffer);
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if self.label.is_empty() {
            return;
        }

        if let Some(buffer) = buffers.get(*bi) {
            let left = self.pos[0] + self.size + self.spacing;
            let top = self.pos[1] + (self.size - 20.0) / 2.0;

            let bounds = if let Some(clip) = self.clip_rect {
                TextBounds {
                    left: clip.x as i32,
                    top: clip.y as i32,
                    right: (clip.x + clip.width) as i32,
                    bottom: (clip.y + clip.height) as i32,
                }
            } else {
                TextBounds {
                    left: left as i32,
                    top: top as i32,
                    right: (left + 400.0) as i32,
                    bottom: (top + 20.0) as i32,
                }
            };

            areas.push(TextArea {
                buffer,
                left,
                top,
                scale: 1.0,
                bounds,
                default_color: Color::rgb(230, 230, 240),
            });
            *bi += 1;
        }
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let total_width = self.get_total_width();
        let size = constraints.constrain(Size::new(total_width, self.natural_size));
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
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[inline]
fn clip_info(c: Option<Rect>) -> ([f32; 2], [f32; 2], f32) {
    match c {
        Some(r) => ([r.x, r.y], [r.x + r.width, r.y + r.height], 1.0),
        None => ([0.0; 2], [1e5; 2], 0.0),
    }
}
