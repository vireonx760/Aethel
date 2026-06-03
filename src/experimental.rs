pub mod custom_shader {
    pub use crate::gui::paint::{FIRST_CUSTOM_SHADER_MODE, ShaderMode};
    pub use crate::gui::shader::{CustomShader, CustomShaderRegistry};
}

pub mod gpu_stats {
    pub use crate::core::frame_stats::FrameStats;
    pub use crate::gpu_core::{GpuDrawStats, GpuStats, GpuUploadStats};
}
