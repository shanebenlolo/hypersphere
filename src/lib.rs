use std::collections::HashMap;

use image::{ImageBuffer, Rgba};
use wgpu::{util::DeviceExt, Surface};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, KeyboardEvent};

const IDENTITY_MATRIX_4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];
const R: f32 = 100.0;
const TOTAL: u32 = 90;

fn map(value: u32, start1: u32, stop1: u32, start2: f32, stop2: f32) -> f32 {
    start2 + (stop2 - start2) * ((value as f32 - start1 as f32) / (stop1 as f32 - start1 as f32))
}

fn create_vertex(lat: f32, lon: f32) -> Vertex {
    let x = R * lat.sin() * lon.cos();
    let y = R * lat.sin() * lon.sin();
    let z = R * lat.cos();
    Vertex {
        position: [x, y, z],
    }
}

fn generate_sphere_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Create vertices
    for i in 0..=TOTAL {
        let lat = map(i, 0, TOTAL, 0.0, std::f32::consts::PI);
        for j in 0..=TOTAL {
            let lon = map(j, 0, TOTAL, 0.0, 2.0 * std::f32::consts::PI);
            vertices.push(create_vertex(lat, lon));
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

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    color: [f32; 4], // Color as RGBA
    light_direction: [f32; 4],
}

const UNIFORMS: Uniforms = Uniforms {
    color: [0.0, 0.0, 0.0, 0.0],
    light_direction: [1.0, 0.0, 0.0, 0.0],
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);
struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}
impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    fn process_key_events(&mut self, event: &WindowEvent) -> bool {
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

    fn update_camera(&self, camera: &mut Camera) {
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

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the fowrard/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and eye so
            // that it doesn't change. The eye therefore still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: Matrix4x4,
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: Matrix4x4 {
                data: IDENTITY_MATRIX_4,
            },
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = Matrix4x4 {
            data: camera.build_view_projection_matrix().into(),
        }
    }
}
impl UniformData for CameraUniform {
    fn as_bytes(&self) -> Vec<u8> {
        self.view_proj.as_bytes()
    }
}

trait UniformData {
    fn as_bytes(&self) -> Vec<u8>;
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
struct Matrix4x4 {
    data: [[f32; 4]; 4],
}
impl UniformData for Matrix4x4 {
    fn as_bytes(&self) -> Vec<u8> {
        self.data
            .iter()
            .flat_map(|inner| inner.iter().flat_map(|&f| f.to_ne_bytes()))
            .collect()
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Vec4 {
    data: [f32; 4],
}
impl UniformData for Vec4 {
    fn as_bytes(&self) -> Vec<u8> {
        self.data.iter().flat_map(|&f| f.to_ne_bytes()).collect()
    }
}

struct MeshSystem<'a> {
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
}

struct MaterialSystem<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl<'a> MaterialSystem<'a> {
    pub fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        Self { device, queue }
    }
    pub fn cube_map_buffer_from_urls(urls: Vec<&[u8]>) -> Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        urls.iter()
            .map(|&cube_bytes| {
                let cube_image =
                    image::load_from_memory(cube_bytes).expect("Failed to load image from memory");
                cube_image.to_rgba8()
            })
            .collect()
    }

    pub fn create_cube_map_texture(
        &self,
        image_data: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    ) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        // all images must have same dimensions
        let dimensions = image_data[0].dimensions();
        let face_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1, // Single layer for each face
        };

        let cube_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 6, // 6 layers for cube texture
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Cube Texture"),
            view_formats: &[],
        });

        // Copy each face into the cube texture
        for (i, data) in image_data.iter().enumerate() {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &cube_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32, // Specifies which layer of the cube map to copy to
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&data),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some((dimensions.0 * 4) as u32), // 4 bytes per pixel for RGBA
                    rows_per_image: Some(dimensions.1 as u32),
                },
                face_size,
            );
        }

        let cube_material_view = cube_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Cube Material View"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(6),
        });

        let cube_material_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Cube Material Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let cube_material_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Cube Material bind group layout"),
                    entries: &[
                        // texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::Cube,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        // sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            // This should match the filterable field of the
                            // corresponding Texture entry above.
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let cube_material_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &cube_material_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&cube_material_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&cube_material_sampler),
                },
            ],
            label: Some("Cube Material bind group"),
        });
        (cube_material_bind_group, cube_material_bind_group_layout)
    }
}

struct CameraSystem<'a> {
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
        let camera = Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: screen_width as f32 / screen_height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
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
    fn create_uniform_buffer<T: UniformData + bytemuck::Pod>(
        device: &wgpu::Device,
        data: &T,
    ) -> wgpu::Buffer {
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

// only allow (multiples of?) 16 bytes of buffer data to be compliant with WebGL2
trait Uniform {
    fn create_uniform_buffer<T: UniformData + bytemuck::Pod>(
        device: &wgpu::Device,
        data: &T,
    ) -> wgpu::Buffer;

    fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout;

    fn create_uniform_bind_group(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup;
}

pub struct MeshComponent {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

pub struct MaterialComponent {
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

struct CameraComponent {
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    camera_controller: CameraController,
}

struct GlobalUniformsSystem<'a> {
    device: &'a wgpu::Device,
}

impl<'a> GlobalUniformsSystem<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self { device }
    }

    pub fn create_global_uniforms(
        &self,
        uniforms: Uniforms,
    ) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let color_uniform_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Color Uniform Buffer"),
                    contents: bytemuck::bytes_of(&UNIFORMS.color),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let normal_uniform_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Normal Uniform Buffer"),
                    contents: bytemuck::bytes_of(&UNIFORMS.light_direction),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let uniform_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("uniform_bind_group_layout"),
                });

        let uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: color_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: normal_uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("uniform_bind_group"),
        });

        (uniform_bind_group, uniform_bind_group_layout)
    }
}

struct GlobalUniformsComponent {
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl<'a> Uniform for GlobalUniformsSystem<'a> {
    // only allow (multiples of?) 16 bytes of buffer data to be compliant with WebGL2
    fn create_uniform_buffer<T: UniformData + bytemuck::Pod>(
        device: &wgpu::Device,
        data: &T,
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Componet Uniform Buffer"),
            contents: bytemuck::bytes_of(data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
    fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Uniform Component Uniform Bind Group Layout"),
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
            label: Some(" Uniform Component Uniform Bind Group"),
        })
    }
}

struct World {
    // camera_component: CameraComponent,
    entities: Vec<Entity>,
    mesh_components: HashMap<Entity, MeshComponent>,
}

impl World {
    pub fn new(device: &wgpu::Device, screen_width: f32, screen_height: f32) -> Self {
        World {
            entities: Vec::new(),
            mesh_components: HashMap::new(),
            // camera_component: CameraComponent::new(device, screen_width, screen_height),
        }
    }

    pub fn create_entity(&mut self) -> Entity {
        let entity = Entity(self.entities.len());
        self.entities.push(entity);
        entity
    }

    pub fn add_mesh_component(&mut self, entity: Entity, component: MeshComponent) {
        self.mesh_components.insert(entity, component);
    }

    pub fn get_mesh_component(&self, entity: Entity) -> Option<&MeshComponent> {
        self.mesh_components.get(&entity)
    }

    // Additional methods for other components and querying entities, etc.
}

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
struct Entity(usize);

struct State {
    // Renderer
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline, // do you belong to renderer

    // Components (these will go inside world next)
    sphere_mesh_component: MeshComponent,
    camera_component: CameraComponent,
    globe_material_component: MaterialComponent,
    global_uniforms_component: GlobalUniformsComponent,
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = State::create_instance();

        // # Safety
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        let adapter = State::create_adapter(&instance, &surface).await;
        let (device, queue) = State::create_device_and_queue(&adapter).await;

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // there should be a seperation of everything above and below this comment

        // MESHES
        let mesh_system = MeshSystem::new(&device);
        let (vertices_vec, indices_vec) = generate_sphere_mesh();
        let sphere_mesh_component = MeshComponent {
            vertex_buffer: mesh_system.create_vertex_buffer(&vertices_vec.as_slice()),
            index_buffer: mesh_system.create_index_buffer(&indices_vec.as_slice()),
            num_indices: indices_vec.len() as u32,
        };

        // MATERIALS
        let material_system = MaterialSystem::new(&device, &queue);
        let image_data = MaterialSystem::cube_map_buffer_from_urls(vec![
            include_bytes!("./assets/1.png"),
            include_bytes!("./assets/2.png"),
            include_bytes!("./assets/3.png"),
            include_bytes!("./assets/4.png"),
            include_bytes!("./assets/5.png"),
            include_bytes!("./assets/6.png"),
        ]);

        let (globe_materal_bind_group, globe_material_bind_group_layout) =
            material_system.create_cube_map_texture(image_data);
        let globe_material_component = MaterialComponent {
            bind_group: globe_materal_bind_group,
            bind_group_layout: globe_material_bind_group_layout,
        };

        // CAMERA
        let camera_system = CameraSystem::new(&device);
        let (
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            camera_controller,
        ) = camera_system.create_camera(config.width, config.height);
        let camera_component = CameraComponent {
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            camera_controller,
        };

        // GLOBAL UNIFORMS
        let uniforms_system = GlobalUniformsSystem::new(&device);
        let (uniforms_bind_group, uniforms_bind_group_layout) =
            uniforms_system.create_global_uniforms(UNIFORMS);
        let global_uniforms_component = GlobalUniformsComponent {
            bind_group: uniforms_bind_group,
            bind_group_layout: uniforms_bind_group_layout,
        };

        // RENDER PIPELINE (TODO)
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &global_uniforms_component.bind_group_layout,
                    &camera_component.camera_bind_group_layout,
                    &globe_material_component.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),

            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },

            // frag is technically optional, so we
            // have to wrap it in Some
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                // every three vertices will correspond to one triangle.
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                // a triangle is facing forward if the vertices are arranged in
                // a counter-clockwise direction
                front_face: wgpu::FrontFace::Ccw,
                // Some(wgpu::Face::Back) makes it so if objects are not facing
                // camera they are not rendered
                cull_mode: Some(wgpu::Face::Back),
                // anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            // Multisampling is a complex topic not being discussed
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                // related to anti-aliasing
                alpha_to_coverage_enabled: false,
            },
            // We won't be rendering to array textures so we can set this to None
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            sphere_mesh_component,
            camera_component,
            globe_material_component,
            global_uniforms_component,
        }
    }

    pub fn create_instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        })
    }
    pub async fn create_adapter(instance: &wgpu::Instance, surface: &Surface) -> wgpu::Adapter {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap()
    }

    pub async fn create_device_and_queue(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap()
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { button, state, .. } => {
                if button == &MouseButton::Left && state == &ElementState::Pressed {
                    print("I have been clicked")
                }
            }
            _ => {}
        }

        self.camera_component
            .camera_controller
            .process_key_events(event)
    }

    // this will go to world I think
    fn update(&mut self) {
        self.camera_component
            .camera_controller
            .update_camera(&mut self.camera_component.camera);
        self.camera_component
            .camera_uniform
            .update_view_proj(&self.camera_component.camera);
        self.queue.write_buffer(
            &self.camera_component.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_component.camera_uniform]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // get the surface to provide a new SurfaceTexture that we will render to.
        let output = self.surface.get_current_texture()?;

        // We need to do this because we want to control how the render code
        //  interacts with the texture.
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Most modern graphics frameworks expect commands to be stored
        //  in a command buffer before being sent to the gpu.
        // The encoder builds a command buffer that we can then send to the gpu.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.global_uniforms_component.bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_component.camera_bind_group, &[]);
        render_pass.set_bind_group(2, &self.globe_material_component.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.sphere_mesh_component.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.sphere_mesh_component.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..self.sphere_mesh_component.num_indices, 0, 0..1);

        drop(render_pass);

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            console_error_panic_hook::set_once();
            tracing_wasm::set_as_global_default();
        } else {
            tracing_subscriber::fmt::init()
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Set up keydown event listener
    // this works outside of canvas.
    // I don't want to delete because it works
    // and is a good reference
    #[cfg(target_arch = "wasm32")]
    setup_keydown_listener();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to
        // set the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());

                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to div.");
    }

    let mut state = State::new(&window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        // new_inner_size is &&mut so we have to dereference it twice
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
        }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                // The system is out of memory, we should probably quit
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }

        _ => {}
    });
}

#[cfg(target_arch = "wasm32")]
fn setup_keydown_listener() {
    let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        if event.key() == "ArrowDown" {
            // Handle the Arrow Down key press
            web_sys::console::log_1(&"Arrow Down pressed".into());
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);

    window()
        .unwrap()
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget(); // Prevents the closure from being garbage-collected
}

// Define a helper function `print`
fn print(message: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        // Use `console::log_1` for WebAssembly target
        web_sys::console::log_1(&message.into());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Use `println!` for non-WebAssembly targets
        println!("{}", message);
    }
}
