use iced_wgpu::wgpu;

pub fn create_buffer_with_data(
    device: &wgpu::Device,
    data: &[u8],
    usage: wgpu::BufferUsage,
) -> wgpu::Buffer {
    let mapped = device.create_buffer_mapped(data.len(), usage);
    mapped.data.copy_from_slice(data);
    mapped.finish()
}
