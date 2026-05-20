use crate::core::input::{InputManager, SpecialKey};
use crate::core::renderer::WidgetInstance;
use crate::gui::geometry::{BoxConstraints, Point, Rect, Size};
use crate::gui::paint::PaintCtx;
use crate::gui::text::{set_buffer_size, set_buffer_text, shape_text, text_area};
use crate::gui::widget::Widget;
use glam::Vec2;
use glyphon::{Attrs, Buffer, Color as GColor, Family, FontSystem, Metrics, TextArea, TextBounds};
use std::any::Any;
use std::time::Duration;
use winit::event::MouseButton;

const POP_W: f32 = 440.0;
const POP_H: f32 = 335.0;
const SV_W: f32 = 250.0;
const SV_H: f32 = 210.0;
const SV_Y: f32 = 46.0;
const HUE_W: f32 = SV_W;
const HUE_Y: f32 = SV_Y + SV_H + 12.0;
const HUE_H: f32 = 18.0;
const PAD_X: f32 = 12.0;
const SIDE_X: f32 = PAD_X + SV_W + 20.0;
const SIDE_W: f32 = POP_W - SIDE_X - PAD_X;
const LABEL_W: f32 = 34.0;
const FIELD_X: f32 = SIDE_X + LABEL_W;
const FIELD_W: f32 = SIDE_W - LABEL_W;
const INPUT_H: f32 = 28.0;
const HEX_Y: f32 = SV_Y + 26.0;
const RGB_Y: f32 = HEX_Y + 58.0;
const RGB_GAP: f32 = 8.0;
const PREVIEW_Y: f32 = RGB_Y + (INPUT_H + RGB_GAP) * 3.0 + 16.0;
const PREVIEW_H: f32 = 36.0;
const FONT_SZ: f32 = 13.0;

#[derive(Default, Clone)]
struct MiniInput {
    text: String,
    cursor: usize,
    focused: bool,
}

impl MiniInput {
    fn new(s: &str) -> Self {
        Self {
            text: s.to_string(),
            cursor: s.chars().count(),
            focused: false,
        }
    }
    fn set(&mut self, s: &str) {
        self.text = s.to_string();
        self.cursor = s.chars().count();
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
    fn insert(&mut self, ch: char) {
        let b = self.byte_of(self.cursor);
        self.text.insert(b, ch);
        self.cursor += 1;
    }
    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let e = self.byte_of(self.cursor);
        let s = self.byte_of(self.cursor - 1);
        self.text.drain(s..e);
        self.cursor -= 1;
    }
    fn delete(&mut self) {
        let n = self.char_count();
        if self.cursor >= n {
            return;
        }
        let s = self.byte_of(self.cursor);
        let e = self.byte_of(self.cursor + 1);
        self.text.drain(s..e);
    }
    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }
    fn move_right(&mut self) {
        let n = self.char_count();
        if self.cursor < n {
            self.cursor += 1;
        }
    }
    fn home(&mut self) {
        self.cursor = 0;
    }
    fn end(&mut self) {
        self.cursor = self.char_count();
    }
}

pub struct ColorPicker {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 3],
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub open: bool,
    pub window_offset: Vec2,

    rect: Rect,
    clip_rect: Option<Rect>,

    dragging_sv: bool,
    dragging_hue: bool,
    dragging_window: bool,
    prev_down: bool,
    last_mouse: Vec2,

    hex_input: MiniInput,
    r_input: MiniInput,
    g_input: MiniInput,
    b_input: MiniInput,
    /// 0=none 1=hex 2=r 3=g 4=b
    focused_field: usize,

    text_gen: u64,
}

impl ColorPicker {
    pub fn new(pos: [f32; 2]) -> Self {
        let mut s = Self {
            pos,
            size: [60.0, 30.0],
            color: [1.0, 0.0, 0.0],
            hue: 0.0,
            saturation: 1.0,
            value: 1.0,
            open: false,
            window_offset: Vec2::new(0.0, 36.0),
            rect: Rect::new(pos[0], pos[1], 60.0, 30.0),
            clip_rect: None,
            dragging_sv: false,
            dragging_hue: false,
            dragging_window: false,
            prev_down: false,
            last_mouse: Vec2::ZERO,
            hex_input: MiniInput::new("ff0000"),
            r_input: MiniInput::new("255"),
            g_input: MiniInput::new("0"),
            b_input: MiniInput::new("0"),
            focused_field: 0,
            text_gen: 0,
        };
        s.sync_color();
        s
    }

    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
        let c = v * s;
        let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
        let m = v - c;
        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        [r + m, g + m, b + m]
    }

    fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let cmax = r.max(g).max(b);
        let cmin = r.min(g).min(b);
        let delta = cmax - cmin;
        let v = cmax;
        let s = if cmax > 1e-6 { delta / cmax } else { 0.0 };
        let h = if delta < 1e-6 {
            0.0
        } else if cmax == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if cmax == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        let h = if h < 0.0 { h + 360.0 } else { h };
        (h, s, v)
    }

    fn sync_color(&mut self) {
        self.color = Self::hsv_to_rgb(self.hue, self.saturation, self.value);
        self.update_text_inputs();
    }

    fn sync_from_rgb(&mut self, r: f32, g: f32, b: f32) {
        let (h, s, v) = Self::rgb_to_hsv(r, g, b);
        self.hue = h;
        self.saturation = s;
        self.value = v;
        self.color = [r, g, b];
        self.update_text_inputs();
    }

    fn update_text_inputs(&mut self) {
        let [r, g, b] = self.color;
        let ri = (r * 255.0).round() as u8;
        let gi = (g * 255.0).round() as u8;
        let bi = (b * 255.0).round() as u8;
        if self.focused_field != 2 {
            self.r_input.set(&ri.to_string());
        }
        if self.focused_field != 3 {
            self.g_input.set(&gi.to_string());
        }
        if self.focused_field != 4 {
            self.b_input.set(&bi.to_string());
        }
        if self.focused_field != 1 {
            self.hex_input
                .set(&format!("{:02x}{:02x}{:02x}", ri, gi, bi));
        }
        self.text_gen += 1;
    }

    fn parse_inputs(&mut self) {
        match self.focused_field {
            1 => {
                // hex
                let h = self.hex_input.text.trim_start_matches('#');
                if h.len() == 6
                    && let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&h[0..2], 16),
                        u8::from_str_radix(&h[2..4], 16),
                        u8::from_str_radix(&h[4..6], 16),
                    )
                {
                    self.sync_from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
                }
            }
            2 => {
                // R
                if let Ok(v) = self.r_input.text.parse::<u8>() {
                    let [_, g, b] = self.color;
                    self.sync_from_rgb(v as f32 / 255.0, g, b);
                }
            }
            3 => {
                // G
                if let Ok(v) = self.g_input.text.parse::<u8>() {
                    let [r, _, b] = self.color;
                    self.sync_from_rgb(r, v as f32 / 255.0, b);
                }
            }
            4 => {
                // B
                if let Ok(v) = self.b_input.text.parse::<u8>() {
                    let [r, g, _] = self.color;
                    self.sync_from_rgb(r, g, v as f32 / 255.0);
                }
            }
            _ => {}
        }
    }

    fn focused_mini_mut(&mut self) -> Option<&mut MiniInput> {
        match self.focused_field {
            1 => Some(&mut self.hex_input),
            2 => Some(&mut self.r_input),
            3 => Some(&mut self.g_input),
            4 => Some(&mut self.b_input),
            _ => None,
        }
    }

    fn popup_pos(&self) -> Vec2 {
        Vec2::from_array(self.pos) + self.window_offset
    }

    fn popup_rect(&self) -> Rect {
        let pp = self.popup_pos();
        Rect::new(pp.x, pp.y, POP_W, POP_H)
    }

    fn button_rect(&self) -> Rect {
        Rect::new(self.pos[0], self.pos[1], self.size[0], self.size[1])
    }

    #[inline]
    fn point_in_rect(point: Vec2, rect: Rect) -> bool {
        point.x >= rect.x
            && point.y >= rect.y
            && point.x < rect.x + rect.width
            && point.y < rect.y + rect.height
    }

    fn hit_popup_or_button(&self, point: Vec2) -> bool {
        Self::point_in_rect(point, self.button_rect())
            || Self::point_in_rect(point, self.popup_rect())
    }

    fn sv_rect(&self) -> Rect {
        let pp = self.popup_pos();
        Rect::new(pp.x + PAD_X, pp.y + SV_Y, SV_W, SV_H)
    }

    fn hue_rect(&self) -> Rect {
        let pp = self.popup_pos();
        Rect::new(pp.x + PAD_X, pp.y + HUE_Y, HUE_W, HUE_H)
    }

    fn preview_rect(&self) -> Rect {
        let pp = self.popup_pos();
        Rect::new(pp.x + SIDE_X, pp.y + PREVIEW_Y, SIDE_W, PREVIEW_H)
    }

    fn input_label_pos(&self, field_idx: usize) -> Vec2 {
        let pp = self.popup_pos();
        let y = match field_idx {
            0 => HEX_Y,
            1 => RGB_Y,
            2 => RGB_Y + INPUT_H + RGB_GAP,
            _ => RGB_Y + (INPUT_H + RGB_GAP) * 2.0,
        };
        Vec2::new(pp.x + SIDE_X, pp.y + y + 7.0)
    }

    fn input_box_rect(&self, field_idx: usize) -> (Vec2, Vec2) {
        // field_idx: 0=hex 1=R 2=G 3=B
        let pp = self.popup_pos();
        let y = match field_idx {
            0 => HEX_Y,
            1 => RGB_Y,
            2 => RGB_Y + INPUT_H + RGB_GAP,
            _ => RGB_Y + (INPUT_H + RGB_GAP) * 2.0,
        };
        (
            Vec2::new(pp.x + FIELD_X, pp.y + y),
            Vec2::new(FIELD_W, INPUT_H),
        )
    }

    fn check_input_click(&mut self, mouse: Vec2) -> bool {
        for (fi, internal_idx) in [(0usize, 1usize), (1, 2), (2, 3), (3, 4)] {
            let (pos, sz) = self.input_box_rect(fi);
            if mouse.cmpge(pos).all() && mouse.cmplt(pos + sz).all() {
                self.parse_inputs();
                self.focused_field = internal_idx;
                if let Some(f) = self.focused_mini_mut() {
                    f.focused = true;
                    f.end();
                }
                return true;
            }
        }
        false
    }

    fn defocus_all(&mut self) {
        self.parse_inputs();
        self.focused_field = 0;
        self.hex_input.focused = false;
        self.r_input.focused = false;
        self.g_input.focused = false;
        self.b_input.focused = false;
    }
}

impl Widget for ColorPicker {
    fn update(&mut self, _dt: f32, input: &InputManager) {
        let mouse = input.mouse_pos;
        let down = input.is_mouse_down(MouseButton::Left);
        let click = down && !self.prev_down;

        if click {
            let bmin = Vec2::from_array(self.pos);
            let bmax = bmin + Vec2::from_array(self.size);
            if mouse.cmpge(bmin).all() && mouse.cmplt(bmax).all() {
                self.open = !self.open;
                if !self.open {
                    self.defocus_all();
                }
                self.prev_down = down;
                self.last_mouse = mouse;
                return;
            }
        }

        if !self.open {
            self.prev_down = down;
            self.last_mouse = mouse;
            return;
        }

        if click && !self.hit_popup_or_button(mouse) {
            self.open = false;
            self.dragging_sv = false;
            self.dragging_hue = false;
            self.dragging_window = false;
            self.defocus_all();
            self.prev_down = down;
            self.last_mouse = mouse;
            return;
        }

        let pp = self.popup_pos();

        let sv_rect = self.sv_rect();
        let sv_min = Vec2::new(sv_rect.x, sv_rect.y);
        let sv_max = sv_min + Vec2::new(sv_rect.width, sv_rect.height);
        let in_sv = mouse.cmpge(sv_min).all() && mouse.cmplt(sv_max).all();

        let hue_rect = self.hue_rect();
        let hue_min = Vec2::new(hue_rect.x, hue_rect.y);
        let hue_max = hue_min + Vec2::new(hue_rect.width, hue_rect.height);
        let in_hue = mouse.cmpge(hue_min).all() && mouse.cmplt(hue_max).all();

        let hdr_max = pp + Vec2::new(POP_W, 30.0);
        let in_hdr = mouse.cmpge(pp).all() && mouse.cmplt(hdr_max).all();

        if click {
            self.parse_inputs();

            if in_sv {
                self.dragging_sv = true;
            } else if in_hue {
                self.dragging_hue = true;
            } else if in_hdr {
                self.dragging_window = true;
            } else if !self.check_input_click(mouse) {
                self.defocus_all();
            }
        }

        if self.dragging_sv {
            if down {
                self.saturation = ((mouse.x - sv_min.x) / sv_rect.width).clamp(0.0, 1.0);
                self.value = 1.0 - ((mouse.y - sv_min.y) / sv_rect.height).clamp(0.0, 1.0);
                self.sync_color();
            } else {
                self.dragging_sv = false;
            }
        }

        if self.dragging_hue {
            if down {
                self.hue = ((mouse.x - hue_min.x) / hue_rect.width).clamp(0.0, 1.0) * 360.0;
                self.sync_color();
            } else {
                self.dragging_hue = false;
            }
        }

        if self.dragging_window {
            if down {
                self.window_offset += mouse - self.last_mouse;
            } else {
                self.dragging_window = false;
            }
        }

        if self.focused_field != 0 {
            if let Some(fi) = self.focused_mini_mut() {
                for &ch in &input.chars_this_frame {
                    fi.insert(ch);
                }
            }
            if input.is_key_pressed(SpecialKey::Backspace)
                && let Some(fi) = self.focused_mini_mut()
            {
                fi.backspace();
            }
            if input.is_key_pressed(SpecialKey::Delete)
                && let Some(fi) = self.focused_mini_mut()
            {
                fi.delete();
            }
            if input.is_key_pressed(SpecialKey::ArrowLeft)
                && let Some(fi) = self.focused_mini_mut()
            {
                fi.move_left();
            }
            if input.is_key_pressed(SpecialKey::ArrowRight)
                && let Some(fi) = self.focused_mini_mut()
            {
                fi.move_right();
            }
            if input.is_key_pressed(SpecialKey::Home)
                && let Some(fi) = self.focused_mini_mut()
            {
                fi.home();
            }
            if input.is_key_pressed(SpecialKey::End)
                && let Some(fi) = self.focused_mini_mut()
            {
                fi.end();
            }
            if input.is_key_pressed(SpecialKey::Return) || input.is_key_pressed(SpecialKey::Tab) {
                self.parse_inputs();
                if input.is_key_pressed(SpecialKey::Tab) {
                    let next = match self.focused_field {
                        1 => 2,
                        2 => 3,
                        3 => 4,
                        _ => 1,
                    };
                    self.focused_field = next;
                    if let Some(fi) = self.focused_mini_mut() {
                        fi.focused = true;
                        fi.end();
                    }
                }
            }
            if input.is_key_pressed(SpecialKey::Escape) {
                self.defocus_all();
            }
            self.text_gen += 1;
        }

        self.last_mouse = mouse;
        self.prev_down = down;
    }

    fn instances(&self) -> Vec<WidgetInstance> {
        let (cmin, cmax, uc) = if let Some(c) = self.clip_rect {
            ([c.x, c.y], [c.x + c.width, c.y + c.height], 1.0)
        } else {
            ([0.0; 2], [1e5; 2], 0.0)
        };
        vec![WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: [self.color[0], self.color[1], self.color[2], 1.0],
            radius: 4.0,
            clip_min: cmin,
            clip_max: cmax,
            use_clip: uc,
            ..Default::default()
        }]
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let (cmin, cmax, uc) = if let Some(c) = self.clip_rect {
            ([c.x, c.y], [c.x + c.width, c.y + c.height], 1.0)
        } else {
            ([0.0; 2], [1e5; 2], 0.0)
        };
        ctx.push_instance(WidgetInstance {
            pos: self.pos,
            size: self.size,
            color: [self.color[0], self.color[1], self.color[2], 1.0],
            radius: 4.0,
            clip_min: cmin,
            clip_max: cmax,
            use_clip: uc,
            ..Default::default()
        });
    }

    fn overlay_instances(&self) -> Vec<WidgetInstance> {
        if !self.open {
            return vec![];
        }
        let pp = self.popup_pos();
        let no_clip = ([0.0; 2], [1e5; 2], 0.0f32);
        let mut out = vec![];

        out.push(WidgetInstance {
            pos: pp.to_array(),
            size: [POP_W, POP_H],
            color: [0.09, 0.09, 0.11, 0.97],
            radius: 10.0,
            clip_min: no_clip.0,
            clip_max: no_clip.1,
            use_clip: 0.0,
            ..Default::default()
        });

        out.push(WidgetInstance {
            pos: pp.to_array(),
            size: [POP_W, 30.0],
            color: [0.18, 0.18, 0.22, 1.0],
            radius: 10.0,
            clip_min: no_clip.0,
            clip_max: no_clip.1,
            use_clip: 0.0,
            ..Default::default()
        });

        let hue_norm = self.hue / 360.0;
        let sv = self.sv_rect();
        out.push(WidgetInstance {
            pos: [sv.x, sv.y],
            size: [sv.width, sv.height],
            color: [hue_norm, 0.0, 0.0, 1.0],
            mode: 1.0,
            radius: 3.0,
            clip_min: no_clip.0,
            clip_max: no_clip.1,
            use_clip: 0.0,
            ..Default::default()
        });
        push_border(&mut out, sv, [1.0, 1.0, 1.0, 0.35], 1.0);

        let cx = sv.x + self.saturation * sv.width;
        let cy = sv.y + (1.0 - self.value) * sv.height;
        out.push(WidgetInstance {
            pos: [cx - 7.0, cy - 7.0],
            size: [14.0, 14.0],
            color: [0.0, 0.0, 0.0, 0.7],
            radius: 7.0,
            use_clip: 0.0,
            ..Default::default()
        });
        out.push(WidgetInstance {
            pos: [cx - 5.0, cy - 5.0],
            size: [10.0, 10.0],
            color: [1.0, 1.0, 1.0, 1.0],
            radius: 5.0,
            use_clip: 0.0,
            ..Default::default()
        });
        out.push(WidgetInstance {
            pos: [cx - 3.0, cy - 3.0],
            size: [6.0, 6.0],
            color: [self.color[0], self.color[1], self.color[2], 1.0],
            radius: 3.0,
            use_clip: 0.0,
            ..Default::default()
        });

        let hue = self.hue_rect();
        out.push(WidgetInstance {
            pos: [hue.x, hue.y],
            size: [hue.width, hue.height],
            color: [1.0, 1.0, 1.0, 1.0],
            mode: 2.0,
            radius: 4.0,
            clip_min: no_clip.0,
            clip_max: no_clip.1,
            use_clip: 0.0,
            ..Default::default()
        });
        push_border(&mut out, hue, [1.0, 1.0, 1.0, 0.35], 1.0);

        let hx = hue.x + (self.hue / 360.0) * hue.width;
        out.push(WidgetInstance {
            pos: [hx - 4.0, hue.y - 4.0],
            size: [8.0, hue.height + 8.0],
            color: [0.0, 0.0, 0.0, 0.75],
            radius: 3.0,
            use_clip: 0.0,
            ..Default::default()
        });
        out.push(WidgetInstance {
            pos: [hx - 2.5, hue.y - 3.0],
            size: [5.0, hue.height + 6.0],
            color: [1.0, 1.0, 1.0, 1.0],
            radius: 2.0,
            use_clip: 0.0,
            ..Default::default()
        });

        out.push(WidgetInstance {
            pos: [pp.x + SIDE_X - 10.0, pp.y + SV_Y],
            size: [1.0, PREVIEW_Y + PREVIEW_H - SV_Y],
            color: [0.3, 0.3, 0.35, 0.5],
            radius: 0.0,
            use_clip: 0.0,
            ..Default::default()
        });

        let field_pairs = [
            (0usize, 1usize, "HEX"),
            (1, 2, "R"),
            (2, 3, "G"),
            (3, 4, "B"),
        ];
        for (fi, fid, _label) in field_pairs {
            let (pos, sz) = self.input_box_rect(fi);
            let focused = self.focused_field == fid;
            let bg = if focused {
                [0.22, 0.22, 0.28, 1.0]
            } else {
                [0.15, 0.15, 0.18, 0.9]
            };
            out.push(WidgetInstance {
                pos: pos.to_array(),
                size: sz.to_array(),
                color: bg,
                radius: 4.0,
                use_clip: 0.0,
                ..Default::default()
            });
            if focused {
                let t = 1.5f32;
                let bc = [0.2, 0.55, 1.0, 0.8];
                for (p, s) in [
                    ([pos.x, pos.y], [sz.x, t]),
                    ([pos.x, pos.y + sz.y - t], [sz.x, t]),
                    ([pos.x, pos.y], [t, sz.y]),
                    ([pos.x + sz.x - t, pos.y], [t, sz.y]),
                ] {
                    out.push(WidgetInstance {
                        pos: p,
                        size: s,
                        color: bc,
                        radius: 0.0,
                        use_clip: 0.0,
                        ..Default::default()
                    });
                }
            }
        }

        let preview = self.preview_rect();
        out.push(WidgetInstance {
            pos: [preview.x, preview.y],
            size: [preview.width, preview.height],
            color: [self.color[0], self.color[1], self.color[2], 1.0],
            radius: 4.0,
            use_clip: 0.0,
            ..Default::default()
        });

        out
    }

    fn paint_overlay(&self, ctx: &mut PaintCtx) {
        if !self.open {
            return;
        }

        let pp = self.popup_pos();
        let hue_norm = self.hue / 360.0;
        let sv = self.sv_rect();

        for instance in [
            WidgetInstance {
                pos: pp.to_array(),
                size: [POP_W, POP_H],
                color: [0.09, 0.09, 0.11, 0.97],
                radius: 10.0,
                use_clip: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: pp.to_array(),
                size: [POP_W, 30.0],
                color: [0.18, 0.18, 0.22, 1.0],
                radius: 10.0,
                use_clip: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [sv.x, sv.y],
                size: [sv.width, sv.height],
                color: [hue_norm, 0.0, 0.0, 1.0],
                mode: 1.0,
                radius: 3.0,
                use_clip: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [self.hue_rect().x, self.hue_rect().y],
                size: [self.hue_rect().width, self.hue_rect().height],
                color: [1.0, 1.0, 1.0, 1.0],
                mode: 2.0,
                radius: 4.0,
                use_clip: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [pp.x + SIDE_X - 10.0, pp.y + SV_Y],
                size: [1.0, PREVIEW_Y + PREVIEW_H - SV_Y],
                color: [0.3, 0.3, 0.35, 0.5],
                radius: 0.0,
                use_clip: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [self.preview_rect().x, self.preview_rect().y],
                size: [self.preview_rect().width, self.preview_rect().height],
                color: [self.color[0], self.color[1], self.color[2], 1.0],
                radius: 4.0,
                use_clip: 0.0,
                ..Default::default()
            },
        ] {
            ctx.push_instance(instance);
        }
        paint_border(ctx, sv, [1.0, 1.0, 1.0, 0.35], 1.0);
        paint_border(ctx, self.hue_rect(), [1.0, 1.0, 1.0, 0.35], 1.0);

        let cx = sv.x + self.saturation * sv.width;
        let cy = sv.y + (1.0 - self.value) * sv.height;
        for instance in [
            WidgetInstance {
                pos: [cx - 7.0, cy - 7.0],
                size: [14.0, 14.0],
                color: [0.0, 0.0, 0.0, 0.7],
                radius: 7.0,
                use_clip: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [cx - 5.0, cy - 5.0],
                size: [10.0, 10.0],
                color: [1.0, 1.0, 1.0, 1.0],
                radius: 5.0,
                use_clip: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [cx - 3.0, cy - 3.0],
                size: [6.0, 6.0],
                color: [self.color[0], self.color[1], self.color[2], 1.0],
                radius: 3.0,
                use_clip: 0.0,
                ..Default::default()
            },
        ] {
            ctx.push_instance(instance);
        }

        let hue = self.hue_rect();
        let hx = hue.x + (self.hue / 360.0) * hue.width;
        for instance in [
            WidgetInstance {
                pos: [hx - 4.0, hue.y - 4.0],
                size: [8.0, hue.height + 8.0],
                color: [0.0, 0.0, 0.0, 0.75],
                radius: 3.0,
                use_clip: 0.0,
                ..Default::default()
            },
            WidgetInstance {
                pos: [hx - 2.5, hue.y - 3.0],
                size: [5.0, hue.height + 6.0],
                color: [1.0, 1.0, 1.0, 1.0],
                radius: 2.0,
                use_clip: 0.0,
                ..Default::default()
            },
        ] {
            ctx.push_instance(instance);
        }

        let field_pairs = [(0usize, 1usize), (1, 2), (2, 3), (3, 4)];
        for (field_index, field_id) in field_pairs {
            let (pos, size) = self.input_box_rect(field_index);
            let focused = self.focused_field == field_id;
            ctx.push_instance(WidgetInstance {
                pos: pos.to_array(),
                size: size.to_array(),
                color: if focused {
                    [0.22, 0.22, 0.28, 1.0]
                } else {
                    [0.15, 0.15, 0.18, 0.9]
                },
                radius: 4.0,
                use_clip: 0.0,
                ..Default::default()
            });

            if focused {
                let border = 1.5f32;
                let border_color = [0.2, 0.55, 1.0, 0.8];
                for (p, s) in [
                    ([pos.x, pos.y], [size.x, border]),
                    ([pos.x, pos.y + size.y - border], [size.x, border]),
                    ([pos.x, pos.y], [border, size.y]),
                    ([pos.x + size.x - border, pos.y], [border, size.y]),
                ] {
                    ctx.push_instance(WidgetInstance {
                        pos: p,
                        size: s,
                        color: border_color,
                        radius: 0.0,
                        use_clip: 0.0,
                        ..Default::default()
                    });
                }
            }
        }
    }

    fn overlay_text_buffers(&mut self, fs: &mut FontSystem, bufs: &mut Vec<Buffer>) {
        if !self.open {
            return;
        }

        let mk = |fs: &mut FontSystem, text: &str| -> Buffer {
            let mut b = Buffer::new(fs, Metrics::new(FONT_SZ, FONT_SZ + 3.0));
            set_buffer_size(&mut b, fs, 200.0, FONT_SZ + 4.0);
            set_buffer_text(
                &mut b,
                fs,
                text,
                Attrs::new()
                    .family(Family::Monospace)
                    .color(GColor::rgba(210, 210, 220, 255)),
            );
            shape_text(&mut b, fs);
            b
        };

        bufs.push(mk(fs, "HEX"));
        bufs.push(mk(fs, "R"));
        bufs.push(mk(fs, "G"));
        bufs.push(mk(fs, "B"));

        bufs.push(mk(fs, &format!("#{}", self.hex_input.text)));
        bufs.push(mk(fs, &self.r_input.text));
        bufs.push(mk(fs, &self.g_input.text));
        bufs.push(mk(fs, &self.b_input.text));

        bufs.push(mk(fs, "Color Picker"));
    }

    fn overlay_text_areas<'a>(
        &self,
        _fs: &mut FontSystem,
        bufs: &'a [Buffer],
        areas: &mut Vec<TextArea<'a>>,
        bi: &mut usize,
    ) {
        if !self.open {
            return;
        }
        let base_buf = *bi;

        let label_positions = [
            self.input_label_pos(0),
            self.input_label_pos(1),
            self.input_label_pos(2),
            self.input_label_pos(3),
        ];
        for (i, pos) in label_positions.iter().enumerate() {
            if let Some(b) = bufs.get(base_buf + i) {
                push_text_area(b, pos.x, pos.y, LABEL_W, areas);
                *bi += 1;
            }
        }

        for fi in 0usize..4 {
            if let Some(b) = bufs.get(base_buf + 4 + fi) {
                let (pos, sz) = self.input_box_rect(fi);
                push_text_area(
                    b,
                    pos.x + 4.0,
                    pos.y + (sz.y - FONT_SZ) / 2.0,
                    sz.x - 8.0,
                    areas,
                );
                *bi += 1;
            }
        }

        if let Some(b) = bufs.get(base_buf + 8) {
            let pp = self.popup_pos();
            push_text_area(b, pp.x + PAD_X, pp.y + 8.0, POP_W - PAD_X * 2.0, areas);
            *bi += 1;
        }
    }

    fn layout(&mut self, c: BoxConstraints) -> Size {
        let s = c.constrain_max(Size::new(self.size[0], self.size[1]));
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
        self.open || self.dragging_sv || self.dragging_hue || self.dragging_window
    }
    fn repaint_interval(&self) -> Option<Duration> {
        (self.dragging_sv || self.dragging_hue || self.dragging_window)
            .then_some(Duration::from_millis(16))
    }
    fn overlay_hit_test(&self, point: Point) -> bool {
        if !self.open {
            return false;
        }
        let rect = self.popup_rect();
        point.x >= rect.x
            && point.y >= rect.y
            && point.x < rect.x + rect.width
            && point.y < rect.y + rect.height
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

fn push_text_area<'a>(buf: &'a Buffer, x: f32, y: f32, max_w: f32, areas: &mut Vec<TextArea<'a>>) {
    areas.push(text_area(
        buf,
        x,
        y,
        TextBounds {
            left: x as i32,
            top: y as i32,
            right: (x + max_w) as i32,
            bottom: (y + FONT_SZ + 4.0) as i32,
        },
        GColor::rgba(210, 210, 220, 255),
    ));
}

fn push_border(out: &mut Vec<WidgetInstance>, rect: Rect, color: [f32; 4], thickness: f32) {
    for instance in border_instances(rect, color, thickness) {
        out.push(instance);
    }
}

fn paint_border(ctx: &mut PaintCtx, rect: Rect, color: [f32; 4], thickness: f32) {
    for instance in border_instances(rect, color, thickness) {
        ctx.push_instance(instance);
    }
}

fn border_instances(rect: Rect, color: [f32; 4], thickness: f32) -> [WidgetInstance; 4] {
    let t = thickness.max(1.0);
    [
        WidgetInstance {
            pos: [rect.x, rect.y],
            size: [rect.width, t],
            color,
            use_clip: 0.0,
            ..Default::default()
        },
        WidgetInstance {
            pos: [rect.x, rect.y + rect.height - t],
            size: [rect.width, t],
            color,
            use_clip: 0.0,
            ..Default::default()
        },
        WidgetInstance {
            pos: [rect.x, rect.y],
            size: [t, rect.height],
            color,
            use_clip: 0.0,
            ..Default::default()
        },
        WidgetInstance {
            pos: [rect.x + rect.width - t, rect.y],
            size: [t, rect.height],
            color,
            use_clip: 0.0,
            ..Default::default()
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::paint::{FramePaint, RenderLayer};

    #[test]
    fn popup_is_wide_and_fields_do_not_overlap() {
        let picker = ColorPicker::new([10.0, 20.0]);
        let popup = picker.popup_rect();
        assert!(popup.width > popup.height);

        let mut previous_bottom = 0.0f32;
        for field in 0..4 {
            let (pos, size) = picker.input_box_rect(field);
            assert!(pos.x >= picker.popup_pos().x + FIELD_X);
            assert!(size.x >= 80.0);
            assert!(pos.y >= previous_bottom);
            previous_bottom = pos.y + size.y;
        }
    }

    #[test]
    fn focused_rgb_field_does_not_block_hex_refresh() {
        let mut picker = ColorPicker::new([0.0, 0.0]);
        picker.focused_field = 2;
        picker.sync_from_rgb(0.0, 1.0, 0.0);

        assert_eq!(picker.hex_input.text, "00ff00");
        assert_eq!(picker.g_input.text, "255");
        assert_eq!(picker.b_input.text, "0");
    }

    #[test]
    fn overlay_instances_include_popup_palette_and_hue_bar() {
        let mut picker = ColorPicker::new([20.0, 30.0]);
        picker.open = true;

        let instances = picker.overlay_instances();

        assert!(
            instances
                .iter()
                .any(|instance| instance.size == [POP_W, POP_H])
        );
        assert!(instances.iter().any(|instance| instance.mode == 1.0));
        assert!(instances.iter().any(|instance| instance.mode == 2.0));
        assert!(instances.len() >= 16);
    }

    #[test]
    fn paint_overlay_emits_overlay_batches_for_window_chrome() {
        let mut picker = ColorPicker::new([20.0, 30.0]);
        picker.open = true;
        let mut frame = FramePaint::new();

        {
            let mut ctx = PaintCtx::new(&mut frame);
            ctx.set_layer(RenderLayer::Overlay);
            picker.paint_overlay(&mut ctx);
        }

        assert!(!frame.overlay_batches().is_empty());
        assert!(
            frame
                .instances()
                .iter()
                .any(|instance| instance.mode == 1.0)
        );
        assert!(
            frame
                .instances()
                .iter()
                .any(|instance| instance.mode == 2.0)
        );
    }
}
