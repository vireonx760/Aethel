use crate::gui::geometry::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityEvent {
    Reused,
    Grew { old: usize, new: usize },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReuseStats {
    pub clears: u64,
    pub grows: u64,
    pub peak_len: usize,
    pub peak_capacity: usize,
}

impl ReuseStats {
    #[inline]
    pub fn record_clear(&mut self) {
        self.clears += 1;
    }

    #[inline]
    pub fn record_len(&mut self, len: usize, capacity: usize) {
        self.peak_len = self.peak_len.max(len);
        self.peak_capacity = self.peak_capacity.max(capacity);
    }

    #[inline]
    pub fn record_grow(&mut self) {
        self.grows += 1;
    }
}

#[derive(Debug, Clone)]
pub struct ReusableVec<T> {
    items: Vec<T>,
    stats: ReuseStats,
}

impl<T> ReusableVec<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            stats: ReuseStats::default(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            stats: ReuseStats {
                peak_capacity: capacity,
                ..ReuseStats::default()
            },
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
        self.stats.record_clear();
    }

    #[inline]
    pub fn push(&mut self, value: T) -> CapacityEvent {
        let old = self.items.capacity();
        self.items.push(value);
        let new = self.items.capacity();
        self.stats.record_len(self.items.len(), new);
        if new > old {
            self.stats.record_grow();
            CapacityEvent::Grew { old, new }
        } else {
            CapacityEvent::Reused
        }
    }

    #[inline]
    pub fn extend_from_slice(&mut self, values: &[T])
    where
        T: Clone,
    {
        let old = self.items.capacity();
        self.items.extend_from_slice(values);
        let new = self.items.capacity();
        self.stats.record_len(self.items.len(), new);
        if new > old {
            self.stats.record_grow();
        }
    }

    #[inline]
    pub fn reserve_exact_for(&mut self, additional: usize) -> CapacityEvent {
        let old = self.items.capacity();
        self.items.reserve_exact(additional);
        let new = self.items.capacity();
        self.stats.record_len(self.items.len(), new);
        if new > old {
            self.stats.record_grow();
            CapacityEvent::Grew { old, new }
        } else {
            CapacityEvent::Reused
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.items
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
    pub fn stats(&self) -> &ReuseStats {
        &self.stats
    }

    #[inline]
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<T> Default for ReusableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReusableString {
    text: String,
    stats: ReuseStats,
}

impl ReusableString {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            text: String::with_capacity(capacity),
            stats: ReuseStats {
                peak_capacity: capacity,
                ..ReuseStats::default()
            },
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.text.clear();
        self.stats.record_clear();
    }

    #[inline]
    pub fn push_str(&mut self, value: &str) -> CapacityEvent {
        let old = self.text.capacity();
        self.text.push_str(value);
        let new = self.text.capacity();
        self.stats.record_len(self.text.len(), new);
        if new > old {
            self.stats.record_grow();
            CapacityEvent::Grew { old, new }
        } else {
            CapacityEvent::Reused
        }
    }

    #[inline]
    pub fn set(&mut self, value: &str) -> CapacityEvent {
        self.clear();
        self.push_str(value)
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.text.capacity()
    }

    #[inline]
    pub fn stats(&self) -> &ReuseStats {
        &self.stats
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitIndex(pub usize);

#[derive(Debug, Clone)]
pub struct BitSet {
    words: Vec<u64>,
    len: usize,
}

impl BitSet {
    #[inline]
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            len: 0,
        }
    }

    #[inline]
    pub fn with_len(len: usize) -> Self {
        let words = len.div_ceil(64);
        Self {
            words: vec![0; words],
            len,
        }
    }

    #[inline]
    pub fn resize(&mut self, len: usize) {
        self.len = len;
        self.words.resize(len.div_ceil(64), 0);
        self.clear_unused_bits();
    }

    #[inline]
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    #[inline]
    pub fn fill(&mut self) {
        self.words.fill(u64::MAX);
        self.clear_unused_bits();
    }

    #[inline]
    pub fn insert(&mut self, index: BitIndex) {
        self.ensure(index.0 + 1);
        let word = index.0 / 64;
        let bit = index.0 % 64;
        self.words[word] |= 1u64 << bit;
    }

    #[inline]
    pub fn remove(&mut self, index: BitIndex) {
        if index.0 >= self.len {
            return;
        }
        let word = index.0 / 64;
        let bit = index.0 % 64;
        self.words[word] &= !(1u64 << bit);
    }

    #[inline]
    pub fn contains(&self, index: BitIndex) -> bool {
        if index.0 >= self.len {
            return false;
        }
        let word = index.0 / 64;
        let bit = index.0 % 64;
        self.words[word] & (1u64 << bit) != 0
    }

    #[inline]
    pub fn union_with(&mut self, other: &Self) {
        self.ensure(other.len);
        for (left, right) in self.words.iter_mut().zip(&other.words) {
            *left |= *right;
        }
    }

    #[inline]
    pub fn intersect_with(&mut self, other: &Self) {
        for (i, word) in self.words.iter_mut().enumerate() {
            *word &= other.words.get(i).copied().unwrap_or(0);
        }
    }

    #[inline]
    pub fn count_ones(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0 || self.words.iter().all(|word| *word == 0)
    }

    pub fn iter_ones(&self) -> BitSetOnes<'_> {
        BitSetOnes {
            set: self,
            word_index: 0,
            word: self.words.first().copied().unwrap_or(0),
        }
    }

    fn ensure(&mut self, len: usize) {
        if len > self.len {
            self.resize(len);
        }
    }

    fn clear_unused_bits(&mut self) {
        let unused = self.words.len() * 64 - self.len;
        if unused == 0 || self.words.is_empty() {
            return;
        }
        let keep = 64 - unused;
        let mask = if keep == 64 {
            u64::MAX
        } else {
            (1u64 << keep) - 1
        };
        if let Some(last) = self.words.last_mut() {
            *last &= mask;
        }
    }
}

impl Default for BitSet {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BitSetOnes<'a> {
    set: &'a BitSet,
    word_index: usize,
    word: u64,
}

impl Iterator for BitSetOnes<'_> {
    type Item = BitIndex;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.word != 0 {
                let bit = self.word.trailing_zeros() as usize;
                self.word &= self.word - 1;
                let index = self.word_index * 64 + bit;
                if index < self.set.len {
                    return Some(BitIndex(index));
                }
            }

            self.word_index += 1;
            self.word = *self.set.words.get(self.word_index)?;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirtyRegion {
    rect: Option<Rect>,
}

impl DirtyRegion {
    #[inline]
    pub fn clean() -> Self {
        Self { rect: None }
    }

    #[inline]
    pub fn full(width: f32, height: f32) -> Self {
        Self {
            rect: Some(Rect::new(0.0, 0.0, width.max(0.0), height.max(0.0))),
        }
    }

    #[inline]
    pub fn is_dirty(self) -> bool {
        self.rect.is_some()
    }

    #[inline]
    pub fn rect(self) -> Option<Rect> {
        self.rect
    }

    #[inline]
    pub fn clear(&mut self) {
        self.rect = None;
    }

    pub fn add_rect(&mut self, rect: Rect) {
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }

        self.rect = Some(match self.rect {
            Some(current) => union_rect(current, rect),
            None => rect,
        });
    }

    pub fn add_many(&mut self, rects: impl IntoIterator<Item = Rect>) {
        for rect in rects {
            self.add_rect(rect);
        }
    }
}

impl Default for DirtyRegion {
    fn default() -> Self {
        Self::clean()
    }
}

#[inline]
fn union_rect(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = a.right().max(b.right());
    let y1 = a.bottom().max(b.bottom());
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitset_iterates_set_bits() {
        let mut set = BitSet::with_len(130);
        set.insert(BitIndex(3));
        set.insert(BitIndex(64));
        set.insert(BitIndex(129));
        let values: Vec<_> = set.iter_ones().map(|idx| idx.0).collect();
        assert_eq!(values, vec![3, 64, 129]);
    }

    #[test]
    fn dirty_region_unions_rects() {
        let mut dirty = DirtyRegion::clean();
        dirty.add_rect(Rect::new(10.0, 10.0, 5.0, 5.0));
        dirty.add_rect(Rect::new(0.0, 0.0, 5.0, 5.0));
        assert_eq!(dirty.rect(), Some(Rect::new(0.0, 0.0, 15.0, 15.0)));
    }
}
