use crate::core::input::InputManager;
use crate::core::renderer::WidgetInstance;
use crate::gui::command::UpdateCtx;
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::layout_enums::{Axis, CrossAxisAlignment, MainAxisAlignment};
use crate::gui::paint::PaintCtx;
use crate::gui::widget::Widget;
use glyphon::{Buffer as TextBuffer, FontSystem, TextArea};
use std::any::Any;
use std::time::Duration;

pub struct Flexible {
    pub child: Box<dyn Widget>,
    pub flex: f32,
    pub fill_cross: bool,
}

impl Flexible {
    pub fn new(child: Box<dyn Widget>, flex: f32) -> Self {
        Self {
            child,
            flex,
            fill_cross: false,
        }
    }
    pub fn fill(child: Box<dyn Widget>) -> Self {
        Self {
            child,
            flex: 1.0,
            fill_cross: false,
        }
    }
}

pub struct Flex {
    children: Vec<Flexible>,
    axis: Axis,
    main_align: MainAxisAlignment,
    cross_align: CrossAxisAlignment,
    spacing: f32,
    padding: [f32; 4], // [left, top, right, bottom]
    rect: Rect,
    clip_rect: Option<Rect>,
}

impl Flex {
    pub fn row() -> Self {
        Self {
            children: Vec::new(),
            axis: Axis::Horizontal,
            main_align: MainAxisAlignment::Start,
            cross_align: CrossAxisAlignment::Center,
            spacing: 0.0,
            padding: [0.0; 4],
            rect: Rect::default(),
            clip_rect: None,
        }
    }

    pub fn column() -> Self {
        Self {
            children: Vec::new(),
            axis: Axis::Vertical,
            main_align: MainAxisAlignment::Start,
            cross_align: CrossAxisAlignment::Start,
            spacing: 0.0,
            padding: [0.0; 4],
            rect: Rect::default(),
            clip_rect: None,
        }
    }

    pub fn with_child(mut self, w: Box<dyn Widget>) -> Self {
        self.children.push(Flexible {
            child: w,
            flex: 0.0,
            fill_cross: false,
        });
        self
    }

    pub fn with_fill_child(mut self, w: Box<dyn Widget>) -> Self {
        self.children.push(Flexible {
            child: w,
            flex: 0.0,
            fill_cross: true,
        });
        self
    }

    pub fn with_flex_child(mut self, w: Box<dyn Widget>, flex: f32) -> Self {
        self.children.push(Flexible {
            child: w,
            flex,
            fill_cross: false,
        });
        self
    }

    pub fn with_spacing(mut self, s: f32) -> Self {
        self.spacing = s;
        self
    }
    pub fn with_padding(mut self, p: [f32; 4]) -> Self {
        self.padding = p;
        self
    }
    pub fn with_main_align(mut self, a: MainAxisAlignment) -> Self {
        self.main_align = a;
        self
    }
    pub fn with_cross_align(mut self, a: CrossAxisAlignment) -> Self {
        self.cross_align = a;
        self
    }
}

impl Widget for Flex {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        if self.children.is_empty() {
            let cs = constraints.constrain_max(Size::ZERO);
            self.rect.width = cs.width;
            self.rect.height = cs.height;
            return cs;
        }

        let avail_w = (constraints.max_width - self.padding[0] - self.padding[2]).max(0.0);
        let avail_h = (constraints.max_height - self.padding[1] - self.padding[3]).max(0.0);

        let (main_max, cross_max) = match self.axis {
            Axis::Horizontal => (avail_w, avail_h),
            Axis::Vertical => (avail_h, avail_w),
        };

        let n = self.children.len();
        let total_spacing = if n > 1 {
            self.spacing * (n - 1) as f32
        } else {
            0.0
        };

        let wants_stretch = |item: &Flexible| -> bool {
            item.fill_cross || self.cross_align == CrossAxisAlignment::Stretch
        };

        let mut fixed_main_total = 0.0f32;
        let mut max_cross = 0.0f32;
        let mut total_flex = 0.0f32;

        for item in &mut self.children {
            if item.flex > 0.0 {
                total_flex += item.flex;
                continue;
            }

            let min_cross = if wants_stretch(item) { cross_max } else { 0.0 };

            let child_c = match self.axis {
                Axis::Horizontal => BoxConstraints::new(
                    0.0,
                    f32::INFINITY,
                    min_cross,
                    cross_max, // cross: height
                ),
                Axis::Vertical => BoxConstraints::new(
                    min_cross,
                    cross_max, // cross: width
                    0.0,
                    f32::INFINITY,
                ),
            };

            let sz = item.child.layout(child_c);
            let (child_main, child_cross) = match self.axis {
                Axis::Horizontal => (sz.width, sz.height),
                Axis::Vertical => (sz.height, sz.width),
            };
            fixed_main_total += child_main;
            max_cross = max_cross.max(child_cross);
        }

        let free = (main_max - fixed_main_total - total_spacing).max(0.0);
        if total_flex > 0.0 {
            let unit = free / total_flex;
            let mut remaining_free = free;

            let last_flex_idx = self.children.iter().rposition(|c| c.flex > 0.0);

            for (i, item) in self.children.iter_mut().enumerate() {
                if item.flex <= 0.0 {
                    continue;
                }

                let is_last = Some(i) == last_flex_idx;
                let alloc = if is_last {
                    remaining_free.floor()
                } else {
                    (unit * item.flex).floor()
                };
                remaining_free -= alloc;

                let min_cross = if wants_stretch(item) { cross_max } else { 0.0 };
                let child_c = match self.axis {
                    Axis::Horizontal => BoxConstraints::new(alloc, alloc, min_cross, cross_max),
                    Axis::Vertical => BoxConstraints::new(min_cross, cross_max, alloc, alloc),
                };
                let sz = item.child.layout(child_c);
                let child_cross = match self.axis {
                    Axis::Horizontal => sz.height,
                    Axis::Vertical => sz.width,
                };
                fixed_main_total += alloc;
                max_cross = max_cross.max(child_cross);
            }
        }

        let used_main = (fixed_main_total + total_spacing).min(main_max);
        let used_cross = max_cross.min(cross_max);

        let (total_w, total_h) = match self.axis {
            Axis::Horizontal => (
                used_main + self.padding[0] + self.padding[2],
                used_cross + self.padding[1] + self.padding[3],
            ),
            Axis::Vertical => (
                used_cross + self.padding[0] + self.padding[2],
                used_main + self.padding[1] + self.padding[3],
            ),
        };

        let final_size = constraints.constrain_max(Size::new(total_w, total_h));
        self.rect.width = final_size.width;
        self.rect.height = final_size.height;

        let (p_ms, p_me, p_cs, _p_ce) = match self.axis {
            Axis::Horizontal => (
                self.padding[0],
                self.padding[2],
                self.padding[1],
                self.padding[3],
            ),
            Axis::Vertical => (
                self.padding[1],
                self.padding[3],
                self.padding[0],
                self.padding[2],
            ),
        };
        let final_main = (match self.axis {
            Axis::Horizontal => final_size.width,
            Axis::Vertical => final_size.height,
        } - p_ms
            - p_me)
            .max(0.0);
        let final_cross = (match self.axis {
            Axis::Horizontal => final_size.height,
            Axis::Vertical => final_size.width,
        } - p_cs
            - _p_ce)
            .max(0.0);

        let (mut cursor, gap) =
            self.compute_start_gap(self.main_align, final_main, fixed_main_total, n);

        let orig_x = self.rect.x + self.padding[0];
        let orig_y = self.rect.y + self.padding[1];

        for item in &mut self.children {
            let child_r = item.child.get_rect();
            let (child_main, child_cross) = match self.axis {
                Axis::Horizontal => (child_r.width, child_r.height),
                Axis::Vertical => (child_r.height, child_r.width),
            };

            let cross_pos = match self.cross_align {
                CrossAxisAlignment::Start | CrossAxisAlignment::Stretch => 0.0,
                CrossAxisAlignment::End => (final_cross - child_cross).max(0.0),
                CrossAxisAlignment::Center => ((final_cross - child_cross) / 2.0).max(0.0),
            };

            let pos = match self.axis {
                Axis::Horizontal => Point::new(orig_x + cursor, orig_y + cross_pos),
                Axis::Vertical => Point::new(orig_x + cross_pos, orig_y + cursor),
            };

            item.child.set_position(pos);
            cursor += child_main + gap;
        }

        final_size
    }

    fn set_position(&mut self, position: Point) {
        let dx = position.x - self.rect.x;
        let dy = position.y - self.rect.y;
        self.rect.x = position.x;
        self.rect.y = position.y;
        for item in &mut self.children {
            let r = item.child.get_rect();
            item.child.set_position(Point::new(r.x + dx, r.y + dy));
        }
    }

    fn get_rect(&self) -> Rect {
        self.rect
    }

    fn set_clip_rect(&mut self, clip: Rect) {
        self.clip_rect = Some(clip);
        for item in &mut self.children {
            item.child.set_clip_rect(clip);
        }
    }

    fn update(&mut self, dt: f32, input: &InputManager) {
        if let Some(capture_idx) = self
            .children
            .iter()
            .rposition(|item| item.child.captures_input())
        {
            self.children[capture_idx].child.update(dt, input);
        } else {
            for item in &mut self.children {
                item.child.update(dt, input);
            }
        }
    }

    fn update_ctx(&mut self, dt: f32, input: &InputManager, ctx: &mut UpdateCtx) {
        if let Some(capture_idx) = self
            .children
            .iter()
            .rposition(|item| item.child.captures_input())
        {
            self.children[capture_idx].child.update_ctx(dt, input, ctx);
        } else {
            for item in &mut self.children {
                item.child.update_ctx(dt, input, ctx);
            }
        }
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        self.children
            .iter()
            .flat_map(|c| c.child.instances())
            .collect()
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        for item in &self.children {
            item.child.paint(ctx);
        }
    }

    fn prepare_text_buffers(&mut self, fs: &mut FontSystem, b: &mut Vec<TextBuffer>) {
        for item in &mut self.children {
            item.child.prepare_text_buffers(fs, b);
        }
    }

    fn prepare_text_areas<'a>(
        &self,
        fs: &mut FontSystem,
        b: &'a [TextBuffer],
        a: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        for item in &self.children {
            item.child.prepare_text_areas(fs, b, a, bi);
        }
    }

    fn overlay_instances(&self) -> Vec<WidgetInstance> {
        self.children
            .iter()
            .flat_map(|c| c.child.overlay_instances())
            .collect()
    }

    fn paint_overlay(&self, ctx: &mut PaintCtx) {
        for item in &self.children {
            item.child.paint_overlay(ctx);
        }
    }

    fn overlay_text_buffers(&mut self, fs: &mut FontSystem, b: &mut Vec<TextBuffer>) {
        for item in &mut self.children {
            item.child.overlay_text_buffers(fs, b);
        }
    }

    fn overlay_text_areas<'a>(
        &self,
        fs: &mut FontSystem,
        b: &'a [TextBuffer],
        a: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        for item in &self.children {
            item.child.overlay_text_areas(fs, b, a, bi);
        }
    }

    fn overlay_hit_test(&self, point: Point) -> bool {
        self.children
            .iter()
            .rev()
            .any(|item| item.child.overlay_hit_test(point))
    }

    fn captures_input(&self) -> bool {
        self.children.iter().any(|item| item.child.captures_input())
    }

    fn requests_repaint(&self) -> bool {
        self.children
            .iter()
            .any(|item| item.child.requests_repaint())
    }

    fn repaint_interval(&self) -> Option<Duration> {
        self.children
            .iter()
            .filter_map(|item| item.child.repaint_interval())
            .min()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Flex {
    fn compute_start_gap(
        &self,
        align: MainAxisAlignment,
        main_size: f32,
        allocated: f32,
        count: usize,
    ) -> (f32, f32) {
        let free = (main_size - allocated - self.spacing * count.saturating_sub(1) as f32).max(0.0);
        match align {
            MainAxisAlignment::Start => (0.0, self.spacing),
            MainAxisAlignment::End => (free, self.spacing),
            MainAxisAlignment::Center => (free / 2.0, self.spacing),
            MainAxisAlignment::SpaceBetween => {
                if count > 1 {
                    (0.0, self.spacing + free / (count - 1) as f32)
                } else {
                    (0.0, 0.0)
                }
            }
            MainAxisAlignment::SpaceAround => {
                let gap = if count > 0 { free / count as f32 } else { 0.0 };
                (gap / 2.0, self.spacing + gap)
            }
            MainAxisAlignment::SpaceEvenly => {
                let gap = if count > 0 {
                    free / (count + 1) as f32
                } else {
                    0.0
                };
                (gap, self.spacing + gap)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::TextInput;

    #[test]
    fn flex_forwards_focused_text_input_repaint_interval() {
        let mut input = TextInput::new([0.0, 0.0], [160.0, 32.0], "Name");
        input.focus();
        let flex = Flex::column().with_child(Box::new(input));

        assert!(Widget::requests_repaint(&flex));
        assert_eq!(
            Widget::repaint_interval(&flex),
            Some(Duration::from_millis(500))
        );
    }

    struct RepaintProbe {
        interval: Option<Duration>,
    }

    impl Widget for RepaintProbe {
        fn update(&mut self, _dt: f32, _input: &InputManager) {}

        fn instances(&self) -> Vec<WidgetInstance> {
            Vec::new()
        }

        fn requests_repaint(&self) -> bool {
            self.interval.is_some()
        }

        fn repaint_interval(&self) -> Option<Duration> {
            self.interval
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn flex_uses_shortest_child_repaint_interval() {
        let flex = Flex::row()
            .with_child(Box::new(RepaintProbe {
                interval: Some(Duration::from_millis(40)),
            }))
            .with_child(Box::new(RepaintProbe {
                interval: Some(Duration::from_millis(16)),
            }));

        assert_eq!(
            Widget::repaint_interval(&flex),
            Some(Duration::from_millis(16))
        );
    }
}
