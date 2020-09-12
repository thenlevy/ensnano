use iced_wgpu::wgpu;
use ultraviolet::{Bivec3, Mat3, Rotor3};
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

pub fn rotation_to_rotor(matrix: &Mat3) -> Rotor3 {
    let trace = matrix[0][0] + matrix [1][1] + matrix[2][2];

    // x -> yz
    // y -> xz
    // z -> xy
        if trace >= 0f32 {
            let s = (1f32 + trace).sqrt();
            let w = 0.5 * s;
            let s = 0.5 / s;
            let yz = (matrix[1][2] - matrix[2][1]) * s;
            let xz = (matrix[2][0] - matrix[0][2]) * s;
            let xy = (matrix[0][1] - matrix[1][0]) * s;
            Rotor3::new(w, Bivec3::new(xy, xz, yz))
        } else if (matrix[0][0] > matrix[1][1]) && (matrix[0][0] > matrix[2][2]) {
            let s = ((matrix[0][0] - matrix[1][1] - matrix[2][2]) + 1f32).sqrt();
            let yz = 0.5 * s;
            let s = 0.5 / s;
            let xz = (matrix[1][0] + matrix[0][1]) * s;
            let xy = (matrix[0][2] + matrix[2][0]) * s;
            let w = (matrix[1][2] - matrix[2][1]) * s;
            Rotor3::new(w, Bivec3::new(xy, xz, yz))
        } else if matrix[1][1] > matrix[2][2] {
            let s = ((matrix[1][1] - matrix[0][0] - matrix[2][2]) + 1f32).sqrt();
            let xz = 0.5 * s;
            let s = 0.5 / s;
            let xy = (matrix[2][1] + matrix[1][2]) * s;
            let yz = (matrix[1][0] + matrix[0][1]) * s;
            let w = (matrix[2][0] - matrix[0][2]) * s;
            Rotor3::new(w, Bivec3::new(xy, xz, yz))
        } else {
            let s = ((matrix[2][2] - matrix[0][0] - matrix[1][1]) + 1f32).sqrt();
            let xy = 0.5 * s;
            let s = 0.5 / s;
            let yz = (matrix[0][2] + matrix[2][0]) * s;
            let xz = (matrix[2][1] + matrix[1][2]) * s;
            let w = (matrix[0][1] - matrix[1][0]) * s;
            Rotor3::new(w, Bivec3::new(xy, xz, yz))
        }

}


