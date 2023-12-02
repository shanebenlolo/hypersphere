use wgpu::util::DeviceExt;

use crate::{
    components::camera::{Camera, CameraController, CameraUniform},
    Uniform,
};

pub struct CameraSystem<'a> {
    device: &'a wgpu::Device,
}

impl<'a> CameraSystem<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self { device }
    }

    pub fn create_camera(
        &self,
        screen_width: u32,
        screen_height: u32,
    ) -> (
        Camera,
        CameraUniform,
        wgpu::Buffer,
        wgpu::BindGroup,
        wgpu::BindGroupLayout,
        CameraController,
    ) {
        // these always need to match or else
        let eye = [0.0, 1.0, 2.0, 0.0];
        let camera_eye = (0.0, 1.0, 2.0);

        let camera = Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: camera_eye.into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: screen_width as f32 / screen_height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new(eye);
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = CameraSystem::create_uniform_buffer(self.device, &camera_uniform);
        let camera_bind_group_layout = CameraSystem::create_uniform_bind_group_layout(self.device);
        let camera_bind_group = CameraSystem::create_uniform_bind_group(
            self.device,
            &camera_buffer,
            &camera_bind_group_layout,
        );
        let camera_controller = CameraController::new(10.0);

        (
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            camera_controller,
        )
    }
}

impl<'a> Uniform for CameraSystem<'a> {
    fn create_uniform_buffer<T: bytemuck::Pod>(device: &wgpu::Device, data: &T) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Component Uniform Buffer"),
            contents: bytemuck::bytes_of(data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
    fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Camera Component Uniform Bind Group Layout"),
        })
    }
    fn create_uniform_bind_group(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Camera Component Uniform Bind Group"),
        })
    }
}
