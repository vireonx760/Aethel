use crate::gui::geometry::{Rect, Size};
use crate::gui::style::{ColorRgba, TextStyle};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextGeneration(pub u64);

impl TextGeneration {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextKey {
    hash: u64,
}

impl TextKey {
    #[inline]
    pub fn new(text: &str, style: TextStyle, bounds: Size) -> Self {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        style.size.to_bits().hash(&mut hasher);
        style.line_height.to_bits().hash(&mut hasher);
        style.color.r.to_bits().hash(&mut hasher);
        style.color.g.to_bits().hash(&mut hasher);
        style.color.b.to_bits().hash(&mut hasher);
        style.color.a.to_bits().hash(&mut hasher);
        bounds.width.to_bits().hash(&mut hasher);
        bounds.height.to_bits().hash(&mut hasher);
        Self {
            hash: hasher.finish(),
        }
    }

    #[inline]
    pub fn raw(self) -> u64 {
        self.hash
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMetricsCache {
    pub width: f32,
    pub height: f32,
    pub line_count: usize,
    pub generation: TextGeneration,
}

impl TextMetricsCache {
    #[inline]
    pub fn empty() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            line_count: 0,
            generation: TextGeneration::ZERO,
        }
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0 || self.line_count == 0
    }

    #[inline]
    pub fn size(self) -> Size {
        Size::new(self.width, self.height)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextRunCache {
    key: Option<TextKey>,
    metrics: TextMetricsCache,
    generation: TextGeneration,
}

impl TextRunCache {
    #[inline]
    pub fn new() -> Self {
        Self {
            key: None,
            metrics: TextMetricsCache::empty(),
            generation: TextGeneration::ZERO,
        }
    }

    #[inline]
    pub fn should_shape(&self, key: TextKey) -> bool {
        self.key != Some(key)
    }

    #[inline]
    pub fn update(&mut self, key: TextKey, metrics: TextMetricsCache) {
        self.key = Some(key);
        self.metrics = metrics;
        self.generation = self.generation.next();
    }

    #[inline]
    pub fn invalidate(&mut self) {
        self.key = None;
        self.generation = self.generation.next();
    }

    #[inline]
    pub fn metrics(&self) -> TextMetricsCache {
        self.metrics
    }

    #[inline]
    pub fn generation(&self) -> TextGeneration {
        self.generation
    }
}

impl Default for TextRunCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextPlacement {
    pub left: f32,
    pub top: f32,
    pub bounds: Rect,
}

impl TextPlacement {
    pub fn within(rect: Rect, text_size: Size, horizontal: TextAlign, vertical: TextAlign) -> Self {
        let x = match horizontal {
            TextAlign::Start => rect.x,
            TextAlign::Center => rect.x + (rect.width - text_size.width).max(0.0) * 0.5,
            TextAlign::End => rect.x + (rect.width - text_size.width).max(0.0),
        };
        let y = match vertical {
            TextAlign::Start => rect.y,
            TextAlign::Center => rect.y + (rect.height - text_size.height).max(0.0) * 0.5,
            TextAlign::End => rect.y + (rect.height - text_size.height).max(0.0),
        };
        Self {
            left: x,
            top: y,
            bounds: rect,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextScratchPlan {
    pub expected_buffers: usize,
    pub expected_areas: usize,
    pub expected_glyph_runs: usize,
}

impl TextScratchPlan {
    #[inline]
    pub fn empty() -> Self {
        Self {
            expected_buffers: 0,
            expected_areas: 0,
            expected_glyph_runs: 0,
        }
    }

    #[inline]
    pub fn reserve_for_widget(mut self, text_areas: usize, glyph_runs: usize) -> Self {
        self.expected_buffers += text_areas;
        self.expected_areas += text_areas;
        self.expected_glyph_runs += glyph_runs;
        self
    }

    #[inline]
    pub fn merge(&mut self, other: &Self) {
        self.expected_buffers += other.expected_buffers;
        self.expected_areas += other.expected_areas;
        self.expected_glyph_runs += other.expected_glyph_runs;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextColorRamp {
    pub normal: ColorRgba,
    pub muted: ColorRgba,
    pub accent: ColorRgba,
    pub disabled: ColorRgba,
}

impl TextColorRamp {
    #[inline]
    pub fn dark_default() -> Self {
        Self {
            normal: ColorRgba::new(0.9, 0.9, 0.95, 1.0),
            muted: ColorRgba::new(0.6, 0.6, 0.68, 1.0),
            accent: ColorRgba::new(0.2, 0.7, 1.0, 1.0),
            disabled: ColorRgba::new(0.4, 0.4, 0.45, 1.0),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextCacheStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub invalidations: u64,
}

impl TextCacheStats {
    #[inline]
    pub fn hit(&mut self) {
        self.cache_hits += 1;
    }

    #[inline]
    pub fn miss(&mut self) {
        self.cache_misses += 1;
    }

    #[inline]
    pub fn invalidate(&mut self) {
        self.invalidations += 1;
    }

    #[inline]
    pub fn hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextCacheBook {
    entries: Vec<TextRunCache>,
    stats: TextCacheStats,
}

impl TextCacheBook {
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(64),
            stats: TextCacheStats::default(),
        }
    }

    #[inline]
    pub fn ensure_entry(&mut self, index: usize) -> &mut TextRunCache {
        if index >= self.entries.len() {
            self.entries.resize_with(index + 1, TextRunCache::new);
        }
        &mut self.entries[index]
    }

    pub fn check_key(&mut self, index: usize, key: TextKey) -> bool {
        let should_shape = self.ensure_entry(index).should_shape(key);
        if should_shape {
            self.stats.miss();
        } else {
            self.stats.hit();
        }
        should_shape
    }

    pub fn update(&mut self, index: usize, key: TextKey, metrics: TextMetricsCache) {
        self.ensure_entry(index).update(key, metrics);
    }

    pub fn invalidate(&mut self, index: usize) {
        self.ensure_entry(index).invalidate();
        self.stats.invalidate();
    }

    #[inline]
    pub fn stats(&self) -> &TextCacheStats {
        &self.stats
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for TextCacheBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_key_changes_with_bounds() {
        let style = TextStyle::new(12.0, [1.0, 1.0, 1.0, 1.0]);
        let a = TextKey::new("hello", style, Size::new(10.0, 10.0));
        let b = TextKey::new("hello", style, Size::new(20.0, 10.0));
        assert_ne!(a, b);
    }

    #[test]
    fn cache_book_records_hit_and_miss() {
        let mut book = TextCacheBook::new();
        let style = TextStyle::new(12.0, [1.0, 1.0, 1.0, 1.0]);
        let key = TextKey::new("hello", style, Size::new(10.0, 10.0));
        assert!(book.check_key(0, key));
        book.update(0, key, TextMetricsCache::empty());
        assert!(!book.check_key(0, key));
        assert_eq!(book.stats().cache_misses, 1);
        assert_eq!(book.stats().cache_hits, 1);
    }
}
