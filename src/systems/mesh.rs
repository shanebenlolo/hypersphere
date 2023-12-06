use wgpu::util::DeviceExt;

use crate::components::mesh::Vertex;

pub struct MeshSystem<'a> {
    device: &'a wgpu::Device,
}

impl<'a> MeshSystem<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self { device }
    }

    pub fn create_model_matrix_bind_group_layout(&self) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX, // Assuming the model matrix is used in the vertex shader
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Mesh Bind Group Layout"),
            })
    }

    pub fn create_mode_matrix_bind_group(
        &self,
        layout: &wgpu::BindGroupLayout,
        model_matrix: [[f32; 4]; 4],
    ) -> wgpu::BindGroup {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Buffer"),
                contents: bytemuck::cast_slice(&[model_matrix]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Mesh Bind Group"),
        })
    }

    // keeping these decoupled and not iterative until
    // we have more geometry
    pub fn create_vertex_buffer(&self, data: &[Vertex]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
    }

    pub fn create_index_buffer(&self, data: &[u32]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            })
    }

    fn map(value: u32, start1: u32, stop1: u32, start2: f32, stop2: f32) -> f32 {
        start2
            + (stop2 - start2) * ((value as f32 - start1 as f32) / (stop1 as f32 - start1 as f32))
    }

    fn create_vertex(radius: f32, lat: f32, lon: f32) -> Vertex {
        let x = radius * lat.sin() * lon.cos();
        let y = radius * lat.sin() * lon.sin();
        let z = radius * lat.cos();
        Vertex {
            position: [x, y, z],
        }
    }

    pub fn generate_sphere_mesh(radius: f32, tri_strips: u32) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Create vertices
        for i in 0..=tri_strips {
            let lat = MeshSystem::map(i, 0, tri_strips, 0.0, std::f32::consts::PI);
            for j in 0..=tri_strips {
                let lon = MeshSystem::map(j, 0, tri_strips, 0.0, 2.0 * std::f32::consts::PI);
                vertices.push(MeshSystem::create_vertex(radius, lat, lon));
            }
        }

        // Create indices for triangle strips
        for i in 0..tri_strips {
            for j in 0..=tri_strips {
                indices.push(i * (tri_strips + 1) + j); // Vertex in current row
                indices.push((i + 1) * (tri_strips + 1) + j); // Vertex in next row
            }

            if i != tri_strips - 1 {
                // Degenerate triangle to stitch strips together: repeat the last vertex of the current strip
                // and the first vertex of the next strip
                indices.push((i + 1) * (tri_strips + 1) + tri_strips);
                indices.push((i + 1) * (tri_strips + 1));
            }
        }

        (vertices, indices)
    }
}
