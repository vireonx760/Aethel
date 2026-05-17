#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuUploadStats {
    pub instance_count: usize,
    pub instance_capacity: u64,
    pub bytes_uploaded: u64,
    pub buffer_growths: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuDrawStats {
    pub draw_packets: usize,
    pub skipped_batches: usize,
    pub pipeline_switches: usize,
    pub scissor_changes: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuStats {
    pub upload: GpuUploadStats,
    pub draw: GpuDrawStats,
    pub custom_pipelines: usize,
}
