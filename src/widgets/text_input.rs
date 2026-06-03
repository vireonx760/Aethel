use crate::core::input::{InputManager, SpecialKey};
use crate::core::renderer::WidgetInstance;
use crate::gui::binding::TextSignal;
use crate::gui::command::{CommandId, UpdateCtx};
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::text::{set_buffer_size, set_buffer_text, shape_text, text_area};
use crate::gui::widget::Widget;
use glam::Vec2;
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, TextArea, TextBounds};
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use winit::event::MouseButton;

const FONT_SZ: f32 = 18.0;
const LINE_H: f32 = 22.0;
const PAD_X: f32 = 10.0;
const BLINK: f32 = 0.5;
const BLINK_PERIOD: Duration = Duration::from_millis(500);

pub type TextCallback = Arc<Mutex<dyn FnMut(&str) + Send + Sync>>;

pub struct TextInput {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    natural_size: [f32; 2],
    rect: Rect,
    clip_rect: Option<Rect>,

    pub text: String,
    pub placeholder: String,
    pub password: bool,
    pub max_length: Option<usize>,

    pub focused: bool,
    prev_down: bool,

    cursor_pos: usize,
    blink_timer: f32,
    cursor_visible: bool,
    scroll_offset: f32,

    char_x: Vec<f32>,

    signal: Option<TextSignal>,
    on_change: Option<TextCallback>,
    on_submit: Option<TextCallback>,
    on_change_cmd: Option<CommandId<String>>,
    on_submit_cmd: Option<CommandId<String>>,
}

impl TextInput {
    pub fn new(pos: [f32; 2], size: [f32; 2], placeholder: impl Into<String>) -> Self {
        Self {
            pos,
            size,
            natural_size: size,
            rect: Rect::new(pos[0], pos[1], size[0], size[1]),
            clip_rect: None,
            text: String::new(),
            placeholder: placeholder.into(),
            password: false,
            max_length: None,
            focused: false,
            prev_down: false,
            cursor_pos: 0,
            blink_timer: 0.0,
            cursor_visible: true,
            scroll_offset: 0.0,
            char_x: vec![0.0],
            signal: None,
            on_change: None,
            on_submit: None,
            on_change_cmd: None,
            on_submit_cmd: None,
        }
    }

    pub fn password(mut self, y: bool) -> Self {
        self.password = y;
        self
    }
    pub fn max_length(mut self, n: usize) -> Self {
        self.max_length = Some(n);
        self
    }

    pub fn initial_text(mut self, t: impl Into<String>) -> Self {
        self.text = t.into();
        self.cursor_pos = 0;
        self
    }

    pub fn on_change<F: FnMut(&str) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn on_submit<F: FnMut(&str) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_submit = Some(Arc::new(Mutex::new(f)));
        self
    }
    pub fn bind_signal(mut self, target: TextSignal) -> Self {
        self.text = target.get();
        self.cursor_pos = self.text.chars().count();
        self.signal = Some(target);
        self
    }
    pub fn on_change_cmd(mut self, command: CommandId<String>) -> Self {
        self.on_change_cmd = Some(command);
        self
    }
    pub fn on_submit_cmd(mut self, command: CommandId<String>) -> Self {
        self.on_submit_cmd = Some(command);
        self
    }

    pub fn set_text(&mut self, t: impl Into<String>) {
        self.text = t.into();
        self.cursor_pos = 0;
        self.char_x = vec![0.0];
        self.scroll_offset = 0.0;
        if let Some(signal) = &self.signal {
            signal.set(&self.text);
        }
    }
    pub fn get_text(&self) -> &str {
        &self.text
    }
    pub fn focus(&mut self) {
        self.focused = true;
        self.reset_blink();
    }
    pub fn blur(&mut self) {
        self.focused = false;
    }

    fn reset_blink(&mut self) {
        self.cursor_visible = true;
        self.blink_timer = 0.0;
    }

    fn advance_blink(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        self.blink_timer += dt;
        let periods = (self.blink_timer / BLINK).floor() as u32;
        if periods == 0 {
            return;
        }

        self.blink_timer -= BLINK * periods as f32;
        if !periods.is_multiple_of(2) {
            self.cursor_visible = !self.cursor_visible;
        }
    }

    fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    fn byte_of(&self, ci: usize) -> usize {
        self.text
            .char_indices()
            .nth(ci)
            .map(|(b, _)| b)
            .unwrap_or(self.text.len())
    }

    fn cursor_x_raw(&self) -> f32 {
        self.char_x
            .get(self.cursor_pos)
            .copied()
            .unwrap_or_else(|| self.char_x.last().copied().unwrap_or(0.0))
    }

    fn text_w(&self) -> f32 {
        self.char_x.last().copied().unwrap_or(0.0)
    }

    fn clamp_scroll(&mut self) {
        let vis = (self.size[0] - PAD_X * 2.0).max(0.0);
        let cx = self.cursor_x_raw();
        if cx - self.scroll_offset > vis - 6.0 {
            self.scroll_offset = cx - vis + 6.0;
        }
        if cx - self.scroll_offset < 0.0 {
            self.scroll_offset = (cx - 6.0).max(0.0);
        }
        let max_s = (self.text_w() - vis).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_s);
    }

    fn insert_ch(&mut self, ch: char) {
        if let Some(m) = self.max_length
            && self.char_count() >= m
        {
            return;
        }
        let b = self.byte_of(self.cursor_pos);
        self.text.insert(b, ch);
        self.cursor_pos += 1;
        self.clamp_scroll();
        self.fire_change();
    }

    fn del_before(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let e = self.byte_of(self.cursor_pos);
        let s = self.byte_of(self.cursor_pos - 1);
        self.text.drain(s..e);
        self.cursor_pos -= 1;
        self.clamp_scroll();
        self.fire_change();
    }

    fn del_after(&mut self) {
        let n = self.char_count();
        if self.cursor_pos >= n {
            return;
        }
        let s = self.byte_of(self.cursor_pos);
        let e = self.byte_of(self.cursor_pos + 1);
        self.text.drain(s..e);
        self.clamp_scroll();
        self.fire_change();
    }

    fn del_word_before(&mut self) {
        while self.cursor_pos > 0 {
            let prev = self.text.chars().nth(self.cursor_pos - 1);
            self.del_before();
            if prev.map(|c| c == ' ').unwrap_or(false) {
                break;
            }
        }
    }

    fn move_word_left(&mut self) {
        while self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            if self
                .text
                .chars()
                .nth(self.cursor_pos)
                .map(|c| c == ' ')
                .unwrap_or(false)
            {
                break;
            }
        }
    }

    fn move_word_right(&mut self) {
        let n = self.char_count();
        while self.cursor_pos < n {
            self.cursor_pos += 1;
            if self.cursor_pos < n
                && self
                    .text
                    .chars()
                    .nth(self.cursor_pos)
                    .map(|c| c == ' ')
                    .unwrap_or(false)
            {
                break;
            }
        }
    }

    fn x_to_char_idx(&self, rel_x: f32) -> usize {
        if self.char_x.len() <= 1 {
            return 0;
        }
        let n = self.char_x.len() - 1;
        for i in 0..n {
            let mid = (self.char_x[i] + self.char_x[i + 1]) / 2.0;
            if rel_x < mid {
                return i;
            }
        }
        n
    }

    fn fire_change(&self) {
        if let Some(signal) = &self.signal {
            signal.set(&self.text);
        }
        if let Some(cb) = &self.on_change
            && let Ok(mut f) = cb.lock()
        {
            f(&self.text);
        }
    }
    fn fire_submit(&self) {
        if let Some(cb) = &self.on_submit
            && let Ok(mut f) = cb.lock()
        {
            f(&self.text);
        }
    }

    fn display_text(&self) -> Option<String> {
        if !self.text.is_empty() {
            if self.password {
                Some("*".repeat(self.char_count()))
            } else {
                Some(self.text.clone())
            }
        } else if !self.focused {
            Some(self.placeholder.clone())
        } else {
            None
        }
    }

    fn mouse_hit(&self, m: Vec2) -> bool {
        let mn = Vec2::from_array(self.pos);
        let mx = mn + Vec2::from_array(self.size);
        if !m.cmpge(mn).all() || !m.cmplt(mx).all() {
            return false;
        }
        if let Some(c) = self.clip_rect {
            let cm = Vec2::new(c.x, c.y);
            let cx = Vec2::new(c.x + c.width, c.y + c.height);
            if !m.cmpge(cm).all() || !m.cmplt(cx).all() {
                return false;
            }
        }
        true
    }
}

impl Widget for TextInput {
    fn update(&mut self, dt: f32, input: &InputManager) {
        if !self.focused
            && let Some(signal) = &self.signal
        {
            let value = signal.get();
            if value != self.text {
                self.text = value;
                self.cursor_pos = self.cursor_pos.min(self.char_count());
            }
        }

        let down = input.is_mouse_down(MouseButton::Left);
        let click = down && !self.prev_down;
        self.prev_down = down;

        if click {
            let hit = self.mouse_hit(input.mouse_pos);
            if hit {
                if !self.focused {
                    self.focused = true;
                    let rel = input.mouse_pos.x - self.pos[0] - PAD_X + self.scroll_offset;
                    self.cursor_pos = self.x_to_char_idx(rel);
                }
                self.reset_blink();
                let rel = input.mouse_pos.x - self.pos[0] - PAD_X + self.scroll_offset;
                self.cursor_pos = self.x_to_char_idx(rel);
                self.clamp_scroll();
            } else {
                self.focused = false;
            }
        }

        if !self.focused {
            return;
        }

        self.advance_blink(dt);

        let had_chars = !input.chars_this_frame.is_empty();
        for &ch in &input.chars_this_frame {
            self.insert_ch(ch);
        }
        if had_chars {
            self.reset_blink();
        }

        if input.is_key_pressed(SpecialKey::CtrlA) {
            self.cursor_pos = self.char_count();
            self.clamp_scroll();
            self.reset_blink();
        }

        let mut nav = false;

        if input.is_key_pressed(SpecialKey::Backspace) {
            if input.ctrl {
                self.del_word_before();
            } else {
                self.del_before();
            }
            nav = true;
        }
        if input.is_key_pressed(SpecialKey::Delete) {
            self.del_after();
            nav = true;
        }
        if input.is_key_pressed(SpecialKey::ArrowLeft) {
            if input.ctrl {
                self.move_word_left();
            } else if self.cursor_pos > 0 {
                self.cursor_pos -= 1;
            }
            nav = true;
        }
        if input.is_key_pressed(SpecialKey::ArrowRight) {
            if input.ctrl {
                self.move_word_right();
            } else {
                let n = self.char_count();
                if self.cursor_pos < n {
                    self.cursor_pos += 1;
                }
            }
            nav = true;
        }
        if input.is_key_pressed(SpecialKey::Home) {
            self.cursor_pos = 0;
            nav = true;
        }
        if input.is_key_pressed(SpecialKey::End) {
            self.cursor_pos = self.char_count();
            nav = true;
        }
        if input.is_key_pressed(SpecialKey::Return) {
            self.fire_submit();
        }
        if input.is_key_pressed(SpecialKey::Escape) {
            self.focused = false;
        }

        if nav {
            self.clamp_scroll();
            self.reset_blink();
        }
    }

    fn update_ctx(&mut self, dt: f32, input: &InputManager, ctx: &mut UpdateCtx) {
        let before = self.text.clone();
        let submit = self.focused && input.is_key_pressed(SpecialKey::Return);
        self.update(dt, input);
        if before != self.text
            && let Some(command) = self.on_change_cmd
        {
            ctx.emit_text(command, self.text.clone());
        }
        if submit && let Some(command) = self.on_submit_cmd {
            ctx.emit_text(command, self.text.clone());
        }
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let (cmin, cmax, uc) = clip_info(self.clip_rect);

        let bg = if self.focused {
            [0.22, 0.22, 0.28, 1.0]
        } else {
            [0.17, 0.17, 0.21, 1.0]
        };

        let mut out = vec![WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: bg,
            radius: 6.0,
            clip_min: cmin,
            clip_max: cmax,
            use_clip: uc,
            ..Default::default()
        }];

        if self.focused {
            let [px, py] = self.pos;
            let [pw, ph] = self.size;
            let bc = [0.2, 0.55, 1.0, 0.85];
            let t = 1.5f32;
            for (p, s) in [
                ([px, py], [pw, t]),
                ([px, py + ph - t], [pw, t]),
                ([px, py], [t, ph]),
                ([px + pw - t, py], [t, ph]),
            ] {
                out.push(WidgetInstance {
                    pos: p,
                    size: s,
                    color: bc,
                    radius: 0.0,
                    clip_min: cmin,
                    clip_max: cmax,
                    use_clip: uc,
                    ..Default::default()
                });
            }
        }

        if self.focused && self.cursor_visible {
            let cx = (self.pos[0] + PAD_X + self.cursor_x_raw() - self.scroll_offset)
                .clamp(self.pos[0] + PAD_X, self.pos[0] + self.size[0] - PAD_X);
            out.push(WidgetInstance {
                pos: [cx, self.pos[1] + 5.0],
                size: [1.5, self.size[1] - 10.0],
                color: [0.35, 0.75, 1.0, 1.0],
                radius: 1.0,
                clip_min: cmin,
                clip_max: cmax,
                use_clip: uc,
                ..Default::default()
            });
        }
        out
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let (cmin, cmax, uc) = clip_info(self.clip_rect);
        let bg = if self.focused {
            [0.22, 0.22, 0.28, 1.0]
        } else {
            [0.17, 0.17, 0.21, 1.0]
        };

        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: bg,
            radius: 6.0,
            clip_min: cmin,
            clip_max: cmax,
            use_clip: uc,
            ..Default::default()
        });

        if self.focused {
            let [px, py] = self.pos;
            let [pw, ph] = self.size;
            let bc = [0.2, 0.55, 1.0, 0.85];
            let t = 1.5f32;
            for (p, s) in [
                ([px, py], [pw, t]),
                ([px, py + ph - t], [pw, t]),
                ([px, py], [t, ph]),
                ([px + pw - t, py], [t, ph]),
            ] {
                ctx.push_instance(WidgetInstance {
                    pos: p,
                    size: s,
                    color: bc,
                    radius: 0.0,
                    clip_min: cmin,
                    clip_max: cmax,
                    use_clip: uc,
                    ..Default::default()
                });
            }
        }

        if self.focused && self.cursor_visible {
            let cx = (self.pos[0] + PAD_X + self.cursor_x_raw() - self.scroll_offset)
                .clamp(self.pos[0] + PAD_X, self.pos[0] + self.size[0] - PAD_X);
            ctx.push_instance(WidgetInstance {
                pos: [cx, self.pos[1] + 5.0],
                size: [1.5, self.size[1] - 10.0],
                color: [0.35, 0.75, 1.0, 1.0],
                radius: 1.0,
                clip_min: cmin,
                clip_max: cmax,
                use_clip: uc,
                ..Default::default()
            });
        }
    }

    fn prepare_text_buffers(&mut self, fs: &mut FontSystem, bufs: &mut Vec<Buffer>) {
        let display_opt = self.display_text();
        let is_ph = self.text.is_empty() && !self.focused;

        if display_opt.is_none() {
            self.char_x = vec![0.0];
            bufs.push(Buffer::new(fs, Metrics::new(FONT_SZ, LINE_H)));
            return;
        }
        let Some(display) = display_opt else {
            return;
        };
        let col = if is_ph {
            Color::rgba(95, 95, 110, 170)
        } else {
            Color::rgba(225, 225, 238, 255)
        };

        let mut buf = Buffer::new(fs, Metrics::new(FONT_SZ, LINE_H));
        set_buffer_size(&mut buf, fs, f32::INFINITY, LINE_H + 4.0);
        set_buffer_text(
            &mut buf,
            fs,
            &display,
            Attrs::new().family(Family::SansSerif).color(col),
        );
        shape_text(&mut buf, fs);

        if !is_ph {
            let n = self.char_count();
            let mut pos = Vec::with_capacity(n + 1);
            pos.push(0.0f32);
            for run in buf.layout_runs() {
                for g in run.glyphs.iter() {
                    pos.push(g.x + g.w);
                }
            }
            while pos.len() <= n {
                let l = pos.last().copied().unwrap_or(0.0);
                pos.push(l + FONT_SZ * 0.55);
            }
            pos.truncate(n + 1);
            self.char_x = pos;
        } else {
            self.char_x = vec![0.0];
        }

        self.clamp_scroll();
        bufs.push(buf);
    }

    fn prepare_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        bufs: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if let Some(buf) = bufs.get(*bi) {
            let left = self.pos[0] + PAD_X - self.scroll_offset;
            let top = self.pos[1] + (self.size[1] - LINE_H) / 2.0;
            let fl = (self.pos[0] + PAD_X) as i32;
            let fr = (self.pos[0] + self.size[0] - PAD_X) as i32;
            let ft = top as i32;
            let fb = (top + LINE_H) as i32;
            let (tl, tt, tr, tb) = if let Some(c) = self.clip_rect {
                (
                    fl.max(c.x as i32),
                    ft.max(c.y as i32),
                    fr.min((c.x + c.width) as i32),
                    fb.min((c.y + c.height) as i32),
                )
            } else {
                (fl, ft, fr, fb)
            };

            let is_ph = self.text.is_empty() && !self.focused;
            let col = if is_ph {
                Color::rgba(95, 95, 110, 170)
            } else {
                Color::rgba(225, 225, 238, 255)
            };

            areas.push(text_area(
                buf,
                left,
                top,
                TextBounds {
                    left: tl,
                    top: tt,
                    right: tr,
                    bottom: tb,
                },
                col,
            ));
            *bi += 1;
        }
    }

    fn layout(&mut self, c: BoxConstraints) -> Size {
        let s = c.constrain_max(Size::new(self.natural_size[0], self.natural_size[1]));
        self.size = [s.width, s.height];
        self.rect.width = s.width;
        self.rect.height = s.height;
        s
    }
    fn set_position(&mut self, p: Point) {
        self.pos = [p.x, p.y];
        self.rect.x = p.x;
        self.rect.y = p.y;
    }
    fn set_clip_rect(&mut self, c: Rect) {
        self.clip_rect = Some(c);
    }
    fn get_rect(&self) -> Rect {
        self.rect
    }
    fn requests_repaint(&self) -> bool {
        self.focused
    }

    fn repaint_interval(&self) -> Option<Duration> {
        self.focused.then_some(BLINK_PERIOD)
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
    if let Some(r) = c {
        ([r.x, r.y], [r.x + r.width, r.y + r.height], 1.0)
    } else {
        ([0.0; 2], [1e5; 2], 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_text_input_requests_blink_repaint_interval() {
        let mut input = TextInput::new([0.0, 0.0], [120.0, 32.0], "Search");

        assert_eq!(Widget::repaint_interval(&input), None);

        input.focus();

        assert!(Widget::requests_repaint(&input));
        assert_eq!(Widget::repaint_interval(&input), Some(BLINK_PERIOD));
    }

    #[test]
    fn cursor_blink_advances_after_idle_waits() {
        let mut input = TextInput::new([0.0, 0.0], [120.0, 32.0], "Search");
        input.focus();

        input.advance_blink(BLINK + 0.01);
        assert!(!input.cursor_visible);

        input.advance_blink(BLINK + 0.01);
        assert!(input.cursor_visible);
    }

    #[test]
    fn cursor_blink_preserves_phase_after_multiple_elapsed_periods() {
        let mut input = TextInput::new([0.0, 0.0], [120.0, 32.0], "Search");
        input.focus();

        input.advance_blink(BLINK * 3.0 + 0.01);

        assert!(!input.cursor_visible);
        assert!(input.blink_timer < BLINK);
    }
}
