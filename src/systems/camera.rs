use bevy_ecs::world::Mut;
use wgpu::util::DeviceExt;
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::components::camera::{Camera, CameraComponent, CameraController, CameraUniform};

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
            eye: (0.0, 0.0, 2_000_000.0).into(),
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
        camera_uniform.update_view_proj(
            camera.eye,
            camera.target,
            camera.up,
            camera.aspect,
            camera.fovy,
            camera.znear,
            camera.zfar,
        );

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

    pub fn update_camera(queue: &wgpu::Queue, mut cam_component: Mut<'_, CameraComponent>) {
        use cgmath::InnerSpace;
        let forward = cam_component.camera.target - cam_component.camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();
        let speed = cam_component.camera_controller.speed.clone();

        let eye = cam_component.camera.eye.clone();
        let target = cam_component.camera.target.clone();
        let up = cam_component.camera.up.clone();
        let aspect = cam_component.camera.aspect.clone();
        let fovy = cam_component.camera.fovy.clone();
        let znear = cam_component.camera.znear.clone();
        let zfar = cam_component.camera.zfar.clone();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if cam_component.camera_controller.is_forward_pressed
            && forward_mag > cam_component.camera_controller.speed
        {
            cam_component.camera.eye += forward_norm * speed
        }
        if cam_component.camera_controller.is_backward_pressed {
            cam_component.camera.eye -= forward_norm * speed
        }

        if cam_component.camera_controller.is_right_pressed {
            // Rotate the camera around the target point to the right
            let rotation_angle =
                cgmath::Rad(cgmath::Deg(cam_component.camera_controller.speed / 5000000.0).0); // Convert to radians
            let rotation_matrix =
                cgmath::Matrix3::from_axis_angle(cam_component.camera.up, -rotation_angle);
            let relative_position = cam_component.camera.eye - cam_component.camera.target;
            cam_component.camera.eye =
                cam_component.camera.target + rotation_matrix * relative_position;
        }
        if cam_component.camera_controller.is_left_pressed {
            // Rotate the camera around the target point to the left
            let rotation_angle =
                cgmath::Rad(cgmath::Deg(cam_component.camera_controller.speed / 5000000.0).0); // Convert to radians
            let rotation_matrix =
                cgmath::Matrix3::from_axis_angle(cam_component.camera.up, rotation_angle);
            let relative_position = cam_component.camera.eye - cam_component.camera.target;
            cam_component.camera.eye =
                cam_component.camera.target + rotation_matrix * relative_position;
        }

        cam_component
            .camera_uniform
            .update_view_proj(eye, target, up, aspect, fovy, znear, zfar);
        queue.write_buffer(
            &cam_component.camera_buffer,
            0,
            bytemuck::cast_slice(&[cam_component.camera_uniform]),
        );
    }

    // The view matrix moves the world to be at the position and rotation of the camera. It's an inverse of whatever the transform matrix of the camera would be.
    // The proj matrix warps the scene to give the effect of depth. Without this, objects up close would be the same size as objects far away.
    // The coordinate system in Wgpu is based on DirectX and Metal's coordinate systems. That means that in normalized device coordinates (opens new window),
    // the x-axis and y-axis are in the range of -1.0 to +1.0, and the z-axis is 0.0 to +1.0. The cgmath crate (as well as most game math crates) is built for OpenGL's coordinate system.
    // This matrix will scale and translate our scene from OpenGL's coordinate system to WGPU's. We'll define it as follows.
    pub fn build_view_projection_matrix(
        eye: cgmath::Point3<f32>,
        target: cgmath::Point3<f32>,
        up: cgmath::Vector3<f32>,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> (
        cgmath::Matrix4<f32>,
        cgmath::Matrix4<f32>,
        cgmath::Matrix4<f32>,
    ) {
        #[cfg(target_arch = "wasm32")]
        // you'll need to figure out how to fix the matrix for the web for proper ray-casting:
        // OPENGL_TO_WGPU_MATRIX * proj * view,
        // OPENGL_TO_WGPU_MATRIX * view.clone(),
        // OPENGL_TO_WGPU_MATRIX * proj.clone(),
        // ^^ does not work. cannot figure out solution...
        {
            let view = cgmath::Matrix4::look_at_rh(eye, target, up);
            let proj = cgmath::perspective(cgmath::Deg(fovy), aspect, znear, zfar);
            return (proj * view, view.clone(), proj.clone());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let view = cgmath::Matrix4::look_at_rh(eye, target, up);
            let proj = cgmath::perspective(cgmath::Deg(fovy), aspect, znear, zfar);
            return (proj * view, view.clone(), proj.clone());
        }
    }
}
