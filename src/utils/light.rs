use crate::utils::create_buffer_with_data;
use iced_wgpu::wgpu;
use ultraviolet::Vec3;
use wgpu::{BindGroup, BindGroupLayout, Device};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Light {
    position: Vec3,
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding: u32,
    color: Vec3,
}

impl Light {
    #[allow(dead_code)]
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            position: position.into(),
            _padding: 0,
            color: color.into(),
        }
    }
}

#[allow(dead_code)]
pub fn create_light(device: &Device) -> (BindGroup, BindGroupLayout) {
    let light = Light::new([0.0, 0.0, 1000.0], [1.0, 1.0, 1.0]);

    let light_buffer = create_buffer_with_data(
        device,
        bytemuck::cast_slice(&[light]),
        wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let light_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("light_bind_group_layout"),
        });

    let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &light_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(light_buffer.slice(..)),
        }],
        label: Some("light bind group"),
    });
    (light_bind_group, light_bind_group_layout)
}

unsafe impl bytemuck::Zeroable for Light {}
unsafe impl bytemuck::Pod for Light {}
