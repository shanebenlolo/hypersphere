use bevy_ecs::component::Component;

#[derive(Component)]
pub struct MaterialComponent {
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniforms: Option<Vec<[f32; 4]>>,
    pub shader: wgpu::ShaderModule,
}

unsafe impl Send for MaterialComponent {}
unsafe impl Sync for MaterialComponent {}
