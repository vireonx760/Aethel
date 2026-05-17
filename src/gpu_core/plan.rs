use crate::gpu_core::stats::GpuDrawStats;
use crate::gui::clip::ScissorRect;
use crate::gui::paint::RenderBatch;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawPacket {
    pub batch_index: usize,
    pub range: Range<u32>,
    pub shader_key: Option<u32>,
    pub scissor: Option<ScissorRect>,
}

#[derive(Debug, Default, Clone)]
pub struct DrawPlanner {
    packets: Vec<DrawPacket>,
    stats: GpuDrawStats,
}

impl DrawPlanner {
    pub fn new() -> Self {
        Self {
            packets: Vec::with_capacity(128),
            stats: GpuDrawStats::default(),
        }
    }

    pub fn plan(&mut self, batches: &[RenderBatch], width: u32, height: u32) -> &[DrawPacket] {
        self.packets.clear();
        self.stats = GpuDrawStats::default();

        let mut last_pipeline = None;
        let mut last_scissor = None;

        for (batch_index, batch) in batches.iter().enumerate() {
            if batch.range.is_empty() {
                self.stats.skipped_batches += 1;
                continue;
            }

            let scissor = match (batch.clip, batch.scissor(width, height)) {
                (Some(_), None) => {
                    self.stats.skipped_batches += 1;
                    continue;
                }
                (_, scissor) => scissor,
            };

            if last_pipeline != Some(batch.shader_key) {
                self.stats.pipeline_switches += 1;
                last_pipeline = Some(batch.shader_key);
            }
            if last_scissor != Some(scissor) {
                self.stats.scissor_changes += 1;
                last_scissor = Some(scissor);
            }

            self.packets.push(DrawPacket {
                batch_index,
                range: batch.range.clone(),
                shader_key: batch.shader_key,
                scissor,
            });
        }

        self.stats.draw_packets = self.packets.len();
        &self.packets
    }

    #[inline]
    pub fn packets(&self) -> &[DrawPacket] {
        &self.packets
    }

    #[inline]
    pub fn stats(&self) -> GpuDrawStats {
        self.stats
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.packets.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::clip::ClipRect;
    use crate::gui::paint::RenderLayer;

    #[test]
    fn planner_skips_offscreen_scissor_batches() {
        let mut planner = DrawPlanner::new();
        let batches = vec![RenderBatch {
            layer: RenderLayer::Regular,
            range: 0..1,
            clip: ClipRect::new(1000.0, 1000.0, 20.0, 20.0),
            shader_key: None,
        }];

        let packets = planner.plan(&batches, 100, 100);
        assert!(packets.is_empty());
        assert_eq!(planner.stats().skipped_batches, 1);
    }

    #[test]
    fn planner_counts_pipeline_switches() {
        let mut planner = DrawPlanner::new();
        let batches = vec![
            RenderBatch {
                layer: RenderLayer::Regular,
                range: 0..1,
                clip: None,
                shader_key: None,
            },
            RenderBatch {
                layer: RenderLayer::Regular,
                range: 1..2,
                clip: None,
                shader_key: Some(16),
            },
            RenderBatch {
                layer: RenderLayer::Regular,
                range: 2..3,
                clip: None,
                shader_key: Some(16),
            },
        ];

        let packets = planner.plan(&batches, 100, 100);
        assert_eq!(packets.len(), 3);
        assert_eq!(planner.stats().pipeline_switches, 2);
    }
}
