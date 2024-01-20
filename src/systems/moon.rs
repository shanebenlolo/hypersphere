use cgmath::SquareMatrix;

use crate::{
    components::{
        camera::CameraComponent, material::MaterialComponent, mesh::MeshComponent,
        render_pipelines::RenderPipelineComponent,
    },
    matrix4_to_array, WGS84_A,
};

use super::{
    material::MaterialSystem, mesh::MeshSystem, render_pipelines::EarthRenderPipelineSystem,
};

pub struct EarthSystem {}

impl EarthSystem {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_format: wgpu::TextureFormat,
        camera_component: &CameraComponent,
    ) -> (MeshComponent, MaterialComponent, RenderPipelineComponent) {
        let mesh_component = EarthSystem::generate_mesh(device);
        let material_component = EarthSystem::generate_material(device, queue);
        let earth_render_pipeline_component = EarthSystem::generate_render_pipeline(
            device,
            texture_format,
            camera_component,
            &mesh_component,
            &material_component,
        );
        (
            mesh_component,
            material_component,
            earth_render_pipeline_component,
        )
    }

    fn generate_mesh(device: &wgpu::Device) -> MeshComponent {
        let earth_matrix = matrix4_to_array(cgmath::Matrix4::identity());
        let earth_matrix_bind_group_layout =
            MeshSystem::create_model_matrix_bind_group_layout(device);
        let earth_matrix_bind_group = MeshSystem::create_mode_matrix_bind_group(
            &device,
            &earth_matrix_bind_group_layout,
            earth_matrix.clone(),
        );

        // you need to fix this to work with both WGS84_A and WGS84_B
        let (earth_vertices_vec, earth_indices_vec) = MeshSystem::generate_sphere_mesh(WGS84_A);

        MeshComponent {
            vertex_buffer: MeshSystem::create_vertex_buffer(
                &device,
                &earth_vertices_vec.as_slice(),
            ),
            index_buffer: MeshSystem::create_index_buffer(&device, &earth_indices_vec.as_slice()),
            num_indices: earth_indices_vec.len() as u32,
            model_matrix_bind_group_layout: earth_matrix_bind_group_layout,
            model_matrix_bind_group: earth_matrix_bind_group,
            model_matrix: earth_matrix,
        }
    }

    fn generate_material(device: &wgpu::Device, queue: &wgpu::Queue) -> MaterialComponent {
        let earth_image_data = MaterialSystem::cube_map_buffer_from_urls(vec![
            include_bytes!("../assets/1.png"),
            include_bytes!("../assets/2.png"),
            include_bytes!("../assets/3.png"),
            include_bytes!("../assets/4.png"),
            include_bytes!("../assets/5.png"),
            include_bytes!("../assets/6.png"),
        ]);
        let (materal_bind_group, material_bind_group_layout) =
            MaterialSystem::create_cube_map_texture(&device, &queue, earth_image_data.clone());
        MaterialComponent {
            bind_group: materal_bind_group,
            bind_group_layout: material_bind_group_layout,
            uniforms: None,
            shader: device
                .create_shader_module(wgpu::include_wgsl!("../shaders/earth_shader.wgsl")),
        }
    }

    fn generate_render_pipeline(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        camera_component: &CameraComponent,
        mesh_component: &MeshComponent,
        material_component: &MaterialComponent,
    ) -> RenderPipelineComponent {
        let earth_pipeline_layouts: &[&wgpu::BindGroupLayout] = &[
            &camera_component.camera_bind_group_layout,
            &material_component.bind_group_layout,
            &mesh_component.model_matrix_bind_group_layout,
        ];
        let earth_render_pipeline_layout =
            EarthRenderPipelineSystem::layout_desc(&device, earth_pipeline_layouts);
        let earth_render_pipeline = EarthRenderPipelineSystem::pipeline_desc(
            &device,
            &earth_render_pipeline_layout,
            &material_component.shader,
            texture_format,
        );
        RenderPipelineComponent {
            render_pipeline: earth_render_pipeline,
            render_pipeline_layout: earth_render_pipeline_layout,
        }
    }
}
