use iced_wgpu::wgpu;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub mod texture;
pub mod light;
pub mod mesh;
pub mod instance;

pub fn create_buffer_with_data(
    device: &wgpu::Device,
    data: &[u8],
    usage: wgpu::BufferUsage,
) -> wgpu::Buffer {
    let descriptor = BufferInitDescriptor {
        label: Some("descriptor"),
        contents: data,
        usage,
    };
    device.create_buffer_init(&descriptor)
}

pub struct BufferDimensions {
    pub width: usize,
    pub height: usize,
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,
}

impl BufferDimensions {
    pub fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = std::mem::size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}

