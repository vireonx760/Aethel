use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::binding::{SelectionSignal, VecSignal};
use crate::gui::command::{CommandId, UpdateCtx};
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use glam::Vec2;
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, TextArea, TextBounds};
use std::any::Any;
use std::sync::{Arc, Mutex};

pub trait ComboBoxItem: Send + Sync + Clone + 'static {
    fn display_name(&self) -> String;
    fn value(&self) -> String;
}

impl ComboBoxItem for String {
    fn display_name(&self) -> String {
        self.clone()
    }

    fn value(&self) -> String {
        self.clone()
    }
}

impl ComboBoxItem for &'static str {
    fn display_name(&self) -> String {
        self.to_string()
    }

    fn value(&self) -> String {
        self.to_string()
    }
}

pub type ComboBoxCallback<T> = Arc<Mutex<dyn FnMut(&T) + Send + Sync>>;

pub struct ComboBox<T: ComboBoxItem> {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    natural_size: [f32; 2],
    pub items: Vec<T>,
    pub selected_index: Option<usize>,
    pub selected_items: Vec<usize>,
    pub open: bool,
    pub multi_select: bool,
    rect: Rect,
    clip_rect: Option<Rect>,

    binding_single: Option<Arc<Mutex<Option<T>>>>,
    binding_multi: Option<Arc<Mutex<Vec<T>>>>,
    signal_single: Option<SelectionSignal<T>>,
    signal_multi: Option<VecSignal<T>>,
    on_select: Option<ComboBoxCallback<T>>,
    on_select_cmd: Option<CommandId<usize>>,
}

impl<T: ComboBoxItem> ComboBox<T> {
    pub fn new(pos: [f32; 2], size: [f32; 2], items: Vec<T>) -> Self {
        Self {
            pos,
            size,
            natural_size: size,
            items,
            selected_index: None,
            selected_items: Vec::new(),
            open: false,
            multi_select: false,
            rect: Rect::new(pos[0], pos[1], size[0], size[1]),
            clip_rect: None,
            binding_single: None,
            binding_multi: None,
            signal_single: None,
            signal_multi: None,
            on_select: None,
            on_select_cmd: None,
        }
    }

    pub fn multi_select(mut self, enabled: bool) -> Self {
        self.multi_select = enabled;
        self
    }

    pub fn bind_single(mut self, target: Arc<Mutex<Option<T>>>) -> Self {
        if let Ok(val) = target.lock()
            && let Some(item) = val.as_ref()
        {
            self.selected_index = self.items.iter().position(|i| i.value() == item.value());
        }
        self.binding_single = Some(target);
        self
    }

    pub fn bind_multi(mut self, target: Arc<Mutex<Vec<T>>>) -> Self {
        if let Ok(val) = target.lock() {
            for item in val.iter() {
                if let Some(idx) = self.items.iter().position(|i| i.value() == item.value())
                    && !self.selected_items.contains(&idx)
                {
                    self.selected_items.push(idx);
                }
            }
        }
        self.binding_multi = Some(target);
        self
    }

    pub fn bind_single_signal(mut self, target: SelectionSignal<T>) -> Self {
        if let Some(item) = target.get() {
            self.selected_index = self.items.iter().position(|i| i.value() == item.value());
        }
        self.signal_single = Some(target);
        self
    }

    pub fn bind_multi_signal(mut self, target: VecSignal<T>) -> Self {
        for item in target.get() {
            if let Some(idx) = self.items.iter().position(|i| i.value() == item.value())
                && !self.selected_items.contains(&idx)
            {
                self.selected_items.push(idx);
            }
        }
        self.signal_multi = Some(target);
        self
    }

    pub fn on_select<F>(mut self, callback: F) -> Self
    where
        F: FnMut(&T) + Send + Sync + 'static,
    {
        self.on_select = Some(Arc::new(Mutex::new(callback)));
        self
    }

    pub fn on_select_cmd(mut self, command: CommandId<usize>) -> Self {
        self.on_select_cmd = Some(command);
        self
    }

    pub fn get_selected(&self) -> Option<&T> {
        self.selected_index.and_then(|idx| self.items.get(idx))
    }

    pub fn get_selected_multi(&self) -> Vec<&T> {
        self.selected_items
            .iter()
            .filter_map(|&idx| self.items.get(idx))
            .collect()
    }

    pub fn set_selected(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected_index = Some(index);
            self.sync_binding();
            self.trigger_callback(index);
        }
    }

    fn sync_binding(&self) {
        if self.multi_select {
            if let Some(signal) = &self.signal_multi {
                let values = self
                    .selected_items
                    .iter()
                    .filter_map(|&idx| self.items.get(idx).cloned())
                    .collect();
                signal.replace(values);
            }
            if let Some(binding) = &self.binding_multi
                && let Ok(mut vec) = binding.lock()
            {
                vec.clear();
                for &idx in &self.selected_items {
                    if let Some(item) = self.items.get(idx) {
                        vec.push(item.clone());
                    }
                }
            }
        } else {
            let selected = self
                .selected_index
                .and_then(|idx| self.items.get(idx).cloned());
            if let Some(signal) = &self.signal_single {
                signal.set(selected.clone());
            }
            if let Some(binding) = &self.binding_single
                && let Ok(mut opt) = binding.lock()
            {
                *opt = selected;
            }
        }
    }

    fn sync_from_signal(&mut self) {
        if self.multi_select {
            if let Some(signal) = &self.signal_multi {
                self.selected_items.clear();
                for item in signal.get() {
                    if let Some(idx) = self.items.iter().position(|i| i.value() == item.value()) {
                        self.selected_items.push(idx);
                    }
                }
            }
        } else if let Some(signal) = &self.signal_single {
            self.selected_index = signal
                .get()
                .and_then(|item| self.items.iter().position(|i| i.value() == item.value()));
        }
    }

    fn emit_command(&self, index: usize, ctx: &mut UpdateCtx) {
        if let Some(command) = self.on_select_cmd {
            ctx.emit(command, crate::gui::command::CommandPayload::Index(index));
        }
    }

    fn clip_arrays(&self) -> ([f32; 2], [f32; 2], f32) {
        if let Some(clip) = self.clip_rect {
            (
                [clip.x, clip.y],
                [clip.x + clip.width, clip.y + clip.height],
                1.0,
            )
        } else {
            ([0.0, 0.0], [100000.0, 100000.0], 0.0)
        }
    }

    fn trigger_callback(&self, index: usize) {
        if let Some(item) = self.items.get(index)
            && let Some(cb) = &self.on_select
            && let Ok(mut func) = cb.lock()
        {
            func(item);
        }
    }

    fn get_display_text(&self) -> String {
        if self.multi_select {
            if self.selected_items.is_empty() {
                "Select items...".to_string()
            } else {
                format!("{} selected", self.selected_items.len())
            }
        } else {
            self.selected_index
                .and_then(|i| self.items.get(i))
                .map(|item| item.display_name())
                .unwrap_or_else(|| "Select...".to_string())
        }
    }

    #[inline]
    fn item_height(&self) -> f32 {
        30.0
    }

    fn dropdown_width(&self) -> f32 {
        let text_width = self
            .items
            .iter()
            .map(|item| item.display_name().chars().count() as f32 * 9.5 + 28.0)
            .fold(self.size[0], f32::max);
        text_width.clamp(self.size[0], 420.0)
    }

    fn dropdown_rect(&self) -> Rect {
        Rect::new(
            self.pos[0],
            self.pos[1] + self.size[1] + 2.0,
            self.dropdown_width(),
            self.items.len() as f32 * self.item_height(),
        )
    }

    fn item_rect(&self, index: usize) -> Rect {
        let list = self.dropdown_rect();
        Rect::new(
            list.x,
            list.y + index as f32 * self.item_height(),
            list.width,
            self.item_height(),
        )
    }

    #[inline]
    fn point_in_rect(point: Vec2, rect: Rect) -> bool {
        point.x >= rect.x
            && point.y >= rect.y
            && point.x < rect.x + rect.width
            && point.y < rect.y + rect.height
    }
}

impl<T: ComboBoxItem> Widget for ComboBox<T> {
    fn update(&mut self, _dt: f32, input: &InputManager) {
        if !self.open {
            self.sync_from_signal();
        }

        let clicked = input.lmb.just_pressed;

        let button_min = Vec2::from_array(self.pos);
        let button_max = button_min + Vec2::from_array(self.size);

        let mut mouse_in_visible =
            input.mouse_pos.cmpge(button_min).all() && input.mouse_pos.cmplt(button_max).all();
        if mouse_in_visible && let Some(clip) = self.clip_rect {
            let cmin = Vec2::new(clip.x, clip.y);
            let cmax = Vec2::new(clip.x + clip.width, clip.y + clip.height);
            if !input.mouse_pos.cmpge(cmin).all() || !input.mouse_pos.cmplt(cmax).all() {
                mouse_in_visible = false;
            }
        }

        if clicked && mouse_in_visible {
            self.open = !self.open;
        }

        if self.open && clicked && !mouse_in_visible {
            let mut selected_item = false;

            for (i, _) in self.items.iter().enumerate() {
                let mouse_in_item = Self::point_in_rect(input.mouse_pos, self.item_rect(i));

                if mouse_in_item {
                    selected_item = true;
                    if self.multi_select {
                        if let Some(pos) = self.selected_items.iter().position(|&x| x == i) {
                            self.selected_items.remove(pos);
                        } else {
                            self.selected_items.push(i);
                        }
                        self.sync_binding();
                        self.trigger_callback(i);
                    } else {
                        self.selected_index = Some(i);
                        self.open = false;
                        self.sync_binding();
                        self.trigger_callback(i);
                    }
                    break;
                }
            }

            if !selected_item {
                self.open = false;
            }
        }
    }

    fn update_ctx(&mut self, dt: f32, input: &InputManager, ctx: &mut UpdateCtx) {
        let before_single = self.selected_index;
        let before_multi_len = self.selected_items.len();
        self.update(dt, input);

        if self.multi_select {
            if before_multi_len != self.selected_items.len()
                && let Some(&idx) = self.selected_items.last()
            {
                self.emit_command(idx, ctx);
            }
        } else if before_single != self.selected_index
            && let Some(idx) = self.selected_index
        {
            self.emit_command(idx, ctx);
        }
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let (clip_min, clip_max, use_clip) = self.clip_arrays();
        vec![WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: [0.25, 0.25, 0.28, 1.0],
            radius: 6.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        }]
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let (clip_min, clip_max, use_clip) = self.clip_arrays();
        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: [0.25, 0.25, 0.28, 1.0],
            radius: 6.0,
            clip_min,
            clip_max,
            use_clip,
            ..Default::default()
        });
    }

    fn overlay_instances(&self) -> Vec<WidgetInstance> {
        if !self.open {
            return vec![];
        }
        let list = self.dropdown_rect();
        let item_h = self.item_height();

        let mut out = vec![WidgetInstance {
            pos: [list.x, list.y],
            size: [list.width, list.height],
            color: [0.18, 0.18, 0.21, 1.0],
            radius: 6.0,
            use_clip: 0.0,
            ..Default::default()
        }];

        for (i, _) in self.items.iter().enumerate() {
            let selected = if self.multi_select {
                self.selected_items.contains(&i)
            } else {
                self.selected_index == Some(i)
            };
            if selected {
                let row = self.item_rect(i);
                out.push(WidgetInstance {
                    pos: [row.x + 2.0, row.y + 2.0],
                    size: [row.width - 4.0, item_h - 4.0],
                    color: [0.0, 0.55, 0.75, 0.5],
                    radius: 4.0,
                    use_clip: 0.0,
                    ..Default::default()
                });
            }
        }
        out
    }

    fn paint_overlay(&self, ctx: &mut PaintCtx) {
        if !self.open {
            return;
        }

        let list = self.dropdown_rect();
        let item_h = self.item_height();

        ctx.push_instance(WidgetInstance {
            pos: [list.x, list.y],
            size: [list.width, list.height],
            color: [0.18, 0.18, 0.21, 1.0],
            radius: 6.0,
            use_clip: 0.0,
            ..Default::default()
        });

        for (i, _) in self.items.iter().enumerate() {
            let selected = if self.multi_select {
                self.selected_items.contains(&i)
            } else {
                self.selected_index == Some(i)
            };
            if selected {
                let row = self.item_rect(i);
                ctx.push_instance(WidgetInstance {
                    pos: [row.x + 2.0, row.y + 2.0],
                    size: [row.width - 4.0, item_h - 4.0],
                    color: [0.0, 0.55, 0.75, 0.5],
                    radius: 4.0,
                    use_clip: 0.0,
                    ..Default::default()
                });
            }
        }
    }

    fn prepare_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        let display_text = self.get_display_text();
        let mut buffer = Buffer::new(font_system, Metrics::new(18.0, 22.0));
        buffer.set_size(font_system, self.size[0], self.size[1]);
        buffer.set_text(
            font_system,
            &display_text,
            Attrs::new()
                .family(Family::SansSerif)
                .color(Color::rgba(230, 230, 240, 255)),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(font_system);
        buffers.push(buffer);
    }

    fn overlay_text_buffers(&mut self, font_system: &mut FontSystem, buffers: &mut Vec<Buffer>) {
        if !self.open {
            return;
        }
        let item_w = self.dropdown_width();
        for item in &self.items {
            let mut b = Buffer::new(font_system, Metrics::new(18.0, 22.0));
            b.set_size(font_system, item_w - 20.0, self.item_height());
            b.set_text(
                font_system,
                &item.display_name(),
                Attrs::new()
                    .family(Family::SansSerif)
                    .color(Color::rgba(230, 230, 240, 255)),
                Shaping::Advanced,
            );
            b.shape_until_scroll(font_system);
            buffers.push(b);
        }
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if let Some(buffer) = buffers.get(*bi) {
            let left = self.pos[0] + 10.0;
            let top = self.pos[1] + (self.size[1] - 22.0) / 2.0;
            let (bl, bt, br, bb) = if let Some(c) = self.clip_rect {
                (
                    (left as i32).max(c.x as i32),
                    (top as i32).max(c.y as i32),
                    ((self.pos[0] + self.size[0] - 10.0) as i32).min((c.x + c.width) as i32),
                    ((top + 22.0) as i32).min((c.y + c.height) as i32),
                )
            } else {
                (
                    left as i32,
                    top as i32,
                    (self.pos[0] + self.size[0] - 10.0) as i32,
                    (top + 22.0) as i32,
                )
            };
            areas.push(TextArea {
                buffer,
                left,
                top,
                scale: 1.0,
                bounds: TextBounds {
                    left: bl,
                    top: bt,
                    right: br,
                    bottom: bb,
                },
                default_color: Color::rgba(230, 230, 240, 255),
            });
            *bi += 1;
        }
    }

    fn overlay_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        buffers: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if !self.open {
            return;
        }
        for (i, _) in self.items.iter().enumerate() {
            if let Some(buffer) = buffers.get(*bi) {
                let row = self.item_rect(i);
                let left = row.x + 10.0;
                let top = row.y + 4.0;
                areas.push(TextArea {
                    buffer,
                    left,
                    top,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: left as i32,
                        top: top as i32,
                        right: (row.x + row.width - 10.0) as i32,
                        bottom: (row.y + row.height) as i32,
                    },
                    default_color: Color::rgba(230, 230, 240, 255),
                });
                *bi += 1;
            }
        }
    }

    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        let size = constraints.constrain(Size::new(self.natural_size[0], self.natural_size[1]));
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

    fn requests_repaint(&self) -> bool {
        self.open
    }

    fn repaint_interval(&self) -> Option<std::time::Duration> {
        None
    }

    fn overlay_hit_test(&self, point: Point) -> bool {
        if !self.open {
            return false;
        }
        let list = self.dropdown_rect();
        point.x >= list.x
            && point.y >= list.y
            && point.x < list.x + list.width
            && point.y < list.y + list.height
    }

    fn captures_input(&self) -> bool {
        self.open
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropdown_width_grows_for_long_items() {
        let combo = ComboBox::new(
            [10.0, 20.0],
            [80.0, 30.0],
            vec!["Short", "A very long item label"],
        );
        assert!(combo.dropdown_width() > combo.size[0]);
    }

    #[test]
    fn open_dropdown_captures_input_and_hit_tests_overlay() {
        let mut combo = ComboBox::new([10.0, 20.0], [100.0, 30.0], vec!["One", "Two"]);
        combo.open = true;

        assert!(combo.captures_input());
        assert!(combo.overlay_hit_test(Point::new(20.0, 55.0)));
        assert!(!combo.overlay_hit_test(Point::new(20.0, 10.0)));
    }
}
