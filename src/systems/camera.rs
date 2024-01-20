use wgpu::util::DeviceExt;
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::components::camera::{Camera, CameraController, CameraUniform};

pub struct CameraSystem {}

impl CameraSystem {
    pub fn create_camera(
        device: &wgpu::Device,
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
        let camera = Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 0.0, 20_000.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: screen_width as f32 / screen_height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100_000_000.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = CameraSystem::create_uniform_buffer(device, &camera_uniform);
        let camera_bind_group_layout =
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
            });

        let camera_bind_group = CameraSystem::create_uniform_bind_group(
            device,
            &camera_buffer,
            &camera_bind_group_layout,
        );
        let camera_controller = CameraController::new(50000.0);

        (
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            camera_controller,
        )
    }

    fn create_uniform_buffer<T: bytemuck::Pod>(device: &wgpu::Device, data: &T) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Component Uniform Buffer"),
            contents: bytemuck::bytes_of(data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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

    pub fn process_key_events(cam_controller: &mut CameraController, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        cam_controller.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        cam_controller.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        cam_controller.is_backward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        cam_controller.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(cam_controller: &mut CameraController, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if cam_controller.is_forward_pressed && forward_mag > cam_controller.speed {
            camera.eye += forward_norm * cam_controller.speed;
        }
        if cam_controller.is_backward_pressed {
            camera.eye -= forward_norm * cam_controller.speed;
        }

        if cam_controller.is_right_pressed {
            // Rotate the camera around the target point to the right
            let rotation_angle = cgmath::Rad(cgmath::Deg(cam_controller.speed / 10000.0).0); // Convert to radians
            let rotation_matrix = cgmath::Matrix3::from_axis_angle(camera.up, -rotation_angle);
            let relative_position = camera.eye - camera.target;
            camera.eye = camera.target + rotation_matrix * relative_position;
        }
        if cam_controller.is_left_pressed {
            // Rotate the camera around the target point to the left
            let rotation_angle = cgmath::Rad(cgmath::Deg(cam_controller.speed / 10000.0).0); // Convert to radians
            let rotation_matrix = cgmath::Matrix3::from_axis_angle(camera.up, rotation_angle);
            let relative_position = camera.eye - camera.target;
            camera.eye = camera.target + rotation_matrix * relative_position;
        }
    }
}
