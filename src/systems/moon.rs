use anise::{almanac::Almanac, astro::Aberration, constants::frames, prelude::*};
use bevy_ecs::world::Mut;
use cgmath::SquareMatrix;
use chrono::{Duration, Utc};
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

    // orbit moon around earth
    pub fn update_position(
        queue: &wgpu::Queue,
        moon_mesh: &Mut<MeshComponent>,
        almanac: &Almanac,
        count: u64,
    ) {
        let now = Utc::now();
        let new_time = now + Duration::seconds(count as i64);
        let formatted_time = now.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string();
        let epoch = Epoch::from_str(&formatted_time).unwrap();

        let state = almanac
            .translate_from_to(
                frames::LUNA_J2000,  // Target
                frames::EARTH_J2000, // Observer
                epoch,
                Aberration::None,
            )
            .unwrap();
        let moon_position_velocity = state.to_cartesian_pos_vel();

        // Define the axial tilt in degrees
        let axial_tilt_degrees = -23.5f32; // Negative because we're tilting the moon's orbit
        let axial_tilt_radians = axial_tilt_degrees.to_radians();

        // Create a rotation matrix for the Earth's axial tilt
        let axial_tilt_matrix = cgmath::Matrix4::from_angle_x(cgmath::Deg(axial_tilt_radians));

        // Apply axial tilt transformation and then translation
        let position = cgmath::Vector3::new(
            moon_position_velocity[1] as f32,
            moon_position_velocity[2] as f32,
            moon_position_velocity[0] as f32,
        );

        // Convert Vector3 to Vector4 by adding a 'w' component of 1.0 for proper transformation
        let position_vec4 = cgmath::Vector4::new(position.x, position.y, position.z, 1.0);

        // Rotate the position vector by the axial tilt
        let tilted_position_vec4 = axial_tilt_matrix * position_vec4;

        // Convert back to Vector3 for translation (ignore the 'w' component)
        let tilted_position = cgmath::Vector3::new(
            tilted_position_vec4.x,
            tilted_position_vec4.y,
            tilted_position_vec4.z,
        );

        // Create the new model matrix with the tilted position
        let new_moon_matrix: [[f32; 4]; 4] =
            cgmath::Matrix4::from_translation(tilted_position).into();

        queue.write_buffer(
            &moon_mesh.model_matrix_buffer,
            0,
            bytemuck::cast_slice(&[new_moon_matrix]),
        );
    }
}
