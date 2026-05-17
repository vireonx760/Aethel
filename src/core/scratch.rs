use std::collections::VecDeque;
use std::mem::size_of;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScratchKind {
    Vec,
    String,
    Queue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScratchPolicy {
    pub min_buffers: usize,
    pub max_idle_buffers: usize,
    pub max_idle_capacity: usize,
}

impl ScratchPolicy {
    #[inline]
    pub const fn new(
        min_buffers: usize,
        max_idle_buffers: usize,
        max_idle_capacity: usize,
    ) -> Self {
        Self {
            min_buffers,
            max_idle_buffers,
            max_idle_capacity,
        }
    }

    #[inline]
    pub const fn frame_default() -> Self {
        Self {
            min_buffers: 2,
            max_idle_buffers: 32,
            max_idle_capacity: 16 * 1024,
        }
    }

    #[inline]
    pub const fn text_default() -> Self {
        Self {
            min_buffers: 2,
            max_idle_buffers: 24,
            max_idle_capacity: 8 * 1024,
        }
    }

    #[inline]
    pub const fn unbounded() -> Self {
        Self {
            min_buffers: 0,
            max_idle_buffers: usize::MAX,
            max_idle_capacity: usize::MAX,
        }
    }

    #[inline]
    pub fn should_keep(self, idle_index: usize, capacity: usize) -> bool {
        idle_index < self.min_buffers
            || (idle_index < self.max_idle_buffers && capacity <= self.max_idle_capacity)
    }
}

impl Default for ScratchPolicy {
    #[inline]
    fn default() -> Self {
        Self::frame_default()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScratchStats {
    pub checkouts: u64,
    pub returns: u64,
    pub allocations: u64,
    pub capacity_grows: u64,
    pub trims: u64,
    pub high_water_buffers: usize,
    pub high_water_capacity: usize,
}

impl ScratchStats {
    #[inline]
    pub fn record_checkout(&mut self, in_use: usize) {
        self.checkouts += 1;
        self.high_water_buffers = self.high_water_buffers.max(in_use);
    }

    #[inline]
    pub fn record_return(&mut self) {
        self.returns += 1;
    }

    #[inline]
    pub fn record_allocation(&mut self, capacity: usize) {
        self.allocations += 1;
        self.high_water_capacity = self.high_water_capacity.max(capacity);
    }

    #[inline]
    pub fn record_growth(&mut self, capacity: usize) {
        self.capacity_grows += 1;
        self.high_water_capacity = self.high_water_capacity.max(capacity);
    }

    #[inline]
    pub fn record_trim(&mut self, count: usize) {
        self.trims += count as u64;
    }

    #[inline]
    pub fn reset_frame_counters(&mut self) {
        self.checkouts = 0;
        self.returns = 0;
        self.allocations = 0;
        self.capacity_grows = 0;
        self.trims = 0;
    }

    #[inline]
    pub fn has_frame_growth(self) -> bool {
        self.allocations != 0 || self.capacity_grows != 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScratchSnapshot {
    pub vec_buffers: usize,
    pub vec_capacity: usize,
    pub string_buffers: usize,
    pub string_capacity: usize,
    pub queue_capacity: usize,
}

impl ScratchSnapshot {
    #[inline]
    pub fn total_capacity_units(self) -> usize {
        self.vec_capacity + self.string_capacity + self.queue_capacity
    }

    #[inline]
    pub fn grew_since(self, previous: Self) -> bool {
        self.vec_capacity > previous.vec_capacity
            || self.string_capacity > previous.string_capacity
            || self.queue_capacity > previous.queue_capacity
            || self.vec_buffers > previous.vec_buffers
            || self.string_buffers > previous.string_buffers
    }

    #[inline]
    pub fn capacity_delta(self, previous: Self) -> isize {
        self.total_capacity_units() as isize - previous.total_capacity_units() as isize
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WarmFrameReport {
    pub before: ScratchSnapshot,
    pub after: ScratchSnapshot,
    pub grew: bool,
    pub capacity_delta: isize,
}

impl WarmFrameReport {
    #[inline]
    pub fn stable(self) -> bool {
        !self.grew && self.capacity_delta <= 0
    }
}

#[derive(Debug, Clone)]
pub struct ScratchVecPool<T> {
    available: Vec<Vec<T>>,
    in_use: usize,
    policy: ScratchPolicy,
    stats: ScratchStats,
    frame_epoch: u64,
}

impl<T> ScratchVecPool<T> {
    #[inline]
    pub fn new(policy: ScratchPolicy) -> Self {
        Self {
            available: Vec::new(),
            in_use: 0,
            policy,
            stats: ScratchStats::default(),
            frame_epoch: 0,
        }
    }

    #[inline]
    pub fn with_preallocated(buffers: usize, capacity: usize, policy: ScratchPolicy) -> Self {
        let mut pool = Self::new(policy);
        pool.preallocate(buffers, capacity);
        pool
    }

    pub fn preallocate(&mut self, buffers: usize, capacity: usize) {
        self.available.reserve(buffers);
        for _ in 0..buffers {
            self.available.push(Vec::with_capacity(capacity));
        }
        self.stats.high_water_capacity =
            self.stats.high_water_capacity.max(self.retained_capacity());
    }

    #[inline]
    pub fn begin_frame(&mut self) {
        self.frame_epoch = self.frame_epoch.wrapping_add(1);
        self.stats.reset_frame_counters();
    }

    #[inline]
    pub fn checkout(&mut self) -> Vec<T> {
        self.checkout_with_capacity(0)
    }

    pub fn checkout_with_capacity(&mut self, min_capacity: usize) -> Vec<T> {
        self.in_use += 1;
        self.stats.record_checkout(self.in_use);

        let mut buffer = match self.available.pop() {
            Some(mut buffer) => {
                buffer.clear();
                buffer
            }
            None => {
                self.stats.record_allocation(min_capacity);
                Vec::with_capacity(min_capacity)
            }
        };

        if buffer.capacity() < min_capacity {
            buffer.reserve(min_capacity - buffer.capacity());
            self.stats.record_growth(buffer.capacity());
        }

        buffer
    }

    #[inline]
    pub fn lease(&mut self) -> ScratchVecLease<'_, T> {
        self.lease_with_capacity(0)
    }

    #[inline]
    pub fn lease_with_capacity(&mut self, min_capacity: usize) -> ScratchVecLease<'_, T> {
        ScratchVecLease {
            buffer: Some(self.checkout_with_capacity(min_capacity)),
            pool: self,
        }
    }

    pub fn recycle(&mut self, mut buffer: Vec<T>) {
        if self.in_use > 0 {
            self.in_use -= 1;
        }
        buffer.clear();
        self.stats.record_return();

        let idle_index = self.available.len();
        if self.policy.should_keep(idle_index, buffer.capacity()) {
            self.available.push(buffer);
        } else {
            self.stats.record_trim(1);
        }
    }

    pub fn trim_idle(&mut self) -> usize {
        let before = self.available.len();
        let policy = self.policy;
        let mut kept = Vec::with_capacity(before.min(policy.max_idle_buffers));

        for mut buffer in self.available.drain(..) {
            buffer.clear();
            let idle_index = kept.len();
            if policy.should_keep(idle_index, buffer.capacity()) {
                kept.push(buffer);
            }
        }

        self.available = kept;
        let trimmed = before.saturating_sub(self.available.len());
        self.stats.record_trim(trimmed);
        trimmed
    }

    #[inline]
    pub fn set_policy(&mut self, policy: ScratchPolicy) {
        self.policy = policy;
    }

    #[inline]
    pub fn policy(&self) -> ScratchPolicy {
        self.policy
    }

    #[inline]
    pub fn retained_buffers(&self) -> usize {
        self.available.len()
    }

    #[inline]
    pub fn in_use(&self) -> usize {
        self.in_use
    }

    #[inline]
    pub fn retained_capacity(&self) -> usize {
        self.available.iter().map(Vec::capacity).sum()
    }

    #[inline]
    pub fn retained_bytes(&self) -> usize {
        self.retained_capacity().saturating_mul(size_of::<T>())
    }

    #[inline]
    pub fn frame_epoch(&self) -> u64 {
        self.frame_epoch
    }

    #[inline]
    pub fn stats(&self) -> ScratchStats {
        self.stats
    }
}

impl<T> Default for ScratchVecPool<T> {
    #[inline]
    fn default() -> Self {
        Self::new(ScratchPolicy::default())
    }
}

pub struct ScratchVecLease<'a, T> {
    buffer: Option<Vec<T>>,
    pool: &'a mut ScratchVecPool<T>,
}

impl<T> ScratchVecLease<'_, T> {
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buffer.as_ref().map_or(0, Vec::capacity)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.as_ref().map_or(0, Vec::len)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn into_vec(mut self) -> Vec<T> {
        self.buffer.take().unwrap_or_default()
    }
}

impl<T> Deref for ScratchVecLease<'_, T> {
    type Target = Vec<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.buffer.as_ref().expect("scratch vec lease was taken")
    }
}

impl<T> DerefMut for ScratchVecLease<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer.as_mut().expect("scratch vec lease was taken")
    }
}

impl<T> Drop for ScratchVecLease<'_, T> {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            self.pool.recycle(buffer);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScratchStringPool {
    available: Vec<String>,
    in_use: usize,
    policy: ScratchPolicy,
    stats: ScratchStats,
}

impl ScratchStringPool {
    #[inline]
    pub fn new(policy: ScratchPolicy) -> Self {
        Self {
            available: Vec::new(),
            in_use: 0,
            policy,
            stats: ScratchStats::default(),
        }
    }

    #[inline]
    pub fn with_preallocated(buffers: usize, capacity: usize, policy: ScratchPolicy) -> Self {
        let mut pool = Self::new(policy);
        pool.preallocate(buffers, capacity);
        pool
    }

    pub fn preallocate(&mut self, buffers: usize, capacity: usize) {
        self.available.reserve(buffers);
        for _ in 0..buffers {
            self.available.push(String::with_capacity(capacity));
        }
        self.stats.high_water_capacity =
            self.stats.high_water_capacity.max(self.retained_capacity());
    }

    #[inline]
    pub fn begin_frame(&mut self) {
        self.stats.reset_frame_counters();
    }

    #[inline]
    pub fn checkout(&mut self) -> String {
        self.checkout_with_capacity(0)
    }

    pub fn checkout_with_capacity(&mut self, min_capacity: usize) -> String {
        self.in_use += 1;
        self.stats.record_checkout(self.in_use);

        let mut buffer = match self.available.pop() {
            Some(mut buffer) => {
                buffer.clear();
                buffer
            }
            None => {
                self.stats.record_allocation(min_capacity);
                String::with_capacity(min_capacity)
            }
        };

        if buffer.capacity() < min_capacity {
            buffer.reserve(min_capacity - buffer.capacity());
            self.stats.record_growth(buffer.capacity());
        }

        buffer
    }

    #[inline]
    pub fn lease(&mut self) -> ScratchStringLease<'_> {
        self.lease_with_capacity(0)
    }

    #[inline]
    pub fn lease_with_capacity(&mut self, min_capacity: usize) -> ScratchStringLease<'_> {
        ScratchStringLease {
            buffer: Some(self.checkout_with_capacity(min_capacity)),
            pool: self,
        }
    }

    pub fn recycle(&mut self, mut buffer: String) {
        if self.in_use > 0 {
            self.in_use -= 1;
        }
        buffer.clear();
        self.stats.record_return();

        let idle_index = self.available.len();
        if self.policy.should_keep(idle_index, buffer.capacity()) {
            self.available.push(buffer);
        } else {
            self.stats.record_trim(1);
        }
    }

    pub fn trim_idle(&mut self) -> usize {
        let before = self.available.len();
        let policy = self.policy;
        let mut kept = Vec::with_capacity(before.min(policy.max_idle_buffers));

        for mut buffer in self.available.drain(..) {
            buffer.clear();
            let idle_index = kept.len();
            if policy.should_keep(idle_index, buffer.capacity()) {
                kept.push(buffer);
            }
        }

        self.available = kept;
        let trimmed = before.saturating_sub(self.available.len());
        self.stats.record_trim(trimmed);
        trimmed
    }

    #[inline]
    pub fn retained_buffers(&self) -> usize {
        self.available.len()
    }

    #[inline]
    pub fn in_use(&self) -> usize {
        self.in_use
    }

    #[inline]
    pub fn retained_capacity(&self) -> usize {
        self.available.iter().map(String::capacity).sum()
    }

    #[inline]
    pub fn stats(&self) -> ScratchStats {
        self.stats
    }
}

impl Default for ScratchStringPool {
    #[inline]
    fn default() -> Self {
        Self::new(ScratchPolicy::text_default())
    }
}

pub struct ScratchStringLease<'a> {
    buffer: Option<String>,
    pool: &'a mut ScratchStringPool,
}

impl ScratchStringLease<'_> {
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buffer.as_ref().map_or(0, String::capacity)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.as_ref().map_or(0, String::len)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn into_string(mut self) -> String {
        self.buffer.take().unwrap_or_default()
    }
}

impl Deref for ScratchStringLease<'_> {
    type Target = String;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.buffer
            .as_ref()
            .expect("scratch string lease was taken")
    }
}

impl DerefMut for ScratchStringLease<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
            .as_mut()
            .expect("scratch string lease was taken")
    }
}

impl Drop for ScratchStringLease<'_> {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            self.pool.recycle(buffer);
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameQueue<T> {
    items: VecDeque<T>,
    stats: ScratchStats,
}

impl<T> FrameQueue<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
            stats: ScratchStats::default(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: VecDeque::with_capacity(capacity),
            stats: ScratchStats::default(),
        }
    }

    #[inline]
    pub fn begin_frame(&mut self) {
        self.clear_reuse();
        self.stats.reset_frame_counters();
    }

    pub fn reserve(&mut self, additional: usize) {
        let before = self.items.capacity();
        self.items.reserve(additional);
        let after = self.items.capacity();
        if after > before {
            self.stats.record_growth(after);
        }
    }

    pub fn push(&mut self, item: T) {
        if self.items.len() == self.items.capacity() {
            self.stats
                .record_growth(self.items.capacity().saturating_add(1));
        }
        self.items.push_back(item);
        self.stats
            .record_checkout(self.stats.high_water_buffers.max(self.items.len()));
        self.stats.high_water_capacity = self.stats.high_water_capacity.max(self.items.capacity());
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    #[inline]
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.items.drain(..)
    }

    #[inline]
    pub fn clear_reuse(&mut self) {
        self.items.clear();
    }

    #[inline]
    pub fn shrink_to_policy(&mut self, policy: ScratchPolicy) {
        if self.items.capacity() > policy.max_idle_capacity {
            self.items
                .shrink_to(policy.max_idle_capacity.max(self.items.len()));
            self.stats.record_trim(1);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.items.capacity()
    }

    #[inline]
    pub fn stats(&self) -> ScratchStats {
        self.stats
    }
}

impl<T> Default for FrameQueue<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextScratchSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
}

impl TextScratchSpan {
    #[inline]
    pub const fn new(start: usize, end: usize, line: usize) -> Self {
        Self { start, end, line }
    }

    #[inline]
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, Clone)]
pub struct FrameScratch {
    widget_indices: ScratchVecPool<usize>,
    dirty_widgets: ScratchVecPool<usize>,
    text_spans: ScratchVecPool<TextScratchSpan>,
    strings: ScratchStringPool,
    events: FrameQueue<usize>,
    policy: ScratchPolicy,
}

impl FrameScratch {
    #[inline]
    pub fn new() -> Self {
        Self::with_policy(ScratchPolicy::frame_default())
    }

    pub fn with_policy(policy: ScratchPolicy) -> Self {
        let text_policy = ScratchPolicy {
            min_buffers: policy.min_buffers,
            max_idle_buffers: policy.max_idle_buffers,
            max_idle_capacity: policy.max_idle_capacity / 2,
        };

        Self {
            widget_indices: ScratchVecPool::with_preallocated(policy.min_buffers, 64, policy),
            dirty_widgets: ScratchVecPool::with_preallocated(policy.min_buffers, 64, policy),
            text_spans: ScratchVecPool::with_preallocated(policy.min_buffers, 32, policy),
            strings: ScratchStringPool::with_preallocated(policy.min_buffers, 128, text_policy),
            events: FrameQueue::with_capacity(64),
            policy,
        }
    }

    #[inline]
    pub fn begin_frame(&mut self) {
        self.widget_indices.begin_frame();
        self.dirty_widgets.begin_frame();
        self.text_spans.begin_frame();
        self.strings.begin_frame();
        self.events.begin_frame();
    }

    #[inline]
    pub fn widget_indices(&mut self, min_capacity: usize) -> ScratchVecLease<'_, usize> {
        self.widget_indices.lease_with_capacity(min_capacity)
    }

    #[inline]
    pub fn dirty_widgets(&mut self, min_capacity: usize) -> ScratchVecLease<'_, usize> {
        self.dirty_widgets.lease_with_capacity(min_capacity)
    }

    #[inline]
    pub fn text_spans(&mut self, min_capacity: usize) -> ScratchVecLease<'_, TextScratchSpan> {
        self.text_spans.lease_with_capacity(min_capacity)
    }

    #[inline]
    pub fn string(&mut self, min_capacity: usize) -> ScratchStringLease<'_> {
        self.strings.lease_with_capacity(min_capacity)
    }

    #[inline]
    pub fn events_mut(&mut self) -> &mut FrameQueue<usize> {
        &mut self.events
    }

    #[inline]
    pub fn snapshot(&self) -> ScratchSnapshot {
        ScratchSnapshot {
            vec_buffers: self.widget_indices.retained_buffers()
                + self.dirty_widgets.retained_buffers()
                + self.text_spans.retained_buffers(),
            vec_capacity: self.widget_indices.retained_capacity()
                + self.dirty_widgets.retained_capacity()
                + self.text_spans.retained_capacity(),
            string_buffers: self.strings.retained_buffers(),
            string_capacity: self.strings.retained_capacity(),
            queue_capacity: self.events.capacity(),
        }
    }

    pub fn trim_idle(&mut self) -> usize {
        let mut trimmed = 0;
        trimmed += self.widget_indices.trim_idle();
        trimmed += self.dirty_widgets.trim_idle();
        trimmed += self.text_spans.trim_idle();
        trimmed += self.strings.trim_idle();
        self.events.shrink_to_policy(self.policy);
        trimmed
    }

    #[inline]
    pub fn warm_frame_guard(&self) -> WarmFrameGuard {
        WarmFrameGuard::new(self.snapshot())
    }

    #[inline]
    pub fn stats(&self) -> FrameScratchStats {
        FrameScratchStats {
            widget_indices: self.widget_indices.stats(),
            dirty_widgets: self.dirty_widgets.stats(),
            text_spans: self.text_spans.stats(),
            strings: self.strings.stats(),
            events: self.events.stats(),
        }
    }
}

impl Default for FrameScratch {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameScratchStats {
    pub widget_indices: ScratchStats,
    pub dirty_widgets: ScratchStats,
    pub text_spans: ScratchStats,
    pub strings: ScratchStats,
    pub events: ScratchStats,
}

impl FrameScratchStats {
    #[inline]
    pub fn had_growth(self) -> bool {
        self.widget_indices.has_frame_growth()
            || self.dirty_widgets.has_frame_growth()
            || self.text_spans.has_frame_growth()
            || self.strings.has_frame_growth()
            || self.events.has_frame_growth()
    }

    #[inline]
    pub fn total_allocations(self) -> u64 {
        self.widget_indices.allocations
            + self.dirty_widgets.allocations
            + self.text_spans.allocations
            + self.strings.allocations
            + self.events.allocations
    }

    #[inline]
    pub fn total_capacity_grows(self) -> u64 {
        self.widget_indices.capacity_grows
            + self.dirty_widgets.capacity_grows
            + self.text_spans.capacity_grows
            + self.strings.capacity_grows
            + self.events.capacity_grows
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarmFrameGuard {
    before: ScratchSnapshot,
}

impl WarmFrameGuard {
    #[inline]
    pub const fn new(before: ScratchSnapshot) -> Self {
        Self { before }
    }

    #[inline]
    pub fn finish(self, after: ScratchSnapshot) -> WarmFrameReport {
        WarmFrameReport {
            before: self.before,
            after,
            grew: after.grew_since(self.before),
            capacity_delta: after.capacity_delta(self.before),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_pool_reuses_capacity_after_return() {
        let mut pool = ScratchVecPool::<u32>::with_preallocated(1, 8, ScratchPolicy::unbounded());
        pool.begin_frame();
        let mut buffer = pool.checkout_with_capacity(8);
        buffer.extend([1, 2, 3, 4]);
        let capacity = buffer.capacity();
        pool.recycle(buffer);

        let second = pool.checkout_with_capacity(4);
        assert!(second.capacity() >= capacity);
        assert_eq!(pool.stats().allocations, 0);
    }

    #[test]
    fn vec_lease_returns_buffer_on_drop() {
        let mut pool = ScratchVecPool::<usize>::default();
        {
            let mut lease = pool.lease_with_capacity(16);
            lease.extend([1, 2, 3]);
            assert_eq!(lease.len(), 3);
        }
        assert_eq!(pool.in_use(), 0);
        assert_eq!(pool.retained_buffers(), 1);
    }

    #[test]
    fn string_pool_reuses_buffer() {
        let mut pool = ScratchStringPool::with_preallocated(1, 16, ScratchPolicy::unbounded());
        let mut text = pool.checkout_with_capacity(16);
        text.push_str("hello");
        pool.recycle(text);

        let text = pool.checkout_with_capacity(5);
        assert!(text.capacity() >= 16);
        assert!(text.is_empty());
    }

    #[test]
    fn frame_queue_preserves_order() {
        let mut queue = FrameQueue::with_capacity(4);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn frame_scratch_warm_guard_reports_stable_frame() {
        let mut scratch = FrameScratch::new();
        {
            let mut ids = scratch.widget_indices(8);
            ids.extend(0..8);
        }

        let guard = scratch.warm_frame_guard();
        scratch.begin_frame();
        {
            let mut ids = scratch.widget_indices(4);
            ids.extend(0..4);
        }
        let report = guard.finish(scratch.snapshot());
        assert!(report.stable());
    }

    #[test]
    fn text_span_len_saturates() {
        let span = TextScratchSpan::new(10, 4, 0);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
    }
}
