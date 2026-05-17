use crate::core::input::InputManager;
use crate::gui::geometry::{Point, Rect};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusChange {
    None,
    Focused(FocusId),
    Blurred(FocusId),
    Swapped { old: FocusId, new: FocusId },
}

#[derive(Debug, Clone, Default)]
pub struct FocusManager {
    focused: Option<FocusId>,
    next: usize,
}

impl FocusManager {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn alloc(&mut self) -> FocusId {
        let id = FocusId(self.next);
        self.next += 1;
        id
    }

    #[inline]
    pub fn focused(&self) -> Option<FocusId> {
        self.focused
    }

    pub fn focus(&mut self, id: FocusId) -> FocusChange {
        match self.focused.replace(id) {
            Some(old) if old == id => FocusChange::None,
            Some(old) => FocusChange::Swapped { old, new: id },
            None => FocusChange::Focused(id),
        }
    }

    pub fn blur(&mut self, id: FocusId) -> FocusChange {
        if self.focused == Some(id) {
            self.focused = None;
            FocusChange::Blurred(id)
        } else {
            FocusChange::None
        }
    }

    pub fn clear(&mut self) -> FocusChange {
        if let Some(id) = self.focused.take() {
            FocusChange::Blurred(id)
        } else {
            FocusChange::None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestBehavior {
    Opaque,
    Transparent,
    Passthrough,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitTarget {
    pub id: usize,
    pub rect: Rect,
    pub z: i32,
    pub behavior: HitTestBehavior,
}

impl HitTarget {
    #[inline]
    pub fn new(id: usize, rect: Rect) -> Self {
        Self {
            id,
            rect,
            z: 0,
            behavior: HitTestBehavior::Opaque,
        }
    }

    #[inline]
    pub fn z(mut self, z: i32) -> Self {
        self.z = z;
        self
    }

    #[inline]
    pub fn behavior(mut self, behavior: HitTestBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    #[inline]
    pub fn contains(self, point: Point) -> bool {
        self.rect.contains(point)
    }
}

#[derive(Debug, Clone, Default)]
pub struct HitTestList {
    targets: Vec<HitTarget>,
}

impl HitTestList {
    #[inline]
    pub fn new() -> Self {
        Self {
            targets: Vec::with_capacity(64),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.targets.clear();
    }

    #[inline]
    pub fn push(&mut self, target: HitTarget) {
        self.targets.push(target);
    }

    pub fn sort_by_z(&mut self) {
        self.targets.sort_by_key(|target| target.z);
    }

    pub fn hit(&self, point: Point) -> Option<HitTarget> {
        self.targets.iter().rev().copied().find(|target| {
            target.behavior != HitTestBehavior::Passthrough && target.contains(point)
        })
    }

    pub fn hits(&self, point: Point, out: &mut Vec<HitTarget>) {
        out.clear();
        out.extend(
            self.targets
                .iter()
                .rev()
                .copied()
                .filter(|target| target.contains(point)),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragGesture {
    pub id: usize,
    pub start: Point,
    pub current: Point,
    pub delta: [f32; 2],
}

impl DragGesture {
    #[inline]
    pub fn new(id: usize, start: Point) -> Self {
        Self {
            id,
            start,
            current: start,
            delta: [0.0, 0.0],
        }
    }

    #[inline]
    pub fn update(&mut self, current: Point) {
        self.current = current;
        self.delta = [current.x - self.start.x, current.y - self.start.y];
    }
}

#[derive(Debug, Clone, Default)]
pub struct DragController {
    active: Option<DragGesture>,
}

impl DragController {
    #[inline]
    pub fn active(&self) -> Option<DragGesture> {
        self.active
    }

    pub fn begin(&mut self, id: usize, point: Point) {
        self.active = Some(DragGesture::new(id, point));
    }

    pub fn update(&mut self, point: Point) -> Option<DragGesture> {
        let gesture = self.active.as_mut()?;
        gesture.update(point);
        Some(*gesture)
    }

    pub fn end(&mut self) -> Option<DragGesture> {
        self.active.take()
    }

    pub fn cancel(&mut self) {
        self.active = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionEventKind {
    HoverEnter,
    HoverExit,
    Press,
    Release,
    Click,
    DragStart,
    DragMove,
    DragEnd,
    Focus,
    Blur,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteractionEvent {
    pub target: usize,
    pub kind: InteractionEventKind,
    pub point: Point,
}

#[derive(Debug, Clone)]
pub struct InteractionQueue {
    events: VecDeque<InteractionEvent>,
}

impl InteractionQueue {
    #[inline]
    pub fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(32),
        }
    }

    #[inline]
    pub fn push(&mut self, event: InteractionEvent) {
        self.events.push_back(event);
    }

    #[inline]
    pub fn pop(&mut self) -> Option<InteractionEvent> {
        self.events.pop_front()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.events.clear();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for InteractionQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetInteractionState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub dragging: bool,
}

impl WidgetInteractionState {
    pub const EMPTY: Self = Self {
        hovered: false,
        pressed: false,
        focused: false,
        dragging: false,
    };

    #[inline]
    pub fn from_input(rect: Rect, input: &InputManager) -> Self {
        let hovered = rect.contains(Point::new(input.mouse_pos.x, input.mouse_pos.y));
        Self {
            hovered,
            pressed: hovered && input.lmb.held,
            focused: false,
            dragging: input.drag.is_some(),
        }
    }

    pub fn diff(self, next: Self, target: usize, point: Point, out: &mut InteractionQueue) {
        if !self.hovered && next.hovered {
            out.push(InteractionEvent {
                target,
                kind: InteractionEventKind::HoverEnter,
                point,
            });
        }
        if self.hovered && !next.hovered {
            out.push(InteractionEvent {
                target,
                kind: InteractionEventKind::HoverExit,
                point,
            });
        }
        if !self.pressed && next.pressed {
            out.push(InteractionEvent {
                target,
                kind: InteractionEventKind::Press,
                point,
            });
        }
        if self.pressed && !next.pressed {
            out.push(InteractionEvent {
                target,
                kind: InteractionEventKind::Release,
                point,
            });
        }
        if !self.focused && next.focused {
            out.push(InteractionEvent {
                target,
                kind: InteractionEventKind::Focus,
                point,
            });
        }
        if self.focused && !next.focused {
            out.push(InteractionEvent {
                target,
                kind: InteractionEventKind::Blur,
                point,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_manager_reports_swap() {
        let mut focus = FocusManager::new();
        let a = focus.alloc();
        let b = focus.alloc();
        assert_eq!(focus.focus(a), FocusChange::Focused(a));
        assert_eq!(focus.focus(b), FocusChange::Swapped { old: a, new: b });
    }

    #[test]
    fn hit_test_returns_topmost_target() {
        let mut list = HitTestList::new();
        list.push(HitTarget::new(1, Rect::new(0.0, 0.0, 20.0, 20.0)).z(1));
        list.push(HitTarget::new(2, Rect::new(0.0, 0.0, 20.0, 20.0)).z(2));
        list.sort_by_z();
        assert_eq!(
            list.hit(Point::new(5.0, 5.0)).map(|target| target.id),
            Some(2)
        );
    }
}
