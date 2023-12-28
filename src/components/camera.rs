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
    // The view matrix moves the world to be at the position and rotation of the camera. It's an inverse of whatever the transform matrix of the camera would be.
    // The proj matrix warps the scene to give the effect of depth. Without this, objects up close would be the same size as objects far away.
    // The coordinate system in Wgpu is based on DirectX and Metal's coordinate systems. That means that in normalized device coordinates (opens new window),
    // the x-axis and y-axis are in the range of -1.0 to +1.0, and the z-axis is 0.0 to +1.0. The cgmath crate (as well as most game math crates) is built for OpenGL's coordinate system.
    // This matrix will scale and translate our scene from OpenGL's coordinate system to WGPU's. We'll define it as follows.
    pub fn build_view_projection_matrix(
        &self,
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
            let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
            let proj =
                cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
            return (proj * view, view.clone(), proj.clone());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
            let proj =
                cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
            return (proj * view, view.clone(), proj.clone());
        }
    }
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

    pub fn update_view_proj(&mut self, camera: &Camera) {
        let (view_proj_matrix, view_matrix, proj_matrix) = camera.build_view_projection_matrix();
        self.view_proj_matrix = view_proj_matrix.into();
        self.view_matrix = view_matrix.into();
        self.proj_matrix = proj_matrix.into();
    }
}
