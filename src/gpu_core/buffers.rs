use crate::core::renderer::WidgetInstance;
use crate::gpu_core::stats::GpuUploadStats;
use std::mem;
use wgpu::{Buffer, BufferAddress, BufferDescriptor, BufferUsages, Device, Queue};

pub const DEFAULT_INSTANCE_CAPACITY: u64 = 1024;

pub struct InstanceBufferArena {
    buffer: Buffer,
    capacity: u64,
    growths: u64,
    high_watermark: u64,
}

impl InstanceBufferArena {
    pub fn new(device: &Device, capacity: u64) -> Self {
        let capacity = capacity.max(1);
        Self {
            buffer: create_instance_buffer(device, capacity),
            capacity,
            growths: 0,
            high_watermark: 0,
        }
    }

    #[inline]
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    #[inline]
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    #[inline]
    pub fn high_watermark(&self) -> u64 {
        self.high_watermark
    }

    pub fn upload(
        &mut self,
        device: &Device,
        queue: &Queue,
        instances: &[WidgetInstance],
    ) -> GpuUploadStats {
        let needed = instances.len() as u64;
        self.high_watermark = self.high_watermark.max(needed);

        if needed > self.capacity {
            let new_capacity = next_capacity(self.capacity, needed);
            self.buffer = create_instance_buffer(device, new_capacity);
            self.capacity = new_capacity;
            self.growths += 1;
        }

        let bytes = bytemuck::cast_slice(instances);
        if !bytes.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytes);
        }

        GpuUploadStats {
            instance_count: instances.len(),
            instance_capacity: self.capacity,
            bytes_uploaded: bytes.len() as u64,
            buffer_growths: self.growths,
        }
    }
}

fn create_instance_buffer(device: &Device, capacity: u64) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("AethelGUI Instance Arena"),
        size: capacity * mem::size_of::<WidgetInstance>() as BufferAddress,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn next_capacity(current: u64, needed: u64) -> u64 {
    let doubled = current.saturating_mul(2).max(DEFAULT_INSTANCE_CAPACITY);
    doubled.max(needed).next_power_of_two()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_capacity_grows_to_power_of_two() {
        assert_eq!(next_capacity(1024, 1025), 2048);
        assert_eq!(next_capacity(1024, 5000), 8192);
    }
}
