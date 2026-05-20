use crate::core::renderer::WidgetInstance;

#[inline]
pub fn all_finite2(values: [f32; 2]) -> bool {
    all_finite4([values[0], values[1], 0.0, 0.0])
}

#[inline]
pub fn all_finite4(values: [f32; 4]) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe { x86::all_finite4_sse2(values) }
    }

    #[cfg(target_arch = "x86")]
    {
        if std::arch::is_x86_feature_detected!("sse2") {
            unsafe { x86::all_finite4_sse2(values) }
        } else {
            all_finite4_scalar(values)
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        all_finite4_scalar(values)
    }
}

#[inline]
pub fn clamp01_f32x4(values: [f32; 4]) -> [f32; 4] {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe { x86::clamp01_f32x4_sse2(values) }
    }

    #[cfg(target_arch = "x86")]
    {
        if std::arch::is_x86_feature_detected!("sse2") {
            unsafe { x86::clamp01_f32x4_sse2(values) }
        } else {
            clamp01_f32x4_scalar(values)
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        clamp01_f32x4_scalar(values)
    }
}

#[inline]
pub fn clamp_f32x2(values: [f32; 2], min: [f32; 2], max: [f32; 2]) -> [f32; 2] {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe { x86::clamp_f32x2_sse2(values, min, max) }
    }

    #[cfg(target_arch = "x86")]
    {
        if std::arch::is_x86_feature_detected!("sse2") {
            unsafe { x86::clamp_f32x2_sse2(values, min, max) }
        } else {
            clamp_f32x2_scalar(values, min, max)
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        clamp_f32x2_scalar(values, min, max)
    }
}

#[inline]
pub fn dot3_f32x4(a: [f32; 4], b: [f32; 4]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe { x86::dot3_f32x4_sse2(a, b) }
    }

    #[cfg(target_arch = "x86")]
    {
        if std::arch::is_x86_feature_detected!("sse2") {
            return unsafe { x86::dot3_f32x4_sse2(a, b) };
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    #[cfg(target_arch = "x86")]
    {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
}

#[inline]
pub fn intersect_ltrb(a: [f32; 4], b: [f32; 4]) -> Option<[f32; 4]> {
    let out = {
        #[cfg(target_arch = "x86_64")]
        {
            unsafe { x86::intersect_ltrb_sse2(a, b) }
        }

        #[cfg(target_arch = "x86")]
        {
            if std::arch::is_x86_feature_detected!("sse2") {
                unsafe { x86::intersect_ltrb_sse2(a, b) }
            } else {
                intersect_ltrb_scalar(a, b)
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        {
            intersect_ltrb_scalar(a, b)
        }
    };

    if out[2] > out[0] && out[3] > out[1] {
        Some(out)
    } else {
        None
    }
}

#[inline]
pub fn translate_widget_instances(instances: &mut [WidgetInstance], delta: [f32; 2]) {
    if delta[0] == 0.0 && delta[1] == 0.0 {
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            x86::translate_widget_instances_sse2(instances, delta);
        }
    }

    #[cfg(target_arch = "x86")]
    {
        if std::arch::is_x86_feature_detected!("sse2") {
            unsafe {
                x86::translate_widget_instances_sse2(instances, delta);
            }
        } else {
            translate_widget_instances_scalar(instances, delta);
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        translate_widget_instances_scalar(instances, delta);
    }
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn all_finite4_scalar(values: [f32; 4]) -> bool {
    values.iter().all(|value| value.is_finite())
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn clamp01_f32x4_scalar(values: [f32; 4]) -> [f32; 4] {
    [
        values[0].clamp(0.0, 1.0),
        values[1].clamp(0.0, 1.0),
        values[2].clamp(0.0, 1.0),
        values[3].clamp(0.0, 1.0),
    ]
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn clamp_f32x2_scalar(values: [f32; 2], min: [f32; 2], max: [f32; 2]) -> [f32; 2] {
    [
        values[0].clamp(min[0], max[0]),
        values[1].clamp(min[1], max[1]),
    ]
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn intersect_ltrb_scalar(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ]
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn translate_widget_instances_scalar(instances: &mut [WidgetInstance], delta: [f32; 2]) {
    for instance in instances {
        instance.pos[0] += delta[0];
        instance.pos[1] += delta[1];
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
mod x86 {
    use crate::core::renderer::WidgetInstance;

    #[cfg(target_arch = "x86")]
    use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::*;

    #[target_feature(enable = "sse2")]
    pub unsafe fn all_finite4_sse2(values: [f32; 4]) -> bool {
        let value = unsafe { _mm_loadu_ps(values.as_ptr()) };
        let abs_mask = _mm_set1_ps(-0.0);
        let abs = _mm_andnot_ps(abs_mask, value);
        let max = _mm_set1_ps(f32::MAX);
        let finite = _mm_cmple_ps(abs, max);
        _mm_movemask_ps(finite) == 0b1111
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn clamp01_f32x4_sse2(values: [f32; 4]) -> [f32; 4] {
        let value = unsafe { _mm_loadu_ps(values.as_ptr()) };
        let zero = _mm_set1_ps(0.0);
        let one = _mm_set1_ps(1.0);
        let clamped = _mm_min_ps(_mm_max_ps(value, zero), one);
        let mut out = [0.0; 4];
        unsafe { _mm_storeu_ps(out.as_mut_ptr(), clamped) };
        out
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn clamp_f32x2_sse2(values: [f32; 2], min: [f32; 2], max: [f32; 2]) -> [f32; 2] {
        let value = _mm_set_ps(0.0, 0.0, values[1], values[0]);
        let min = _mm_set_ps(0.0, 0.0, min[1], min[0]);
        let max = _mm_set_ps(0.0, 0.0, max[1], max[0]);
        let clamped = _mm_min_ps(_mm_max_ps(value, min), max);
        let mut out = [0.0; 4];
        unsafe { _mm_storeu_ps(out.as_mut_ptr(), clamped) };
        [out[0], out[1]]
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn dot3_f32x4_sse2(a: [f32; 4], b: [f32; 4]) -> f32 {
        let av = unsafe { _mm_loadu_ps(a.as_ptr()) };
        let bv = unsafe { _mm_loadu_ps(b.as_ptr()) };
        let product = _mm_mul_ps(av, bv);
        let mut lanes = [0.0; 4];
        unsafe { _mm_storeu_ps(lanes.as_mut_ptr(), product) };
        lanes[0] + lanes[1] + lanes[2]
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn intersect_ltrb_sse2(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
        let av = unsafe { _mm_loadu_ps(a.as_ptr()) };
        let bv = unsafe { _mm_loadu_ps(b.as_ptr()) };
        let lower = _mm_max_ps(av, bv);
        let upper = _mm_min_ps(av, bv);
        let mut lo = [0.0; 4];
        let mut hi = [0.0; 4];
        unsafe {
            _mm_storeu_ps(lo.as_mut_ptr(), lower);
            _mm_storeu_ps(hi.as_mut_ptr(), upper);
        }
        [lo[0], lo[1], hi[2], hi[3]]
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn translate_widget_instances_sse2(
        instances: &mut [WidgetInstance],
        delta: [f32; 2],
    ) {
        let add = _mm_set_ps(0.0, 0.0, delta[1], delta[0]);
        for instance in instances {
            let ptr = instance as *mut WidgetInstance as *mut f32;
            let value = unsafe { _mm_loadu_ps(ptr) };
            let translated = _mm_add_ps(value, add);
            unsafe { _mm_storeu_ps(ptr, translated) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp01_handles_four_lanes() {
        assert_eq!(clamp01_f32x4([-1.0, 0.25, 2.0, 1.0]), [0.0, 0.25, 1.0, 1.0]);
    }

    #[test]
    fn finite_rejects_nan_and_infinity() {
        assert!(all_finite4([0.0, -1.0, 4.0, f32::MAX]));
        assert!(!all_finite4([0.0, f32::INFINITY, 4.0, 1.0]));
        assert!(!all_finite4([0.0, f32::NAN, 4.0, 1.0]));
    }

    #[test]
    fn intersect_ltrb_returns_overlap() {
        assert_eq!(
            intersect_ltrb([0.0, 0.0, 10.0, 10.0], [5.0, 4.0, 15.0, 9.0]),
            Some([5.0, 4.0, 10.0, 9.0])
        );
        assert_eq!(
            intersect_ltrb([0.0, 0.0, 2.0, 2.0], [5.0, 5.0, 8.0, 8.0]),
            None
        );
    }

    #[test]
    fn dot3_ignores_padding_lane() {
        assert_eq!(
            dot3_f32x4([1.0, 2.0, 3.0, 99.0], [4.0, 5.0, 6.0, 99.0]),
            32.0
        );
    }

    #[test]
    fn translate_instances_updates_only_position_lanes() {
        let mut instances = [WidgetInstance {
            pos: [1.0, 2.0],
            size: [3.0, 4.0],
            ..Default::default()
        }];
        translate_widget_instances(&mut instances, [5.0, 6.0]);
        assert_eq!(instances[0].pos, [6.0, 8.0]);
        assert_eq!(instances[0].size, [3.0, 4.0]);
    }
}
