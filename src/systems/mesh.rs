use wgpu::util::DeviceExt;

use crate::components::mesh::{Vertex, R, TOTAL};

pub struct MeshSystem<'a> {
    device: &'a wgpu::Device,
}

impl<'a> MeshSystem<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self { device }
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

    fn create_vertex(lat: f32, lon: f32) -> Vertex {
        let x = R * lat.sin() * lon.cos();
        let y = R * lat.sin() * lon.sin();
        let z = R * lat.cos();
        Vertex {
            position: [x, y, z],
        }
    }

    pub fn generate_sphere_mesh() -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Create vertices
        for i in 0..=TOTAL {
            let lat = MeshSystem::map(i, 0, TOTAL, 0.0, std::f32::consts::PI);
            for j in 0..=TOTAL {
                let lon = MeshSystem::map(j, 0, TOTAL, 0.0, 2.0 * std::f32::consts::PI);
                vertices.push(MeshSystem::create_vertex(lat, lon));
            }
        }

        // Create indices for triangle strips
        for i in 0..TOTAL {
            for j in 0..=TOTAL {
                indices.push(i * (TOTAL + 1) + j); // Vertex in current row
                indices.push((i + 1) * (TOTAL + 1) + j); // Vertex in next row
            }

            if i != TOTAL - 1 {
                // Degenerate triangle to stitch strips together: repeat the last vertex of the current strip
                // and the first vertex of the next strip
                indices.push((i + 1) * (TOTAL + 1) + TOTAL);
                indices.push((i + 1) * (TOTAL + 1));
            }
        }

        (vertices, indices)
    }
}
