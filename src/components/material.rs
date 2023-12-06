pub struct MaterialComponent {
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniforms: Vec<[f32; 4]>,
    pub shader: wgpu::ShaderModule,
}
