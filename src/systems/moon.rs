use bevy_ecs::world::Mut;
use cgmath::SquareMatrix;
use wgpu::util::DeviceExt;

use crate::{
    components::{
        camera::CameraComponent, material::MaterialComponent, mesh::MeshComponent,
        render_pipelines::RenderPipelineComponent,
    },
    matrix4_to_array, MOON_APPROX,
};

use super::{material::MaterialSystem, mesh::MeshSystem, pipelines::EarthRenderPipelineSystem};

pub struct MoonSystem {}

impl MoonSystem {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_format: wgpu::TextureFormat,
        camera_component: &CameraComponent,
    ) -> (MeshComponent, MaterialComponent, RenderPipelineComponent) {
        let mesh_component = MoonSystem::generate_mesh(device);
        let material_component = MoonSystem::generate_material(device, queue);
        let moon_render_pipeline_component = MoonSystem::generate_render_pipeline(
            device,
            texture_format,
            camera_component,
            &mesh_component,
            &material_component,
        );
        (
            mesh_component,
            material_component,
            moon_render_pipeline_component,
        )
    }

    pub fn generate_mesh(device: &wgpu::Device) -> MeshComponent {
        let moon_matrix = matrix4_to_array(cgmath::Matrix4::identity());

        let moon_matrix_bind_group_layout =
            MeshSystem::create_model_matrix_bind_group_layout(device);
        let moon_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Buffer"),
            contents: bytemuck::cast_slice(&[moon_matrix]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let moon_matrix_bind_group = MeshSystem::create_model_matrix_bind_group(
            &device,
            &moon_matrix_bind_group_layout,
            &moon_buffer,
        );

        // you need to fix this to work with both WGS84_A and WGS84_B
        let (moon_vertices_vec, moon_indices_vec) = MeshSystem::generate_sphere_mesh(MOON_APPROX);

        MeshComponent {
            vertex_buffer: MeshSystem::create_vertex_buffer(&device, &moon_vertices_vec.as_slice()),
            index_buffer: MeshSystem::create_index_buffer(&device, &moon_indices_vec.as_slice()),
            num_indices: moon_indices_vec.len() as u32,
            model_matrix_bind_group_layout: moon_matrix_bind_group_layout,
            model_matrix_bind_group: moon_matrix_bind_group,
            model_matrix_buffer: moon_buffer,
            model_matrix: moon_matrix,
        }
    }

    fn generate_material(device: &wgpu::Device, queue: &wgpu::Queue) -> MaterialComponent {
        let moon_image_data = MaterialSystem::cube_map_buffer_from_urls(vec![
            include_bytes!("../assets/1.png"),
            include_bytes!("../assets/2.png"),
            include_bytes!("../assets/3.png"),
            include_bytes!("../assets/4.png"),
            include_bytes!("../assets/5.png"),
            include_bytes!("../assets/6.png"),
        ]);
        let (materal_bind_group, material_bind_group_layout) =
            MaterialSystem::create_cube_map_texture(&device, &queue, moon_image_data.clone());
        MaterialComponent {
            bind_group: materal_bind_group,
            bind_group_layout: material_bind_group_layout,
            uniforms: None,
            shader: device.create_shader_module(wgpu::include_wgsl!("../shaders/moon_shader.wgsl")),
        }
    }

    fn generate_render_pipeline(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        camera_component: &CameraComponent,
        mesh_component: &MeshComponent,
        material_component: &MaterialComponent,
    ) -> RenderPipelineComponent {
        let moon_pipeline_layouts: &[&wgpu::BindGroupLayout] = &[
            &camera_component.camera_bind_group_layout,
            &material_component.bind_group_layout,
            &mesh_component.model_matrix_bind_group_layout,
        ];
        let moon_render_pipeline_layout =
            EarthRenderPipelineSystem::layout_desc(&device, moon_pipeline_layouts);
        let moon_render_pipeline = EarthRenderPipelineSystem::pipeline_desc(
            &device,
            &moon_render_pipeline_layout,
            &material_component.shader,
            texture_format,
        );
        RenderPipelineComponent {
            render_pipeline: moon_render_pipeline,
            render_pipeline_layout: moon_render_pipeline_layout,
        }
    }

    // this should go in translation system
    pub fn update_position(
        queue: &wgpu::Queue,
        moon_mesh: &Mut<'_, MeshComponent>,
        xyz: (f64, f64, f64),
    ) {
        let new_moon_matrix: [[f32; 4]; 4] = cgmath::Matrix4::from_translation(
            cgmath::Vector3::new(0.0 as f32, 0.0, xyz.2 as f32 / 100.0),
        )
        .into();

        queue.write_buffer(
            &moon_mesh.model_matrix_buffer,
            0,
            bytemuck::cast_slice(&[new_moon_matrix]),
        );
    }
}
