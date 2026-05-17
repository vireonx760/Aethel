use crate::core::scratch::FrameScratchStats;
use crate::gui::command::CommandStats;
use crate::gui::paint::PaintStats;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameStats {
    pub frame_index: u64,
    pub instances: usize,
    pub regular_instances: usize,
    pub overlay_instances: usize,
    pub batches: usize,
    pub text_buffers: usize,
    pub text_areas: usize,
    pub paint: PaintStats,
    pub commands: CommandStats,
    pub scratch: FrameScratchStats,
}

impl FrameStats {
    #[inline]
    pub fn next_frame(&mut self) {
        self.frame_index += 1;
        self.instances = 0;
        self.regular_instances = 0;
        self.overlay_instances = 0;
        self.batches = 0;
        self.text_buffers = 0;
        self.text_areas = 0;
    }

    #[inline]
    pub fn record_paint(
        &mut self,
        instances: usize,
        regular_count: usize,
        batches: usize,
        paint: &PaintStats,
    ) {
        self.instances = instances;
        self.regular_instances = regular_count.min(instances);
        self.overlay_instances = instances.saturating_sub(self.regular_instances);
        self.batches = batches;
        self.paint = paint.clone();
    }

    #[inline]
    pub fn record_text(&mut self, buffers: usize, areas: usize) {
        self.text_buffers = buffers;
        self.text_areas = areas;
    }

    #[inline]
    pub fn record_commands(&mut self, commands: &CommandStats) {
        self.commands = commands.clone();
    }

    #[inline]
    pub fn record_scratch(&mut self, scratch: FrameScratchStats) {
        self.scratch = scratch;
    }
}
