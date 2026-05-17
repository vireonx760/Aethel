use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindingId<T> {
    raw: u64,
    _marker: PhantomData<fn() -> T>,
}

impl<T> BindingId<T> {
    #[inline]
    pub fn raw(self) -> u64 {
        self.raw
    }
}

#[derive(Debug, Default)]
pub struct BindingIds {
    next: AtomicU64,
}

impl BindingIds {
    #[inline]
    pub fn new() -> Self {
        Self {
            next: AtomicU64::new(1),
        }
    }

    #[inline]
    pub fn alloc<T>(&self) -> BindingId<T> {
        BindingId {
            raw: self.next.fetch_add(1, Ordering::Relaxed),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct BoolSignal {
    inner: Arc<BoolInner>,
}

#[derive(Debug)]
struct BoolInner {
    value: AtomicBool,
    generation: AtomicU64,
}

impl BoolSignal {
    #[inline]
    pub fn new(value: bool) -> Self {
        Self {
            inner: Arc::new(BoolInner {
                value: AtomicBool::new(value),
                generation: AtomicU64::new(1),
            }),
        }
    }

    #[inline]
    pub fn get(&self) -> bool {
        self.inner.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set(&self, value: bool) -> bool {
        let old = self.inner.value.swap(value, Ordering::Relaxed);
        if old != value {
            self.inner.generation.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn toggle(&self) -> bool {
        let next = !self.get();
        self.set(next);
        next
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }
}

impl fmt::Debug for BoolSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoolSignal")
            .field("value", &self.get())
            .field("generation", &self.generation())
            .finish()
    }
}

#[derive(Clone)]
pub struct F32Signal {
    inner: Arc<F32Inner>,
}

#[derive(Debug)]
struct F32Inner {
    bits: AtomicU32,
    generation: AtomicU64,
}

impl F32Signal {
    #[inline]
    pub fn new(value: f32) -> Self {
        Self {
            inner: Arc::new(F32Inner {
                bits: AtomicU32::new(value.to_bits()),
                generation: AtomicU64::new(1),
            }),
        }
    }

    #[inline]
    pub fn get(&self) -> f32 {
        f32::from_bits(self.inner.bits.load(Ordering::Relaxed))
    }

    #[inline]
    pub fn set(&self, value: f32) -> bool {
        let value = if value.is_finite() { value } else { 0.0 };
        let bits = value.to_bits();
        let old = self.inner.bits.swap(bits, Ordering::Relaxed);
        if old != bits {
            self.inner.generation.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_clamped(&self, value: f32, min: f32, max: f32) -> bool {
        self.set(value.clamp(min, max))
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }
}

impl fmt::Debug for F32Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("F32Signal")
            .field("value", &self.get())
            .field("generation", &self.generation())
            .finish()
    }
}

#[derive(Clone)]
pub struct I32Signal {
    inner: Arc<I32Inner>,
}

#[derive(Debug)]
struct I32Inner {
    value: AtomicI32,
    generation: AtomicU64,
}

impl I32Signal {
    #[inline]
    pub fn new(value: i32) -> Self {
        Self {
            inner: Arc::new(I32Inner {
                value: AtomicI32::new(value),
                generation: AtomicU64::new(1),
            }),
        }
    }

    #[inline]
    pub fn get(&self) -> i32 {
        self.inner.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set(&self, value: i32) -> bool {
        let old = self.inner.value.swap(value, Ordering::Relaxed);
        if old != value {
            self.inner.generation.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_clamped(&self, value: i32, min: i32, max: i32) -> bool {
        self.set(value.clamp(min, max))
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }
}

impl fmt::Debug for I32Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("I32Signal")
            .field("value", &self.get())
            .field("generation", &self.generation())
            .finish()
    }
}

#[derive(Clone)]
pub struct U32Signal {
    inner: Arc<U32Inner>,
}

#[derive(Debug)]
struct U32Inner {
    value: AtomicU32,
    generation: AtomicU64,
}

impl U32Signal {
    #[inline]
    pub fn new(value: u32) -> Self {
        Self {
            inner: Arc::new(U32Inner {
                value: AtomicU32::new(value),
                generation: AtomicU64::new(1),
            }),
        }
    }

    #[inline]
    pub fn get(&self) -> u32 {
        self.inner.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set(&self, value: u32) -> bool {
        let old = self.inner.value.swap(value, Ordering::Relaxed);
        if old != value {
            self.inner.generation.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_clamped(&self, value: u32, min: u32, max: u32) -> bool {
        self.set(value.clamp(min, max))
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }
}

impl fmt::Debug for U32Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("U32Signal")
            .field("value", &self.get())
            .field("generation", &self.generation())
            .finish()
    }
}

#[derive(Clone)]
pub struct TextSignal {
    inner: Arc<TextInner>,
}

#[derive(Debug)]
struct TextInner {
    value: RwLock<String>,
    generation: AtomicU64,
}

impl TextSignal {
    #[inline]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(TextInner {
                value: RwLock::new(value.into()),
                generation: AtomicU64::new(1),
            }),
        }
    }

    pub fn get(&self) -> String {
        self.inner
            .value
            .read()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    pub fn read<R>(&self, f: impl FnOnce(&str) -> R) -> Option<R> {
        self.inner.value.read().ok().map(|value| f(&value))
    }

    pub fn set(&self, value: impl Into<String>) -> bool {
        let value = value.into();
        if let Ok(mut guard) = self.inner.value.write()
            && *guard != value
        {
            *guard = value;
            self.inner.generation.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        false
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }
}

impl fmt::Debug for TextSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSignal")
            .field("value", &self.get())
            .field("generation", &self.generation())
            .finish()
    }
}

#[derive(Clone)]
pub struct SelectionSignal<T> {
    inner: Arc<SelectionInner<T>>,
}

#[derive(Debug)]
struct SelectionInner<T> {
    value: RwLock<Option<T>>,
    generation: AtomicU64,
}

impl<T: Clone> SelectionSignal<T> {
    #[inline]
    pub fn new(value: Option<T>) -> Self {
        Self {
            inner: Arc::new(SelectionInner {
                value: RwLock::new(value),
                generation: AtomicU64::new(1),
            }),
        }
    }

    pub fn get(&self) -> Option<T> {
        self.inner.value.read().ok().and_then(|value| value.clone())
    }

    pub fn set(&self, value: Option<T>) -> bool {
        if let Ok(mut guard) = self.inner.value.write() {
            *guard = value;
            self.inner.generation.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        false
    }

    #[inline]
    pub fn clear(&self) -> bool {
        self.set(None)
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }
}

impl<T: Clone + fmt::Debug> fmt::Debug for SelectionSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectionSignal")
            .field("value", &self.get())
            .field("generation", &self.generation())
            .finish()
    }
}

#[derive(Clone)]
pub struct VecSignal<T> {
    inner: Arc<VecInner<T>>,
}

#[derive(Debug)]
struct VecInner<T> {
    value: RwLock<Vec<T>>,
    generation: AtomicU64,
}

impl<T: Clone> VecSignal<T> {
    #[inline]
    pub fn new(value: Vec<T>) -> Self {
        Self {
            inner: Arc::new(VecInner {
                value: RwLock::new(value),
                generation: AtomicU64::new(1),
            }),
        }
    }

    pub fn get(&self) -> Vec<T> {
        self.inner
            .value
            .read()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    pub fn replace(&self, value: Vec<T>) -> bool {
        if let Ok(mut guard) = self.inner.value.write() {
            *guard = value;
            self.inner.generation.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        false
    }

    pub fn mutate(&self, f: impl FnOnce(&mut Vec<T>)) -> bool {
        if let Ok(mut guard) = self.inner.value.write() {
            f(&mut guard);
            self.inner.generation.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        false
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }
}

impl<T: Clone + fmt::Debug> fmt::Debug for VecSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VecSignal")
            .field("value", &self.get())
            .field("generation", &self.generation())
            .finish()
    }
}

#[derive(Clone)]
pub struct LegacyMutexBinding<T> {
    target: Arc<Mutex<T>>,
}

impl<T> LegacyMutexBinding<T> {
    #[inline]
    pub fn new(target: Arc<Mutex<T>>) -> Self {
        Self { target }
    }

    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.target.lock().ok().map(|guard| f(&guard))
    }

    pub fn write<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.target.lock().ok().map(|mut guard| f(&mut guard))
    }

    #[inline]
    pub fn raw(&self) -> &Arc<Mutex<T>> {
        &self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_signal_generation_changes_only_on_change() {
        let signal = BoolSignal::new(false);
        let start = signal.generation();
        assert!(!signal.set(false));
        assert_eq!(signal.generation(), start);
        assert!(signal.set(true));
        assert!(signal.generation() > start);
    }

    #[test]
    fn f32_signal_sanitizes_nan() {
        let signal = F32Signal::new(1.0);
        signal.set(f32::NAN);
        assert_eq!(signal.get(), 0.0);
    }
}
