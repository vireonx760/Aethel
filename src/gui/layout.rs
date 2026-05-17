// src/gui/layout.rs

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    Fixed(f32),
    Percent(f32),
    Fill(f32),
    #[default]
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutParams {
    pub width: Length,
    pub height: Length,
    pub padding: [f32; 4],
    pub margin: [f32; 4],
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            width: Length::Content,
            height: Length::Content,
            padding: [0.0; 4],
            margin: [0.0; 4],
        }
    }
}

///
/// # Margin
///
pub fn calculate_linear_layout(
    axis: Axis,
    available_space: Rect,
    children_params: &[LayoutParams],
    children_content_sizes: &[(f32, f32)],
    spacing: f32,
    alignment: Alignment,
) -> Vec<Rect> {
    let count = children_params.len();
    if count == 0 {
        return Vec::new();
    }

    let mut result_rects = vec![Rect::default(); count];

    let (total_avail_main, total_avail_cross) = match axis {
        Axis::Horizontal => (available_space.w, available_space.h),
        Axis::Vertical => (available_space.h, available_space.w),
    };

    let mut used_main = 0.0f32;
    let mut total_fill_factor = 0.0f32;
    let mut fixed_sizes = vec![0.0f32; count];

    for (i, params) in children_params.iter().enumerate() {
        let (len_main, _) = match axis {
            Axis::Horizontal => (params.width, params.height),
            Axis::Vertical => (params.height, params.width),
        };
        let (margin_start, margin_end) = main_margins(axis, params);

        let content_size = match axis {
            Axis::Horizontal => children_content_sizes[i].0,
            Axis::Vertical => children_content_sizes[i].1,
        };

        let size_main = match len_main {
            Length::Fixed(v) => v,
            Length::Percent(p) => total_avail_main * p,
            Length::Content => content_size,
            Length::Fill(f) => {
                total_fill_factor += f;
                0.0
            }
        };

        fixed_sizes[i] = size_main;
        used_main += size_main + margin_start + margin_end;
    }

    used_main += spacing * (count as f32 - 1.0).max(0.0);

    let remaining_space = (total_avail_main - used_main).max(0.0);

    let mut current_main_pos = 0.0f32;

    for (i, params) in children_params.iter().enumerate() {
        let (len_main, len_cross) = match axis {
            Axis::Horizontal => (params.width, params.height),
            Axis::Vertical => (params.height, params.width),
        };
        let (margin_start, margin_end) = main_margins(axis, params);
        let (margin_cross_start, _margin_cross_end) = cross_margins(axis, params);

        let size_main = if let Length::Fill(f) = len_main {
            if total_fill_factor > 0.0 {
                (f / total_fill_factor) * remaining_space
            } else {
                0.0
            }
        } else {
            fixed_sizes[i]
        };

        let content_cross = match axis {
            Axis::Horizontal => children_content_sizes[i].1,
            Axis::Vertical => children_content_sizes[i].0,
        };

        let size_cross = match len_cross {
            Length::Fixed(v) => v,
            Length::Percent(p) => total_avail_cross * p,
            Length::Fill(_) => total_avail_cross,
            Length::Content => {
                if alignment == Alignment::Stretch {
                    total_avail_cross
                } else {
                    content_cross
                }
            }
        };

        let pos_cross = margin_cross_start
            + match alignment {
                Alignment::Start | Alignment::Stretch => 0.0,
                Alignment::Center => (total_avail_cross - size_cross).max(0.0) / 2.0,
                Alignment::End => (total_avail_cross - size_cross).max(0.0),
            };

        let item_main_start = current_main_pos + margin_start;

        match axis {
            Axis::Horizontal => {
                result_rects[i] = Rect::new(
                    available_space.x + item_main_start,
                    available_space.y + pos_cross,
                    size_main,
                    size_cross,
                );
            }
            Axis::Vertical => {
                result_rects[i] = Rect::new(
                    available_space.x + pos_cross,
                    available_space.y + item_main_start,
                    size_cross,
                    size_main,
                );
            }
        }

        current_main_pos += margin_start + size_main + margin_end + spacing;
    }

    result_rects
}

#[inline]
fn main_margins(axis: Axis, p: &LayoutParams) -> (f32, f32) {
    match axis {
        // margin: [Top, Right, Bottom, Left]
        Axis::Horizontal => (p.margin[3], p.margin[1]), // Left, Right
        Axis::Vertical => (p.margin[0], p.margin[2]),   // Top, Bottom
    }
}

#[inline]
fn cross_margins(axis: Axis, p: &LayoutParams) -> (f32, f32) {
    match axis {
        Axis::Horizontal => (p.margin[0], p.margin[2]), // Top, Bottom
        Axis::Vertical => (p.margin[3], p.margin[1]),   // Left, Right
    }
}
