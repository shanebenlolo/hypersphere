use bevy_ecs::component::Component;

use super::{
    material::MaterialComponent, mesh::MeshComponent, render_pipelines::RenderPipelineComponent,
};

#[derive(Component)]
pub struct MoonComponent {
    pub mesh_component: MeshComponent,
    pub material_component: MaterialComponent,
    pub render_pipeline_component: RenderPipelineComponent,
}

unsafe impl Send for MoonComponent {}
unsafe impl Sync for MoonComponent {}
