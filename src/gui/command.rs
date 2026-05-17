use crate::gui::binding::BindingId;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CommandId<T = ()> {
    raw: u64,
    _marker: PhantomData<fn(T)>,
}

impl<T> Clone for CommandId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for CommandId<T> {}

impl<T> CommandId<T> {
    #[inline]
    pub const fn from_raw(raw: u64) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn raw(self) -> u64 {
        self.raw
    }

    #[inline]
    pub fn erase(self) -> CommandId<()> {
        CommandId::from_raw(self.raw)
    }
}

#[derive(Debug, Default)]
pub struct CommandIds {
    next: AtomicU64,
}

impl CommandIds {
    #[inline]
    pub fn new() -> Self {
        Self {
            next: AtomicU64::new(1),
        }
    }

    #[inline]
    pub fn alloc<T>(&self) -> CommandId<T> {
        CommandId::from_raw(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub enum CommandPayload {
    None,
    Bool(bool),
    F32(f32),
    I32(i32),
    U32(u32),
    Index(usize),
    Text(String),
    BindingU64(u64),
}

impl CommandPayload {
    #[inline]
    pub fn as_bool(self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(value),
            _ => None,
        }
    }

    #[inline]
    pub fn as_f32(self) -> Option<f32> {
        match self {
            Self::F32(value) => Some(value),
            _ => None,
        }
    }

    #[inline]
    pub fn as_i32(self) -> Option<i32> {
        match self {
            Self::I32(value) => Some(value),
            _ => None,
        }
    }

    #[inline]
    pub fn as_u32(self) -> Option<u32> {
        match self {
            Self::U32(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiCommand {
    pub id: CommandId<()>,
    pub widget: Option<WidgetId>,
    pub payload: CommandPayload,
}

impl UiCommand {
    #[inline]
    pub fn new(id: CommandId<()>) -> Self {
        Self {
            id,
            widget: None,
            payload: CommandPayload::None,
        }
    }

    #[inline]
    pub fn widget(mut self, widget: WidgetId) -> Self {
        self.widget = Some(widget);
        self
    }

    #[inline]
    pub fn payload(mut self, payload: CommandPayload) -> Self {
        self.payload = payload;
        self
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommandStats {
    pub emitted: usize,
    pub drained: usize,
    pub capacity_growths: usize,
}

#[derive(Debug, Clone)]
pub struct CommandQueue {
    commands: Vec<UiCommand>,
    stats: CommandStats,
}

impl CommandQueue {
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(32)
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
            stats: CommandStats::default(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.stats.drained += self.commands.len();
        self.commands.clear();
    }

    #[inline]
    pub fn emit(&mut self, command: UiCommand) {
        if self.commands.len() == self.commands.capacity() {
            self.stats.capacity_growths += 1;
        }
        self.commands.push(command);
        self.stats.emitted += 1;
    }

    #[inline]
    pub fn emit_id<T>(&mut self, id: CommandId<T>) {
        self.emit(UiCommand::new(id.erase()));
    }

    #[inline]
    pub fn emit_bool<T>(&mut self, id: CommandId<T>, value: bool) {
        self.emit(UiCommand::new(id.erase()).payload(CommandPayload::Bool(value)));
    }

    #[inline]
    pub fn emit_f32<T>(&mut self, id: CommandId<T>, value: f32) {
        self.emit(UiCommand::new(id.erase()).payload(CommandPayload::F32(value)));
    }

    #[inline]
    pub fn emit_i32<T>(&mut self, id: CommandId<T>, value: i32) {
        self.emit(UiCommand::new(id.erase()).payload(CommandPayload::I32(value)));
    }

    #[inline]
    pub fn emit_u32<T>(&mut self, id: CommandId<T>, value: u32) {
        self.emit(UiCommand::new(id.erase()).payload(CommandPayload::U32(value)));
    }

    #[inline]
    pub fn emit_text<T>(&mut self, id: CommandId<T>, value: impl Into<String>) {
        self.emit(UiCommand::new(id.erase()).payload(CommandPayload::Text(value.into())));
    }

    #[inline]
    pub fn emit_binding<T>(&mut self, id: CommandId<T>, binding: BindingId<T>) {
        self.emit(UiCommand::new(id.erase()).payload(CommandPayload::BindingU64(binding.raw())));
    }

    #[inline]
    pub fn commands(&self) -> &[UiCommand] {
        &self.commands
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.commands.capacity()
    }

    #[inline]
    pub fn stats(&self) -> &CommandStats {
        &self.stats
    }

    pub fn drain_to(&mut self, out: &mut Vec<UiCommand>) {
        out.append(&mut self.commands);
        self.stats.drained = self.stats.emitted;
    }
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UpdateFlags {
    pub dirty: bool,
    pub layout_dirty: bool,
    pub text_dirty: bool,
    pub needs_redraw: bool,
}

impl UpdateFlags {
    #[inline]
    pub fn any(self) -> bool {
        self.dirty || self.layout_dirty || self.text_dirty || self.needs_redraw
    }

    #[inline]
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.needs_redraw = true;
    }

    #[inline]
    pub fn mark_layout(&mut self) {
        self.layout_dirty = true;
        self.needs_redraw = true;
    }

    #[inline]
    pub fn mark_text(&mut self) {
        self.text_dirty = true;
        self.needs_redraw = true;
    }
}

pub struct UpdateCtx<'a> {
    commands: &'a mut CommandQueue,
    flags: UpdateFlags,
    widget: Option<WidgetId>,
}

impl<'a> UpdateCtx<'a> {
    #[inline]
    pub fn new(commands: &'a mut CommandQueue) -> Self {
        Self {
            commands,
            flags: UpdateFlags::default(),
            widget: None,
        }
    }

    #[inline]
    pub fn emit<T>(&mut self, id: CommandId<T>, payload: CommandPayload) {
        let mut command = UiCommand::new(id.erase()).payload(payload);
        if let Some(widget) = self.widget {
            command = command.widget(widget);
        }
        self.commands.emit(command);
        self.flags.mark_dirty();
    }

    #[inline]
    pub fn emit_id<T>(&mut self, id: CommandId<T>) {
        self.emit(id, CommandPayload::None);
    }

    #[inline]
    pub fn emit_bool<T>(&mut self, id: CommandId<T>, value: bool) {
        self.emit(id, CommandPayload::Bool(value));
    }

    #[inline]
    pub fn emit_f32<T>(&mut self, id: CommandId<T>, value: f32) {
        self.emit(id, CommandPayload::F32(value));
    }

    #[inline]
    pub fn emit_i32<T>(&mut self, id: CommandId<T>, value: i32) {
        self.emit(id, CommandPayload::I32(value));
    }

    #[inline]
    pub fn emit_u32<T>(&mut self, id: CommandId<T>, value: u32) {
        self.emit(id, CommandPayload::U32(value));
    }

    #[inline]
    pub fn emit_text<T>(&mut self, id: CommandId<T>, value: impl Into<String>) {
        self.emit(id, CommandPayload::Text(value.into()));
    }

    #[inline]
    pub fn mark_dirty(&mut self) {
        self.flags.mark_dirty();
    }

    #[inline]
    pub fn mark_layout(&mut self) {
        self.flags.mark_layout();
    }

    #[inline]
    pub fn mark_text(&mut self) {
        self.flags.mark_text();
    }

    #[inline]
    pub fn flags(&self) -> UpdateFlags {
        self.flags
    }
}

impl fmt::Debug for UpdateCtx<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdateCtx")
            .field("flags", &self.flags)
            .field("widget", &self.widget)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_queue_reuses_capacity_after_clear() {
        let mut queue = CommandQueue::with_capacity(1);
        let id = CommandId::<()>::from_raw(7);
        queue.emit_id(id);
        let cap = queue.capacity();
        queue.clear();
        queue.emit_id(id);
        assert_eq!(queue.capacity(), cap);
    }

    #[test]
    fn update_ctx_marks_dirty_when_command_emitted() {
        let mut queue = CommandQueue::new();
        let mut ctx = UpdateCtx::new(&mut queue);
        ctx.emit_id(CommandId::<()>::from_raw(1));
        assert!(ctx.flags().dirty);
        assert_eq!(queue.len(), 1);
    }
}
