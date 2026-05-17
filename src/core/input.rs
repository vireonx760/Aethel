use glam::Vec2;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialKey {
    Backspace = 0,
    Delete = 1,
    ArrowLeft = 2,
    ArrowRight = 3,
    ArrowUp = 4,
    ArrowDown = 5,
    Home = 6,
    End = 7,
    Return = 8,
    Escape = 9,
    Tab = 10,
    CtrlA = 11,
    CtrlC = 12,
    CtrlV = 13,
    CtrlX = 14,
    CtrlZ = 15,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragState {
    pub widget_idx: usize,
    pub start_mouse: Vec2,
    pub start_widget: Vec2,
    pub delta: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MouseButtonState {
    pub held: bool,
    pub just_pressed: bool,
    pub just_released: bool,
}

impl MouseButtonState {
    #[inline]
    fn press(&mut self) {
        self.held = true;
        self.just_pressed = true;
    }

    #[inline]
    fn release(&mut self) {
        self.held = false;
        self.just_released = true;
    }

    #[inline]
    fn end_frame(&mut self) {
        self.just_pressed = false;
        self.just_released = false;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseButtons {
    bits: u8,
}

impl MouseButtons {
    #[inline]
    pub fn contains(self, button: MouseButton) -> bool {
        mouse_button_bit(button).is_some_and(|bit| self.bits & bit != 0)
    }

    #[inline]
    fn insert(&mut self, button: MouseButton) {
        if let Some(bit) = mouse_button_bit(button) {
            self.bits |= bit;
        }
    }

    #[inline]
    fn remove(&mut self, button: MouseButton) {
        if let Some(bit) = mouse_button_bit(button) {
            self.bits &= !bit;
        }
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.bits == 0
    }
}

pub struct InputManager {
    pub mouse_pos: Vec2,
    pub mouse_prev: Vec2,
    pub mouse_delta: Vec2,
    pub mouse_down: MouseButtons,
    pub scroll_delta: f32,
    pub lmb: MouseButtonState,
    pub rmb: MouseButtonState,
    pub drag: Option<DragState>,
    pub chars_this_frame: Vec<char>,
    pub special_keys_this_frame: u32,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            mouse_pos: Vec2::ZERO,
            mouse_prev: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            mouse_down: MouseButtons::default(),
            scroll_delta: 0.0,
            lmb: MouseButtonState::default(),
            rmb: MouseButtonState::default(),
            drag: None,
            chars_this_frame: Vec::with_capacity(16),
            special_keys_this_frame: 0,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    #[inline]
    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(button)
    }

    #[inline]
    pub fn is_key_pressed(&self, key: SpecialKey) -> bool {
        self.special_keys_this_frame & special_key_bit(key) != 0
    }

    #[inline]
    pub fn any_key_pressed(&self) -> bool {
        self.special_keys_this_frame != 0 || !self.chars_this_frame.is_empty()
    }

    #[inline]
    pub fn begin_drag(&mut self, widget_idx: usize, widget_pos: Vec2) {
        self.drag = Some(DragState {
            widget_idx,
            start_mouse: self.mouse_pos,
            start_widget: widget_pos,
            delta: Vec2::ZERO,
        });
    }

    #[inline]
    pub fn drag_target(&self, widget_idx: usize) -> Option<Vec2> {
        self.drag.and_then(|drag| {
            if drag.widget_idx == widget_idx {
                Some(drag.start_widget + drag.delta)
            } else {
                None
            }
        })
    }

    #[inline]
    pub fn is_dragging(&self, widget_idx: usize) -> bool {
        self.drag.is_some_and(|drag| drag.widget_idx == widget_idx)
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_prev = self.mouse_pos;
                self.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
                self.mouse_delta = self.mouse_pos - self.mouse_prev;

                if let Some(drag) = &mut self.drag {
                    drag.delta = self.mouse_pos - drag.start_mouse;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    self.mouse_down.insert(*button);
                    match button {
                        MouseButton::Left => self.lmb.press(),
                        MouseButton::Right => self.rmb.press(),
                        _ => {}
                    }
                }
                ElementState::Released => {
                    self.mouse_down.remove(*button);
                    match button {
                        MouseButton::Left => {
                            self.lmb.release();
                            self.drag = None;
                        }
                        MouseButton::Right => self.rmb.release(),
                        _ => {}
                    }
                }
            },
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                self.scroll_delta += y;
            }
            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                use winit::keyboard::ModifiersState;
                self.ctrl = state.contains(ModifiersState::CONTROL);
                self.shift = state.contains(ModifiersState::SHIFT);
                self.alt = state.contains(ModifiersState::ALT);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if !self.ctrl
                    && let Some(text) = &event.text
                {
                    for ch in text.chars().filter(|ch| !ch.is_control()) {
                        self.chars_this_frame.push(ch);
                    }
                }

                if self.ctrl
                    && let Key::Character(character) = &event.logical_key
                {
                    self.handle_ctrl_character(character);
                }

                self.handle_named_key(&event.logical_key);
            }
            _ => {}
        }
    }

    pub fn end_frame(&mut self) {
        self.chars_this_frame.clear();
        self.special_keys_this_frame = 0;
        self.scroll_delta = 0.0;
        self.mouse_delta = Vec2::ZERO;
        self.lmb.end_frame();
        self.rmb.end_frame();
    }

    #[inline]
    fn mark_key(&mut self, key: SpecialKey) {
        self.special_keys_this_frame |= special_key_bit(key);
    }

    fn handle_ctrl_character(&mut self, character: &str) {
        match character.to_ascii_lowercase().chars().next().unwrap_or(' ') {
            'a' => self.mark_key(SpecialKey::CtrlA),
            'c' => self.mark_key(SpecialKey::CtrlC),
            'v' => self.mark_key(SpecialKey::CtrlV),
            'x' => self.mark_key(SpecialKey::CtrlX),
            'z' => self.mark_key(SpecialKey::CtrlZ),
            _ => {}
        }
    }

    fn handle_named_key(&mut self, key: &Key) {
        match key {
            Key::Named(NamedKey::Backspace) => self.mark_key(SpecialKey::Backspace),
            Key::Named(NamedKey::Delete) => self.mark_key(SpecialKey::Delete),
            Key::Named(NamedKey::ArrowLeft) => self.mark_key(SpecialKey::ArrowLeft),
            Key::Named(NamedKey::ArrowRight) => self.mark_key(SpecialKey::ArrowRight),
            Key::Named(NamedKey::ArrowUp) => self.mark_key(SpecialKey::ArrowUp),
            Key::Named(NamedKey::ArrowDown) => self.mark_key(SpecialKey::ArrowDown),
            Key::Named(NamedKey::Home) => self.mark_key(SpecialKey::Home),
            Key::Named(NamedKey::End) => self.mark_key(SpecialKey::End),
            Key::Named(NamedKey::Enter) => self.mark_key(SpecialKey::Return),
            Key::Named(NamedKey::Escape) => self.mark_key(SpecialKey::Escape),
            Key::Named(NamedKey::Tab) => self.mark_key(SpecialKey::Tab),
            _ => {}
        }
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn special_key_bit(key: SpecialKey) -> u32 {
    1u32 << key as u8
}

#[inline]
fn mouse_button_bit(button: MouseButton) -> Option<u8> {
    match button {
        MouseButton::Left => Some(1 << 0),
        MouseButton::Right => Some(1 << 1),
        MouseButton::Middle => Some(1 << 2),
        MouseButton::Back => Some(1 << 3),
        MouseButton::Forward => Some(1 << 4),
        MouseButton::Other(idx) if idx < 3 => Some(1 << (5 + idx as u8)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_bit_tracks_pressed_key() {
        let mut input = InputManager::new();
        input.mark_key(SpecialKey::Backspace);
        assert!(input.is_key_pressed(SpecialKey::Backspace));
        input.end_frame();
        assert!(!input.is_key_pressed(SpecialKey::Backspace));
    }

    #[test]
    fn mouse_buttons_track_independent_buttons() {
        let mut buttons = MouseButtons::default();
        buttons.insert(MouseButton::Left);
        buttons.insert(MouseButton::Right);
        assert!(buttons.contains(MouseButton::Left));
        assert!(buttons.contains(MouseButton::Right));
        buttons.remove(MouseButton::Left);
        assert!(!buttons.contains(MouseButton::Left));
        assert!(buttons.contains(MouseButton::Right));
    }
}
