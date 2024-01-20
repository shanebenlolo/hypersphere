use super::{
    material::MaterialComponent, mesh::MeshComponent, render_pipelines::RenderPipelineComponent,
};

pub struct MoonComponent {
    pub mesh_component: MeshComponent,
    pub material_component: MaterialComponent,
    pub render_pipeline_component: RenderPipelineComponent,
}
