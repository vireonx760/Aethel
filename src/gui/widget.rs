use crate::core::frame_stats::FrameStats;
use crate::core::input::InputManager;
use crate::core::renderer::{TextLayer, WidgetInstance};
use crate::core::scratch::FrameScratch;
use crate::gui::command::{CommandQueue, UiCommand, UpdateCtx};
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::{FramePaint, PaintCtx, RenderLayer};
use crate::gui::shader::CustomShader;
use glyphon::{Buffer as TextBuffer, FontSystem, TextArea};
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const LAYOUT_MIN: f32 = 24.0;
const MIN_CONTENT: f32 = 8.0;

pub struct UiController<S> {
    pub state: Arc<Mutex<S>>,
}

impl<S: Send + Sync + 'static> UiController<S> {
    pub fn new(state: S) -> Self {
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn action<F>(&self, f: F) -> impl FnMut() + Send + Sync + 'static
    where
        F: Fn(&mut S) + Send + Sync + 'static,
    {
        let state = Arc::clone(&self.state);
        move || {
            if let Ok(mut guard) = state.lock() {
                f(&mut guard);
            }
        }
    }

    pub fn handle(&self) -> Arc<Mutex<S>> {
        Arc::clone(&self.state)
    }

    pub fn read<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&S) -> R,
    {
        self.state.lock().ok().map(|guard| f(&guard))
    }
}

pub trait Widget: Send + Sync {
    fn layout(&mut self, _constraints: BoxConstraints) -> Size {
        Size::ZERO
    }

    fn set_position(&mut self, _position: Point) {}

    fn get_rect(&self) -> Rect {
        Rect::default()
    }

    fn set_clip_rect(&mut self, _clip: Rect) {}

    fn update(&mut self, dt: f32, input: &InputManager);

    fn update_ctx(&mut self, dt: f32, input: &InputManager, _ctx: &mut UpdateCtx) {
        self.update(dt, input);
    }

    fn instances(&self) -> Vec<WidgetInstance>;

    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.push_instances(&self.instances());
    }

    fn prepare_text_buffers(&mut self, _fs: &mut FontSystem, _bufs: &mut Vec<TextBuffer>) {}

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        _bufs: &'a [TextBuffer],
        _areas: &mut Vec<TextArea<'a>>,
        _bi: &mut usize,
    ) {
    }

    fn overlay_instances(&self) -> Vec<WidgetInstance> {
        Vec::new()
    }

    fn paint_overlay(&self, ctx: &mut PaintCtx) {
        let previous = ctx.set_layer(RenderLayer::Overlay);
        ctx.push_instances(&self.overlay_instances());
        ctx.set_layer(previous);
    }

    fn overlay_text_buffers(&mut self, _fs: &mut FontSystem, _bufs: &mut Vec<TextBuffer>) {}

    fn overlay_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        _bufs: &'a [TextBuffer],
        _areas: &mut Vec<TextArea<'a>>,
        _bi: &mut usize,
    ) {
    }

    fn priority_click(&self) -> bool {
        false
    }

    fn requests_repaint(&self) -> bool {
        false
    }

    fn repaint_interval(&self) -> Option<Duration> {
        self.requests_repaint().then_some(Duration::from_millis(16))
    }

    fn overlay_hit_test(&self, _point: Point) -> bool {
        false
    }

    fn captures_input(&self) -> bool {
        false
    }

    fn custom_shaders(&self) -> &[CustomShader] {
        &[]
    }

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Debug, Clone)]
struct ClipGroup {
    panel_idx: usize,
    widget_idxs: Vec<usize>,
    child_offsets: Vec<[f32; 2]>,
    prev_clip: Rect,
}

#[derive(Debug, Clone)]
struct RelayoutGroup {
    panel_idx: usize,
    widget_idx: usize,
    insets: [f32; 4],
    prev_rect: Rect,
}

#[derive(Debug, Clone)]
struct WidgetTextLayer {
    widget_indices: Vec<usize>,
    instance_end: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreparedTextLayers {
    pub regular: Vec<TextLayer>,
    pub overlay_start: usize,
}

#[derive(Debug, Default, Clone)]
struct ZOrder {
    panel_indices: Vec<usize>,
}

impl ZOrder {
    fn register(&mut self, panel_idx: usize) {
        if !self.panel_indices.contains(&panel_idx) {
            self.panel_indices.push(panel_idx);
        }
    }

    fn bring_to_front(&mut self, panel_idx: usize) {
        if let Some(pos) = self.panel_indices.iter().position(|&idx| idx == panel_idx) {
            self.panel_indices.remove(pos);
        }
        self.panel_indices.push(panel_idx);
    }
}

pub struct GuiManager {
    pub widgets: Vec<Box<dyn Widget>>,
    clip_groups: Vec<ClipGroup>,
    relayout_groups: Vec<RelayoutGroup>,
    z_order: ZOrder,
    frame_paint: FramePaint,
    text_layer_specs: Vec<WidgetTextLayer>,
    managed_widgets: Vec<bool>,
    commands: CommandQueue,
    scratch: FrameScratch,
    frame_stats: FrameStats,
    pub last_regular_count: usize,
}

impl GuiManager {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            clip_groups: Vec::new(),
            relayout_groups: Vec::new(),
            z_order: ZOrder::default(),
            frame_paint: FramePaint::with_capacity(512, 64),
            text_layer_specs: Vec::with_capacity(16),
            managed_widgets: Vec::with_capacity(128),
            commands: CommandQueue::with_capacity(32),
            scratch: FrameScratch::new(),
            frame_stats: FrameStats::default(),
            last_regular_count: 0,
        }
    }

    pub fn add<W: Widget + 'static>(&mut self, widget: W) -> usize {
        let idx = self.widgets.len();
        self.widgets.push(Box::new(widget));
        idx
    }

    pub fn register_panel(&mut self, panel_idx: usize) {
        self.z_order.register(panel_idx);
    }

    pub fn bring_to_front(&mut self, panel_idx: usize) {
        self.z_order.bring_to_front(panel_idx);
    }

    pub fn register_clip_group(&mut self, panel_idx: usize, widget_indices: Vec<usize>) {
        let clip = self.widgets[panel_idx].get_rect();
        let child_offsets = widget_indices
            .iter()
            .map(|&widget_idx| {
                let rect = self.widgets[widget_idx].get_rect();
                [rect.x - clip.x, rect.y - clip.y]
            })
            .collect();
        for &widget_idx in &widget_indices {
            self.widgets[widget_idx].set_clip_rect(clip);
        }

        self.z_order.register(panel_idx);
        self.clip_groups.push(ClipGroup {
            panel_idx,
            widget_idxs: widget_indices,
            child_offsets,
            prev_clip: clip,
        });
    }

    pub fn register_relayout_group(
        &mut self,
        panel_idx: usize,
        widget_idx: usize,
        insets: [f32; 4],
    ) {
        let prev_rect = self.widgets[panel_idx].get_rect();
        Self::do_relayout(&mut self.widgets, panel_idx, widget_idx, insets);
        self.relayout_groups.push(RelayoutGroup {
            panel_idx,
            widget_idx,
            insets,
            prev_rect,
        });
    }

    pub fn update(&mut self, dt: f32, input: &InputManager) {
        self.scratch.begin_frame();
        self.commands.clear();
        {
            let mut ctx = UpdateCtx::new(&mut self.commands);
            if let Some(capture_idx) = self
                .widgets
                .iter()
                .rposition(|widget| widget.captures_input())
            {
                self.widgets[capture_idx].update_ctx(dt, input, &mut ctx);
            } else {
                for widget in &mut self.widgets {
                    widget.update_ctx(dt, input, &mut ctx);
                }
            }
        }

        self.update_z_order();
        self.update_relayout_groups();
        self.update_clip_groups();
        self.frame_stats.record_commands(self.commands.stats());
        self.frame_stats.record_scratch(self.scratch.stats());
    }

    pub fn update_parallel(&mut self, dt: f32, input: &InputManager) {
        self.update(dt, input);
    }

    pub fn collect_paint(&mut self) -> &FramePaint {
        self.frame_paint.clear();
        self.text_layer_specs.clear();
        self.frame_stats.next_frame();
        self.prepare_managed_widget_flags();

        {
            let mut ctx = PaintCtx::new(&mut self.frame_paint);

            let mut standalone_widgets = Vec::new();
            for (idx, widget) in self.widgets.iter().enumerate() {
                if !self.managed_widgets[idx] {
                    widget.paint(&mut ctx);
                    standalone_widgets.push(idx);
                }
            }
            if !standalone_widgets.is_empty() {
                ctx.flush_batch();
                self.text_layer_specs.push(WidgetTextLayer {
                    widget_indices: standalone_widgets,
                    instance_end: ctx.instance_len() as u32,
                });
            }

            for &panel_idx in &self.z_order.panel_indices {
                let mut layer_widgets = vec![panel_idx];
                self.widgets[panel_idx].paint(&mut ctx);
                if let Some(group) = self
                    .clip_groups
                    .iter()
                    .find(|group| group.panel_idx == panel_idx)
                {
                    for &widget_idx in &group.widget_idxs {
                        self.widgets[widget_idx].paint(&mut ctx);
                        layer_widgets.push(widget_idx);
                    }
                }
                ctx.flush_batch();
                self.text_layer_specs.push(WidgetTextLayer {
                    widget_indices: layer_widgets,
                    instance_end: ctx.instance_len() as u32,
                });
            }

            ctx.set_layer(RenderLayer::Overlay);
            ctx.clear_clip_stack();
            for widget in &self.widgets {
                widget.paint_overlay(&mut ctx);
            }
        }

        self.last_regular_count = self.frame_paint.regular_count();
        self.frame_stats.record_paint(
            self.frame_paint.instances().len(),
            self.frame_paint.regular_count(),
            self.frame_paint.batches().len(),
            self.frame_paint.stats(),
        );
        &self.frame_paint
    }

    pub fn collect_instances(&mut self) -> &[WidgetInstance] {
        self.collect_paint().instances()
    }

    pub fn frame_paint(&self) -> &FramePaint {
        &self.frame_paint
    }

    pub fn prepare_text<'a>(
        &mut self,
        fs: &mut FontSystem,
        bufs: &'a mut Vec<TextBuffer>,
        areas: &mut Vec<TextArea<'a>>,
    ) -> usize {
        self.prepare_text_layers(fs, bufs, areas).overlay_start
    }

    pub fn prepare_text_layers<'a>(
        &mut self,
        fs: &mut FontSystem,
        bufs: &'a mut Vec<TextBuffer>,
        areas: &mut Vec<TextArea<'a>>,
    ) -> PreparedTextLayers {
        bufs.clear();
        areas.clear();

        let mut specs = self.text_layer_specs.clone();
        if specs.is_empty() {
            specs.push(WidgetTextLayer {
                widget_indices: (0..self.widgets.len()).collect(),
                instance_end: self.last_regular_count as u32,
            });
        }

        for layer in &specs {
            for &widget_idx in &layer.widget_indices {
                self.widgets[widget_idx].prepare_text_buffers(fs, bufs);
            }
        }

        for widget in &mut self.widgets {
            widget.overlay_text_buffers(fs, bufs);
        }

        let mut buffer_index = 0usize;
        let mut regular = Vec::with_capacity(specs.len());
        for layer in &specs {
            let area_start = areas.len();
            for &widget_idx in &layer.widget_indices {
                self.widgets[widget_idx].prepare_text_areas(fs, bufs, areas, &mut buffer_index);
            }
            regular.push(TextLayer::new(layer.instance_end, area_start, areas.len()));
        }
        let overlay_start = areas.len();

        for widget in &self.widgets {
            widget.overlay_text_areas(fs, bufs, areas, &mut buffer_index);
        }

        self.frame_stats.record_text(bufs.len(), areas.len());
        PreparedTextLayers {
            regular,
            overlay_start,
        }
    }

    pub fn commands(&self) -> &[UiCommand] {
        self.commands.commands()
    }

    pub fn command_queue(&self) -> &CommandQueue {
        &self.commands
    }

    pub fn frame_stats(&self) -> &FrameStats {
        &self.frame_stats
    }

    pub fn frame_scratch(&self) -> &FrameScratch {
        &self.scratch
    }

    pub fn for_each_custom_shader(&self, mut f: impl FnMut(&CustomShader)) {
        for widget in &self.widgets {
            for shader in widget.custom_shaders() {
                f(shader);
            }
        }
    }

    pub fn needs_continuous_update(&self) -> bool {
        self.widgets.iter().any(|widget| widget.requests_repaint())
    }

    pub fn next_repaint_interval(&self) -> Option<Duration> {
        self.widgets
            .iter()
            .filter_map(|widget| widget.repaint_interval())
            .min()
    }

    pub fn captures_pointer_at(&self, point: Point) -> bool {
        self.widgets.iter().rev().any(|widget| {
            widget.overlay_hit_test(point)
                || widget.captures_input()
                || widget.get_rect().contains(point)
        })
    }

    fn prepare_managed_widget_flags(&mut self) {
        if self.managed_widgets.len() < self.widgets.len() {
            self.managed_widgets.resize(self.widgets.len(), false);
        }
        self.managed_widgets[..self.widgets.len()].fill(false);

        for &panel_idx in &self.z_order.panel_indices {
            if panel_idx < self.managed_widgets.len() {
                self.managed_widgets[panel_idx] = true;
            }
        }

        for group in &self.clip_groups {
            for &widget_idx in &group.widget_idxs {
                if widget_idx < self.managed_widgets.len() {
                    self.managed_widgets[widget_idx] = true;
                }
            }
        }
    }

    fn update_z_order(&mut self) {
        let activated = self
            .z_order
            .panel_indices
            .iter()
            .copied()
            .find(|&panel_idx| self.widgets[panel_idx].priority_click());

        if let Some(panel_idx) = activated {
            self.z_order.bring_to_front(panel_idx);
        }
    }

    fn update_relayout_groups(&mut self) {
        for index in 0..self.relayout_groups.len() {
            let group = self.relayout_groups[index].clone();
            let cur = self.widgets[group.panel_idx].get_rect();

            if cur.width < LAYOUT_MIN || cur.height < LAYOUT_MIN {
                continue;
            }

            if rect_approx_eq(cur, group.prev_rect) {
                continue;
            }

            Self::do_relayout(
                &mut self.widgets,
                group.panel_idx,
                group.widget_idx,
                group.insets,
            );
            self.relayout_groups[index].prev_rect = cur;
        }
    }

    fn update_clip_groups(&mut self) {
        for index in 0..self.clip_groups.len() {
            let panel_idx = self.clip_groups[index].panel_idx;
            let prev_clip = self.clip_groups[index].prev_clip;
            let cur = self.widgets[panel_idx].get_rect();

            if rect_approx_eq(cur, prev_clip) {
                continue;
            }

            let mut widget_indices = self
                .scratch
                .widget_indices(self.clip_groups[index].widget_idxs.len());
            widget_indices.extend(self.clip_groups[index].widget_idxs.iter().copied());
            let child_offsets = self.clip_groups[index].child_offsets.clone();

            for (slot, widget_idx) in widget_indices.iter().copied().enumerate() {
                if let Some(offset) = child_offsets.get(slot) {
                    self.widgets[widget_idx]
                        .set_position(Point::new(cur.x + offset[0], cur.y + offset[1]));
                }
                self.widgets[widget_idx].set_clip_rect(cur);
            }
            self.clip_groups[index].prev_clip = cur;
        }
    }

    fn do_relayout(
        widgets: &mut [Box<dyn Widget>],
        panel_idx: usize,
        widget_idx: usize,
        insets: [f32; 4],
    ) {
        let panel_rect = widgets[panel_idx].get_rect();
        let max_w = (panel_rect.width - insets[0] - insets[2]).max(0.0);
        let max_h = (panel_rect.height - insets[1] - insets[3]).max(0.0);

        if max_w < MIN_CONTENT {
            return;
        }

        let effective_h = if max_h < 1.0 { f32::INFINITY } else { max_h };
        widgets[widget_idx].layout(BoxConstraints::new(0.0, max_w, 0.0, effective_h));
        widgets[widget_idx].set_position(Point::new(
            panel_rect.x + insets[0],
            panel_rect.y + insets[1],
        ));
    }
}

impl Default for GuiManager {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn rect_approx_eq(a: Rect, b: Rect) -> bool {
    (a.x - b.x).abs() < 0.5
        && (a.y - b.y).abs() < 0.5
        && (a.width - b.width).abs() < 0.5
        && (a.height - b.height).abs() < 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestWidget {
        rect: Rect,
        clip: Option<Rect>,
        overlay: Option<Rect>,
        instances: Vec<WidgetInstance>,
        activated: bool,
        capture: bool,
    }

    impl TestWidget {
        fn new(rect: Rect) -> Self {
            Self {
                rect,
                clip: None,
                overlay: None,
                instances: vec![WidgetInstance {
                    pos: [rect.x, rect.y],
                    size: [rect.width, rect.height],
                    color: [1.0; 4],
                    ..Default::default()
                }],
                activated: false,
                capture: false,
            }
        }

        fn with_overlay(mut self, rect: Rect) -> Self {
            self.overlay = Some(rect);
            self
        }
    }

    impl Widget for TestWidget {
        fn update(&mut self, _dt: f32, _input: &InputManager) {}

        fn instances(&self) -> Vec<WidgetInstance> {
            self.instances.clone()
        }

        fn set_position(&mut self, position: Point) {
            self.rect.x = position.x;
            self.rect.y = position.y;
        }

        fn get_rect(&self) -> Rect {
            self.rect
        }

        fn set_clip_rect(&mut self, clip: Rect) {
            self.clip = Some(clip);
        }

        fn priority_click(&self) -> bool {
            self.activated
        }

        fn overlay_hit_test(&self, point: Point) -> bool {
            self.overlay.is_some_and(|rect| rect.contains(point))
        }

        fn captures_input(&self) -> bool {
            self.capture
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn relayout_group_positions_child_inside_panel() {
        let mut gui = GuiManager::new();
        let panel = gui.add(TestWidget::new(Rect::new(10.0, 20.0, 200.0, 100.0)));
        let child = gui.add(TestWidget::new(Rect::new(0.0, 0.0, 50.0, 20.0)));
        gui.register_relayout_group(panel, child, [5.0, 6.0, 7.0, 8.0]);

        assert_eq!(gui.widgets[child].get_rect().x, 15.0);
        assert_eq!(gui.widgets[child].get_rect().y, 26.0);
    }

    #[test]
    fn collect_paint_reuses_instance_capacity() {
        let mut gui = GuiManager::new();
        gui.add(TestWidget::new(Rect::new(0.0, 0.0, 10.0, 10.0)));
        gui.collect_paint();
        let cap = gui.collect_paint().instance_capacity();
        gui.collect_paint();
        assert_eq!(gui.collect_paint().instance_capacity(), cap);
    }

    #[test]
    fn text_layers_follow_panel_z_order() {
        let mut gui = GuiManager::new();
        let panel_a = gui.add(TestWidget::new(Rect::new(0.0, 0.0, 100.0, 100.0)));
        let panel_b = gui.add(TestWidget::new(Rect::new(10.0, 10.0, 100.0, 100.0)));
        let child_a = gui.add(TestWidget::new(Rect::new(0.0, 0.0, 10.0, 10.0)));
        let child_b = gui.add(TestWidget::new(Rect::new(0.0, 0.0, 10.0, 10.0)));

        gui.register_clip_group(panel_a, vec![child_a]);
        gui.register_clip_group(panel_b, vec![child_b]);
        gui.bring_to_front(panel_a);
        gui.collect_paint();

        assert_eq!(gui.text_layer_specs.len(), 2);
        assert_eq!(
            gui.text_layer_specs[0].widget_indices,
            vec![panel_b, child_b]
        );
        assert_eq!(
            gui.text_layer_specs[1].widget_indices,
            vec![panel_a, child_a]
        );
        assert!(gui.text_layer_specs[0].instance_end <= gui.text_layer_specs[1].instance_end);
    }

    #[test]
    fn clip_group_children_follow_panel_motion() {
        let mut gui = GuiManager::new();
        let panel = gui.add(TestWidget::new(Rect::new(10.0, 20.0, 200.0, 100.0)));
        let child = gui.add(TestWidget::new(Rect::new(30.0, 55.0, 50.0, 20.0)));
        gui.register_clip_group(panel, vec![child]);

        gui.widgets[panel].set_position(Point::new(110.0, 120.0));
        gui.update_clip_groups();

        let child_rect = gui.widgets[child].get_rect();
        assert_eq!(child_rect.x, 130.0);
        assert_eq!(child_rect.y, 155.0);
        assert_eq!(
            gui.widgets[child]
                .as_any()
                .downcast_ref::<TestWidget>()
                .unwrap()
                .clip,
            Some(Rect::new(110.0, 120.0, 200.0, 100.0))
        );
    }

    #[test]
    fn captures_pointer_checks_widget_rects_and_overlays() {
        let mut gui = GuiManager::new();
        gui.add(
            TestWidget::new(Rect::new(10.0, 20.0, 80.0, 40.0))
                .with_overlay(Rect::new(120.0, 140.0, 60.0, 70.0)),
        );

        assert!(gui.captures_pointer_at(Point::new(20.0, 30.0)));
        assert!(gui.captures_pointer_at(Point::new(150.0, 160.0)));
        assert!(!gui.captures_pointer_at(Point::new(260.0, 260.0)));
    }
}
