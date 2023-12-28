use image::{ImageBuffer, Rgba};

pub struct MaterialSystem {}

impl MaterialSystem {
    pub fn cube_map_buffer_from_urls(urls: Vec<&[u8]>) -> Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        urls.iter()
            .map(|&cube_bytes| {
                let cube_image =
                    image::load_from_memory(cube_bytes).expect("Failed to load image from memory");
                cube_image.to_rgba8()
            })
            .collect()
    }

    pub fn create_cube_map_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image_data: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    ) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let dimensions = image_data[0].dimensions();
        let face_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1, // Single layer for each face
        };

        let cube_map_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 6, // 6 layers for cube texture
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Cube Map Texture"),
            view_formats: &[],
        });

        // Copy each face into the cube texture
        for (i, data) in image_data.iter().enumerate() {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &cube_map_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32, // Specifies which layer of the cube map to copy to
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&data),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some((dimensions.0 * 4) as u32), // 4 bytes per pixel for RGBA
                    rows_per_image: Some(dimensions.1 as u32),
                },
                face_size,
            );
        }

        let cube_map_material_view = cube_map_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Cube Map Material View"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(6),
        });

        let cube_map_material_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Cube Map Material Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let cube_map_material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cube Map Material bind group layout"),
                entries: &[
                    // texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let cube_map_material_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &cube_map_material_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&cube_map_material_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&cube_map_material_sampler),
                },
            ],
            label: Some("Cube Map Material bind group"),
        });
        (
            cube_map_material_bind_group,
            cube_map_material_bind_group_layout,
        )
    }

    pub fn create_2d_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image_data: ImageBuffer<Rgba<u8>, Vec<u8>>,
    ) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let dimensions = image_data.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1, // Single layer for 2D texture
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("2D Texture"),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&image_data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some((dimensions.0 * 4) as u32), // 4 bytes per pixel for RGBA
                rows_per_image: Some(dimensions.1 as u32),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("2D Texture View"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: None,
        });

        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("2D Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("2D Texture bind group layout"),
                entries: &[
                    // Texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler),
                },
            ],
            label: Some("2D Texture bind group"),
        });

        (texture_bind_group, texture_bind_group_layout)
    }
}
