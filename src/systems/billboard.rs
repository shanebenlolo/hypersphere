use crate::{
    components::{
        camera::CameraComponent, material::MaterialComponent, mesh::MeshComponent,
        render_pipelines::RenderPipelineComponent,
    },
    matrix4_to_array,
};

use super::{
    material::MaterialSystem, mesh::MeshSystem, render_pipelines::BillboardRenderPipelineSystem,
};

pub struct BillboardSystem {}

impl BillboardSystem {
    pub fn create_billboard_mesh(
        device: &wgpu::Device,
        size: f32,
        lat: f32,
        lon: f32,
        globe_radius: f32,
    ) -> MeshComponent {
        let (x, y, z) = MeshSystem::lat_lon_to_cartesian(lat, lon, globe_radius);
        let translation = cgmath::Vector3::new(x, y, z);
        let billboard_matrix = matrix4_to_array(cgmath::Matrix4::from_translation(translation));
        let billboard_matrix_bind_group_layout =
            MeshSystem::create_model_matrix_bind_group_layout(&device);
        let billboard_matrix_bind_group = MeshSystem::create_model_matrix_bind_group(
            &device,
            &billboard_matrix_bind_group_layout,
            billboard_matrix.clone(),
        );
        let (billboard_vertices_vec, billboard_indices_vec) =
            MeshSystem::generate_square_mesh(size);

        MeshComponent {
            vertex_buffer: MeshSystem::create_vertex_buffer(
                &device,
                &billboard_vertices_vec.as_slice(),
            ),
            index_buffer: MeshSystem::create_index_buffer(
                &device,
                &billboard_indices_vec.as_slice(),
            ),
            num_indices: billboard_indices_vec.len() as u32,
            model_matrix_bind_group_layout: billboard_matrix_bind_group_layout,
            model_matrix_bind_group: billboard_matrix_bind_group,
            model_matrix: billboard_matrix,
        }
    }

    pub fn create_billboard_material(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> MaterialComponent {
        let billboard_image_data = include_bytes!("../assets/billboard.png");
        let billboard_dyn_image = image::load_from_memory(billboard_image_data)
            .expect("Failed to load image from memory");
        let billboard_image_buffer = billboard_dyn_image.to_rgba8();
        let (materal_bind_group, material_bind_group_layout) =
            MaterialSystem::create_2d_texture(&device, &queue, billboard_image_buffer);
        MaterialComponent {
            bind_group: materal_bind_group,
            bind_group_layout: material_bind_group_layout,
            uniforms: None,
            shader: device
                .create_shader_module(wgpu::include_wgsl!("../shaders/billboard_shader.wgsl")),
        }
    }

    pub fn create_render_pipeline(
        device: &wgpu::Device,
        camera: &CameraComponent,
        material: &MaterialComponent,
        mesh: &MeshComponent,
        texture_format: &wgpu::TextureFormat,
    ) -> RenderPipelineComponent {
        let billboard_pipeline_layouts: &[&wgpu::BindGroupLayout] = &[
            &camera.camera_bind_group_layout,
            &material.bind_group_layout,
            &mesh.model_matrix_bind_group_layout,
        ];
        let billboard_render_pipeline_layout =
            BillboardRenderPipelineSystem::layout_desc(&device, billboard_pipeline_layouts);
        let billboard_render_pipeline = BillboardRenderPipelineSystem::pipeline_desc(
            &device,
            &billboard_render_pipeline_layout,
            &material.shader,
            texture_format,
        );
        RenderPipelineComponent {
            render_pipeline: billboard_render_pipeline,
            render_pipeline_layout: billboard_render_pipeline_layout,
        }
    }
}
