// primitives/triangle.rs
use crate::core::renderer::WidgetInstance;

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub x3: f32,
    pub y3: f32,
    pub color: [f32; 4],
}

impl Triangle {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            x3,
            y3,
            color: [1.0; 4],
        }
    }

    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn to_outline_instances(&self) -> Vec<WidgetInstance> {
        vec![
            self.line_instance(self.x1, self.y1, self.x2, self.y2, 2.0),
            self.line_instance(self.x2, self.y2, self.x3, self.y3, 2.0),
            self.line_instance(self.x3, self.y3, self.x1, self.y1, 2.0),
        ]
    }

    pub fn to_fill_instances(&self) -> Vec<WidgetInstance> {
        self.to_fill_instances_with_step(2.0)
    }

    pub fn to_fill_instances_with_step(&self, step: f32) -> Vec<WidgetInstance> {
        let step = step.max(0.5);

        let mut verts = [(self.x1, self.y1), (self.x2, self.y2), (self.x3, self.y3)];
        verts.sort_by(|a, b| a.1.total_cmp(&b.1));
        let (xt, yt) = verts[0];
        let (xm, ym) = verts[1];
        let (xb, yb) = verts[2];

        let total_h = yb - yt;
        if total_h < 0.5 {
            return vec![];
        }

        let estimated = ((total_h / step) as usize + 2).max(1);
        let mut out = Vec::with_capacity(estimated);

        let mut y = yt;
        while y < yb {
            let y_end = (y + step).min(yb);

            let (xl, xr) = scanline_x(y, xt, yt, xm, ym, xb, yb);

            let x_left = xl.min(xr);
            let x_right = xl.max(xr);
            let w = x_right - x_left;
            let h = y_end - y;

            if w > 0.1 {
                out.push(WidgetInstance {
                    pos: [x_left, y],
                    size: [w, h],
                    color: self.color,
                    radius: 0.0,
                    ..Default::default()
                });
            }

            y = y_end;
        }

        out
    }

    pub fn to_stroke_instances(&self, width: f32) -> Vec<WidgetInstance> {
        vec![
            self.line_instance(self.x1, self.y1, self.x2, self.y2, width),
            self.line_instance(self.x2, self.y2, self.x3, self.y3, width),
            self.line_instance(self.x3, self.y3, self.x1, self.y1, width),
        ]
    }

    fn line_instance(&self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32) -> WidgetInstance {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);
        WidgetInstance {
            pos: [(x1 + x2) / 2.0 - len / 2.0, (y1 + y2) / 2.0 - width / 2.0],
            size: [len, width],
            color: self.color,
            radius: width / 2.0,
            rotation: angle,
            ..Default::default()
        }
    }
}

fn scanline_x(y: f32, xt: f32, yt: f32, xm: f32, ym: f32, xb: f32, yb: f32) -> (f32, f32) {
    let long = if (yb - yt).abs() > 1e-6 {
        xt + (y - yt) / (yb - yt) * (xb - xt)
    } else {
        xt
    };

    let short = if y < ym {
        if (ym - yt).abs() > 1e-6 {
            xt + (y - yt) / (ym - yt) * (xm - xt)
        } else {
            xt
        }
    } else {
        if (yb - ym).abs() > 1e-6 {
            xm + (y - ym) / (yb - ym) * (xb - xm)
        } else {
            xm
        }
    };

    (long, short)
}
