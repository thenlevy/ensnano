use iced_wgpu::wgpu;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub fn create_buffer_with_data(
    device: &wgpu::Device,
    data: &[u8],
    usage: wgpu::BufferUsage,
) -> wgpu::Buffer {
    let descriptor = BufferInitDescriptor {
        label: Some("descriptor"),
        contents: data,
        usage
    };
    device.create_buffer_init(&descriptor)
}
