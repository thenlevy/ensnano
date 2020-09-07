use crate::consts::*;
use crate::utils::create_buffer_with_data;
use iced_wgpu::wgpu;
use std::f32::consts::PI;
use std::ops::Range;
use wgpu::Device;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MeshVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

unsafe impl bytemuck::Pod for MeshVertex {}
unsafe impl bytemuck::Zeroable for MeshVertex {}

impl Vertex for MeshVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: VERTEX_POSITION_ADRESS,
                    format: wgpu::VertexFormat::Float3,
                },
                // Normal
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: VERTEX_NORMAL_ADRESS,
                    format: wgpu::VertexFormat::Float3,
                },
            ],
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

impl Mesh {
    /// Represents a tube with radius of BOUND_RADIUS, height of BOUND_LENGTH, and centered at
    /// (0, 0, 0) pointing to (1, 0, 0).
    pub fn tube(device: &Device) -> Self {
        let vertices = (0..(2 * NB_RAY_TUBE))
            .map(|i| {
                let point = i / 2;
                let side = if i % 2 == 0 { -1. } else { 1. };
                let theta = (point as f32) * 2. * PI / NB_RAY_TUBE as f32;
                let position = [
                    side * BOUND_LENGTH / 2.,
                    theta.sin() * BOUND_RADIUS,
                    theta.cos() * BOUND_RADIUS,
                ];

                let normal = [0., theta.sin(), theta.cos()];
                MeshVertex { position, normal }
            })
            .collect::<Vec<_>>();
        let vertex_buffer = create_buffer_with_data(
            device,
            bytemuck::cast_slice(vertices.as_slice()),
            wgpu::BufferUsage::VERTEX,
        );

        let mut indices: Vec<_> = (0u16..(2 * NB_RAY_TUBE as u16)).collect();
        indices.push(0);
        indices.push(1);
        let index_buffer = create_buffer_with_data(
            device,
            bytemuck::cast_slice(indices.as_slice()),
            wgpu::BufferUsage::INDEX,
        );

        let num_elements = indices.len() as u32;
        Self {
            vertex_buffer,
            index_buffer,
            num_elements,
        }
    }

    pub fn sphere(device: &Device) -> Self {
        let mut vertices = Vec::new();

        let stack_step = PI / NB_STACK_SPHERE as f32;
        let sector_step = 2. * PI / NB_SECTOR_SPHERE as f32;
        for i in 0..=NB_STACK_SPHERE {
            // 0..=x means that x is included
            let stack_angle = PI / 2. - (i as f32) * stack_step;
            let xy = SPHERE_RADIUS * stack_angle.cos();
            let z = SPHERE_RADIUS * stack_angle.sin();

            for j in 0..=NB_SECTOR_SPHERE {
                let sector_angle = j as f32 * sector_step;

                let x = xy * sector_angle.cos();
                let y = xy * sector_angle.sin();
                let position = [x, y, z];
                let normal = [x, y, z];

                vertices.push(MeshVertex { position, normal })
            }
        }
        let vertex_buffer = create_buffer_with_data(
            device,
            bytemuck::cast_slice(vertices.as_slice()),
            wgpu::BufferUsage::VERTEX,
        );

        let mut indices = Vec::new();

        for i in 0..NB_STACK_SPHERE {
            let mut k1: u16 = i * (NB_SECTOR_SPHERE + 1); // begining of ith stack
            let mut k2: u16 = k1 + NB_SECTOR_SPHERE + 1; // begining of (i + 1)th stack

            for _ in 0..NB_SECTOR_SPHERE {
                if i > 0 {
                    indices.push(k1);
                    indices.push(k2);
                    indices.push(k1 + 1);
                }

                if i < NB_STACK_SPHERE - 1 {
                    indices.push(k1 + 1);
                    indices.push(k2);
                    indices.push(k2 + 1);
                }
                k1 += 1;
                k2 += 1;
            }
        }
        let index_buffer = create_buffer_with_data(
            device,
            bytemuck::cast_slice(indices.as_slice()),
            wgpu::BufferUsage::INDEX,
        );

        let num_elements = indices.len() as u32;
        Self {
            vertex_buffer,
            index_buffer,
            num_elements,
        }
    }
}

pub trait DrawModel<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        viewer: &'b wgpu::BindGroup,
        instance_bg: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        viewer: &'b wgpu::BindGroup,
        instances_bg: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        viewer: &'b wgpu::BindGroup,
        instances_bg: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, 0..1, viewer, instances_bg, light);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        viewer: &'b wgpu::BindGroup,
        instances_bg: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffers(0, &[(&mesh.vertex_buffer, 0)]);
        self.set_index_buffer(&mesh.index_buffer, 0);
        self.set_bind_group(VIEWER_BINDING_ID, &viewer, &[]);
        self.set_bind_group(INSTANCES_BINDING_ID, &instances_bg, &[]);
        self.set_bind_group(LIGHT_BINDING_ID, &light, &[]);
        //self.draw_indexed(0..mesh.num_elements, 0, instances);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}
