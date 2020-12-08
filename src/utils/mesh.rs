use crate::consts::*;
use iced_wgpu::wgpu;
use std::ops::Range;

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
        model_matrices: &'b wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        viewer: &'b wgpu::BindGroup,
        instances_bg: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
        model_matrices: &'b wgpu::BindGroup,
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
        model_matrices: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, 0..1, viewer, instances_bg, light, model_matrices);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        viewer: &'b wgpu::BindGroup,
        instances_bg: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
        model_matrices: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(VIEWER_BINDING_ID, &viewer, &[]);
        self.set_bind_group(INSTANCES_BINDING_ID, &instances_bg, &[]);
        self.set_bind_group(LIGHT_BINDING_ID, &light, &[]);
        self.set_bind_group(MODEL_BINDING_ID, &model_matrices, &[]);
        //self.draw_indexed(0..mesh.num_elements, 0, instances);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}
