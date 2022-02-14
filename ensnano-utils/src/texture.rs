/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use crate::PhySize;
use iced_wgpu::wgpu;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: wgpu::Extent3d,
}

pub struct SampledTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bg_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float; // 1.

    pub fn create_depth_texture(device: &wgpu::Device, size: &PhySize, sample_count: u32) -> Self {
        let size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            label: Some("desc"),
        };
        let texture = device.create_texture(&desc);

        let view_descriptor = wgpu::TextureViewDescriptor {
            label: Some("view_descriptor"),
            format: Some(Self::DEPTH_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let view = texture.create_view(&view_descriptor);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // 4.
            label: Some("sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: Some(wgpu::CompareFunction::LessEqual), // 5.
            anisotropy_clamp: None,
            border_color: None,
        });

        Self {
            texture,
            view,
            sampler,
            size,
        }
    }

    /*
    pub fn clear(&mut self, queue: Rc<wgpu::Queue>) {
        let clear = vec![ 1f32; (self.size.width * self.size.height) as usize];

        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            bytemuck::cast_slice(clear.as_slice()),
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * self.size.width,
                rows_per_image: self.size.height,
            },
            self.size,
        );
    }*/

    pub fn create_msaa_texture(
        device: &wgpu::Device,
        size: &PhySize,
        sample_count: u32,
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureView {
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            label: Some("Multisampled frame descriptor"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}

impl SampledTexture {
    pub fn create_target_texture(device: &wgpu::Device, size: &PhySize) -> Self {
        let texture_descriptor = &wgpu::TextureDescriptor {
            label: Some("target texture descriptor"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        let texture = device.create_texture(texture_descriptor);
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&Default::default());
        let bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: false,
                        comparison: false,
                    },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
        Self {
            texture,
            view,
            sampler,
            bind_group,
            bg_layout,
        }
    }
}
