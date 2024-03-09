use bevy_ecs::component::Component;

#[derive(Component)]
pub struct RenderPipelineComponent {
    pub render_pipeline: wgpu::RenderPipeline,
    pub render_pipeline_layout: wgpu::PipelineLayout,
}

unsafe impl Send for RenderPipelineComponent {}
unsafe impl Sync for RenderPipelineComponent {}
