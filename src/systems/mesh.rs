use wgpu::util::DeviceExt;

use crate::components::mesh::{BillboardVertex, Vertex};

pub struct MeshSystem {}

impl MeshSystem {
    pub fn create_model_matrix_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        model_matrix: [[f32; 4]; 4],
    ) -> wgpu::BindGroup {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Buffer"),
            contents: bytemuck::cast_slice(&[model_matrix]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
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
    pub fn create_vertex_buffer<T>(device: &wgpu::Device, data: &[T]) -> wgpu::Buffer
    where
        T: bytemuck::Pod,
    {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn create_index_buffer(device: &wgpu::Device, data: &[u32]) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

    pub fn generate_rectangle_mesh(size: (f32, f32)) -> (Vec<BillboardVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let (width, height) = size;
        let half_width = width / 2.0;
        let half_height = height / 2.0;

        // Define four corners of the rectangle
        // We'll later rotate these to match the specified latitude and longitude
        vertices.push(BillboardVertex {
            position: [-half_width, -half_height, 0.0], // Bottom left
            tex_coords: [0.0, 0.0],                     // Texture coordinates
        });
        vertices.push(BillboardVertex {
            position: [half_width, -half_height, 0.0], // Bottom right
            tex_coords: [1.0, 0.0],                    // Texture coordinates
        });
        vertices.push(BillboardVertex {
            position: [half_width, half_height, 0.0], // Top right
            tex_coords: [1.0, 1.0],                   // Texture coordinates
        });
        vertices.push(BillboardVertex {
            position: [-half_width, half_height, 0.0], // Top left
            tex_coords: [0.0, 1.0],                    // Texture coordinates
        });

        // Define indices for two triangles that make up the rectangle
        indices.push(0);
        indices.push(1);
        indices.push(2);
        indices.push(0);
        indices.push(2);
        indices.push(3);

        (vertices, indices)
    }

    pub fn degrees_to_radians(degrees: f32) -> f32 {
        degrees * (std::f32::consts::PI / 180.0)
    }

    pub fn lat_lon_to_cartesian(lat: f32, lon: f32, radius: f32) -> (f32, f32, f32) {
        let phi = (90.0 - lat).to_radians(); // Convert latitude to radians and start from the north pole
        let theta = lon.to_radians(); // Convert longitude to radians

        let x = radius * phi.sin() * theta.cos();
        let y = radius * phi.cos();
        let z = radius * phi.sin() * theta.sin();
        (x, y, z)
    }
}
