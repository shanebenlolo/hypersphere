use crate::components::mesh::Vertex;

pub struct GlobeRenderPipelineSystem<'a> {
    device: &'a wgpu::Device,
}

impl<'a> GlobeRenderPipelineSystem<'a> {
    pub fn new(device: &'a wgpu::Device) -> GlobeRenderPipelineSystem {
        Self { device }
    }

    pub fn layout_desc(
        &self,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::PipelineLayout {
        self.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Globe Render Pipeline Layout"),
                bind_group_layouts,
                push_constant_ranges: &[],
            })
    }

    pub fn pipeline_desc(
        &self,
        pipeline_layout: &wgpu::PipelineLayout,
        shader_module: &wgpu::ShaderModule,
        texture_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Globe Render Pipeline"),
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
                depth_stencil: None,
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

pub struct PointRenderPipelineSystem<'a> {
    device: &'a wgpu::Device,
}

impl<'a> PointRenderPipelineSystem<'a> {
    pub fn new(device: &'a wgpu::Device) -> PointRenderPipelineSystem {
        Self { device }
    }

    pub fn layout_desc(
        &self,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::PipelineLayout {
        self.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Point Render Pipeline Layout"),
                bind_group_layouts,
                push_constant_ranges: &[],
            })
    }

    pub fn pipeline_desc(
        &self,
        pipeline_layout: &wgpu::PipelineLayout,
        shader_module: &wgpu::ShaderModule,
        texture_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Point Render Pipeline"),
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
                    topology: wgpu::PrimitiveTopology::PointList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }
}
