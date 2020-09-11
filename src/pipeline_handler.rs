use crate::{camera, instance, light, mesh, texture, uniforms, utils};
use camera::{Camera, Projection};
use iced_wgpu::wgpu;
use instance::{Instance, InstanceRaw};
use light::create_light;
use mesh::{DrawModel, Mesh, Vertex};
use texture::Texture;
use uniforms::Uniforms;
use utils::create_buffer_with_data;
use wgpu::{BindGroup, BindGroupLayout, Device, RenderPass, RenderPipeline, StencilStateDescriptor, include_spirv};

/// A structure that can create a pipeline which will draw several instances of the same
/// mesh.
pub struct PipelineHandler {
    mesh: Mesh,
    instances: Vec<Instance>,
    viewer_data: Uniforms,
    bind_groups: BindGroups,
    vertex_module: wgpu::ShaderModule,
    fragment_module: wgpu::ShaderModule,
    primitive_topology: wgpu::PrimitiveTopology,
    flavour: Flavour,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Flavour {
    Real,
    Fake,
    Selected,
}

impl PipelineHandler {
    pub fn new(
        device: &Device,
        mesh: Mesh,
        instances: Vec<Instance>,
        camera: &Camera,
        projection: &Projection,
        primitive_topology: wgpu::PrimitiveTopology,
        flavour: Flavour,
    ) -> Self {
        let instances_data: Vec<_> = instances.iter().map(|i| i.to_raw()).collect();
        let (instances_bg, instances_layout) = create_instances_bind_group(device, &instances_data);

        let mut viewer_data = Uniforms::new();
        viewer_data.update_view_proj(camera, projection);
        let (viewer, viewer_layout) = create_viewer_bind_group(device, &viewer_data);

        let (light, light_layout) = create_light(device);

        let bind_groups = BindGroups {
            instances: instances_bg,
            instances_layout,
            viewer,
            viewer_layout,
            light,
            light_layout,
        };

        let vertex_module = 
            device.create_shader_module(include_spirv!("vert.spv"));
        let fragment_module = match flavour {
            Flavour::Real => device
                .create_shader_module(include_spirv!("frag.spv")),
            Flavour::Fake => device.create_shader_module(include_spirv!("fake_color.spv")),
            Flavour::Selected => device.create_shader_module(include_spirv!("selected_frag.spv")),
        };

        Self {
            mesh,
            instances,
            viewer_data,
            bind_groups,
            vertex_module,
            fragment_module,
            primitive_topology,
            flavour,
        }
    }

    pub fn update_viewer(&mut self, device: &Device, camera: &Camera, projection: &Projection) {
        self.viewer_data.update_view_proj(camera, projection);
        let (viewer, viewer_layout) = create_viewer_bind_group(device, &self.viewer_data);
        self.bind_groups.viewer = viewer;
        self.bind_groups.viewer_layout = viewer_layout;
    }

    pub fn update_instances(&mut self, device: &Device, instances: Vec<Instance>) {
        self.instances = instances;
        let instances_data: Vec<_> = self.instances.iter().map(|i| i.to_raw()).collect();
        let (instances_bg, instances_layout) = create_instances_bind_group(device, &instances_data);
        self.bind_groups.instances = instances_bg;
        self.bind_groups.instances_layout = instances_layout;
    }

    pub fn draw<'a, 'b: 'a>(&'b self, device: &Device, render_pass: &mut RenderPass<'a>) {
        let pipeline = self.create_pipeline(device);
        render_pass.set_pipeline(&pipeline);

        render_pass.draw_mesh_instanced(
            &self.mesh,
            0..self.instances.len() as u32,
            &self.bind_groups.viewer,
            &self.bind_groups.instances,
            &self.bind_groups.light,
        );
    }

    fn create_pipeline(&self, device: &Device) -> RenderPipeline {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &self.bind_groups.viewer_layout,
                    &self.bind_groups.instances_layout,
                    &self.bind_groups.light_layout,
                ],
                push_constant_ranges: &[],
                label: Some("render_pipeline_layout"),
            });

        let format = match self.flavour {
            Flavour::Fake => wgpu::TextureFormat::Bgra8Unorm,
            _ => wgpu::TextureFormat::Bgra8UnormSrgb,
        };

        let color_blend = if self.flavour != Flavour::Fake {
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,                 
            }
        } else {
            wgpu::BlendDescriptor::REPLACE
        };
        let alpha_blend = if self.flavour != Flavour::Fake {
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,                 
            }
        } else {
            wgpu::BlendDescriptor::REPLACE
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &self.vertex_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &self.fragment_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
                clamp_depth: false,
            }),
            primitive_topology: self.primitive_topology,
            color_states: &[wgpu::ColorStateDescriptor {
                format,
                color_blend,
                alpha_blend,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[mesh::MeshVertex::desc()],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
            label: Some("render pipeline") 
        })
    }
}

struct BindGroups {
    instances: BindGroup,
    instances_layout: BindGroupLayout,
    viewer: BindGroup,
    viewer_layout: BindGroupLayout,
    light: BindGroup,
    light_layout: BindGroupLayout,
}
/// Create the bind group for the model matrices.
fn create_instances_bind_group<I: bytemuck::Pod>(
    device: &Device,
    instances_data: &[I],
) -> (BindGroup, BindGroupLayout) {
    // create the model matrices and fill them in instance_buffer
    // instances_data has type &[InstanceRaw]
    let instance_buffer_size = instances_data.len() * std::mem::size_of::<InstanceRaw>();
    let instance_buffer = create_buffer_with_data(
        &device,
        bytemuck::cast_slice(instances_data),
        wgpu::BufferUsage::STORAGE,
    );

    let instance_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::StorageBuffer {
                    // We don't plan on changing the size of this buffer
                    dynamic: false,
                    // The shader is not allowed to modify it's contents
                    readonly: true,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("instance_bind_group_layout"),
        });

    let instance_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &instance_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(instance_buffer.slice(..)),
        }],
        label: Some("instance_bind_group"),
    });

    (instance_bind_group, instance_bind_group_layout)
}

/// Create the bind group for the perspective and view matrices.
fn create_viewer_bind_group<V: bytemuck::Pod>(
    device: &Device,
    viewer_data: &V,
) -> (BindGroup, BindGroupLayout) {
    let viewer_buffer = create_buffer_with_data(
        &device,
        bytemuck::cast_slice(&[*viewer_data]),
        wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );
    let uniform_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // perspective and view
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false, min_binding_size: None },
                    count: None,
                },
            ],
            label: Some("uniform_bind_group_layout"),
        });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uniform_bind_group_layout,
        entries: &[
            // perspective and view
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(viewer_buffer.slice(..)),
            },
        ],
        label: Some("uniform_bind_group"),
    });

    (uniform_bind_group, uniform_bind_group_layout)
}
