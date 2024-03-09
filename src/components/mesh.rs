use bevy_ecs::component::Component;

#[derive(Component)]
pub struct MeshComponent {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub model_matrix_bind_group_layout: wgpu::BindGroupLayout,
    pub model_matrix_bind_group: wgpu::BindGroup,
    pub model_matrix_buffer: wgpu::Buffer,
    pub model_matrix: [[f32; 4]; 4],
}

unsafe impl Send for MeshComponent {}
unsafe impl Sync for MeshComponent {}

// Needed to ensure rust compiled our data correctly for the shaders
// Needed to store the data in a buffer without compiler rearranging
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// Needed to ensure rust compiled our data correctly for the shaders
// Needed to store the data in a buffer without compiler rearranging
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BillboardVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl BillboardVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
