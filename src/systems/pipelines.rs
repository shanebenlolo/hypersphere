use crate::components::mesh::{BillboardVertex, Vertex};

pub struct EarthRenderPipelineSystem {}

impl EarthRenderPipelineSystem {
    pub fn layout_desc(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Earth Render Pipeline Layout"),
            bind_group_layouts,
            push_constant_ranges: &[],
        })
    }

    pub fn pipeline_desc(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        shader_module: &wgpu::ShaderModule,
        texture_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Earth Render Pipeline"),
            layout: Some(pipeline_layout),

            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },

            // frag is technically optional, so we
            // have to wrap it in Some
            fragment: Some(wgpu::FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                // every three vertices will correspond to one triangle.
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                // a triangle is facing forward if the vertices are arranged in
                // a counter-clockwise direction
                front_face: wgpu::FrontFace::Ccw,
                // Some(wgpu::Face::Back) makes it so if objects are not facing
                // camera they are not rendered
                cull_mode: Some(wgpu::Face::Back),
                // anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },

            // depth stencil only working on wasm :(
            #[cfg(not(target_arch = "wasm32"))]
            depth_stencil: None,
            #[cfg(target_arch = "wasm32")]
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),

            // Multisampling is a complex topic not being discussed
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                // related to anti-aliasing
                alpha_to_coverage_enabled: false,
            },
            // We won't be rendering to array textures so we can set this to None
            multiview: None,
        })
    }
}

pub struct BillboardRenderPipelineSystem {}

impl BillboardRenderPipelineSystem {
    pub fn layout_desc(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Earth Render Pipeline Layout"),
            bind_group_layouts,
            push_constant_ranges: &[],
        })
    }

    pub fn pipeline_desc(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        shader_module: &wgpu::ShaderModule,
        texture_format: &wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Billboard Render Pipeline"),
            layout: Some(pipeline_layout),

            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: "vs_main",
                buffers: &[BillboardVertex::desc()],
            },

            fragment: Some(wgpu::FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: *texture_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },

            // depth stencil only working on wasm :(
            #[cfg(not(target_arch = "wasm32"))]
            depth_stencil: None,
            #[cfg(target_arch = "wasm32")]
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }
}
