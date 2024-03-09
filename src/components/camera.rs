use crate::systems::camera::CameraSystem;
use bevy_ecs::component::Component;

#[rustfmt::skip]
pub const IDENTITY_MATRIX_4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

#[derive(Component)]
pub struct CameraComponent {
    pub camera: Camera,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_controller: CameraController,
}

unsafe impl Send for CameraComponent {}
unsafe impl Sync for CameraComponent {}

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

pub struct CameraController {
    pub speed: f32,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
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
}

// Needed to ensure rust compiled our data correctly for the shaders
// Needed to store the data in a buffer without compiler rearranging
#[repr(C)]
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

    pub fn update_view_proj(
        &mut self,
        eye: cgmath::Point3<f32>,
        target: cgmath::Point3<f32>,
        up: cgmath::Vector3<f32>,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) {
        let (view_proj_matrix, view_matrix, proj_matrix) =
            CameraSystem::build_view_projection_matrix(eye, target, up, aspect, fovy, znear, zfar);
        self.view_proj_matrix = view_proj_matrix.into();
        self.view_matrix = view_matrix.into();
        self.proj_matrix = proj_matrix.into();
    }
}
