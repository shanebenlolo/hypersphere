use cgmath::Vector3;
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);
pub const IDENTITY_MATRIX_4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

pub struct CameraComponent {
    pub camera: Camera,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_controller: CameraController,
}

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    //     The build_view_projection_matrix is where the magic happens.

    // The view matrix moves the world to be at the position and rotation of the camera. It's essentially an inverse of whatever the transform matrix of the camera would be.
    // The proj matrix warps the scene to give the effect of depth. Without this, objects up close would be the same size as objects far away.
    // The coordinate system in Wgpu is based on DirectX and Metal's coordinate systems. That means that in normalized device coordinates (opens new window), the x-axis and y-axis are in the range of -1.0 to +1.0, and the z-axis is 0.0 to +1.0. The cgmath crate (as well as most game math crates) is built for OpenGL's coordinate system. This matrix will scale and translate our scene from OpenGL's coordinate system to WGPU's. We'll define it as follows.

    // this really needs to be refactored so we don't have to clone these matrices. Just combine the camera abd cameraUniform or something...
    pub fn build_view_projection_matrix(
        &self,
    ) -> (
        cgmath::Matrix4<f32>,
        cgmath::Matrix4<f32>,
        cgmath::Matrix4<f32>,
    ) {
        // you'll need to figure out how to fix the matrix for the web:
        // OPENGL_TO_WGPU_MATRIX * proj * view,
        // OPENGL_TO_WGPU_MATRIX * view.clone(),
        // OPENGL_TO_WGPU_MATRIX * proj.clone(),
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return (proj * view, view.clone(), proj.clone());
    }
}

pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}
impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn process_key_events(&mut self, event: &WindowEvent) -> bool {
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
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        if self.is_right_pressed {
            // Rotate the camera around the target point to the right
            let rotation_angle = cgmath::Rad(cgmath::Deg(self.speed / 100.0).0); // Convert to radians
            let rotation_matrix = cgmath::Matrix3::from_axis_angle(camera.up, -rotation_angle);
            let relative_position = camera.eye - camera.target;
            camera.eye = camera.target + rotation_matrix * relative_position;
        }
        if self.is_left_pressed {
            // Rotate the camera around the target point to the left
            let rotation_angle = cgmath::Rad(cgmath::Deg(self.speed / 100.0).0); // Convert to radians
            let rotation_matrix = cgmath::Matrix3::from_axis_angle(camera.up, rotation_angle);
            let relative_position = camera.eye - camera.target;
            camera.eye = camera.target + rotation_matrix * relative_position;
        }
    }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub view_proj_matrix: [[f32; 4]; 4],
    pub view_matrix: [[f32; 4]; 4],
    pub proj_matrix: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj_matrix: IDENTITY_MATRIX_4,
            view_matrix: IDENTITY_MATRIX_4,
            proj_matrix: IDENTITY_MATRIX_4,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        let (view_proj_matrix, view_matrix, proj_matrix) = camera.build_view_projection_matrix();
        self.view_proj_matrix = view_proj_matrix.into();
        self.view_matrix = view_matrix.into();
        self.proj_matrix = proj_matrix.into();
    }
}
