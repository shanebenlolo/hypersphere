mod components;
mod systems;
mod world;

use cgmath::SquareMatrix;
use components::{
    camera::CameraComponent, material::MaterialComponent, mesh::MeshComponent,
    render_pipelines::RenderPipelineComponent,
};
use systems::{
    camera::CameraSystem,
    material::MaterialSystem,
    mesh::MeshSystem,
    render_pipelines::{GlobeRenderPipelineSystem, PointRenderPipelineSystem},
};
use wgpu::Surface;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, KeyboardEvent};
use world::World;

pub trait Uniform {
    // only allow (multiples of?) 16 bytes of buffer
    // data to be compliant with WebGL2.
    // should probably make T enforce this.
    fn create_uniform_buffer<T: bytemuck::Pod>(device: &wgpu::Device, data: &T) -> wgpu::Buffer;

    // fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout;

    fn create_uniform_bind_group(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup;
}

// this belongs somewhere else like serialization util or something
fn matrix4_to_array(mat: cgmath::Matrix4<f32>) -> [[f32; 4]; 4] {
    let m: [[f32; 4]; 4] = mat.into();
    [
        [m[0][0], m[0][1], m[0][2], m[0][3]],
        [m[1][0], m[1][1], m[1][2], m[1][3]],
        [m[2][0], m[2][1], m[2][2], m[2][3]],
        [m[3][0], m[3][1], m[3][2], m[3][3]],
    ]
}

struct State {
    // renderer
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // scene
    world: World,

    // I don't think this needs to
    // be a "component" but we will
    // leave it as such for the time being
    camera_component: CameraComponent,
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

        // there should be a seperation of everything above and below this comment

        // -------------------------------------------
        // THESE ARE CURRENTLY GLOBAL BUT SHOULDN'T BE
        // -------------------------------------------
        let color: [f32; 4] = [0.0, 0.0, 0.0, 0.0]; // Color as RGBA
        let light_direction: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
        let image_data = MaterialSystem::cube_map_buffer_from_urls(vec![
            include_bytes!("./assets/1.png"),
            include_bytes!("./assets/2.png"),
            include_bytes!("./assets/3.png"),
            include_bytes!("./assets/4.png"),
            include_bytes!("./assets/5.png"),
            include_bytes!("./assets/6.png"),
        ]);
        // -------------------------------------------
        // THESE ARE CURRENTLY GLOBAL BUT SHOULDN'T BE
        // -------------------------------------------

        // init world and systems
        let mut world = World::new();
        let mesh_system = MeshSystem::new(&device);
        let material_system = MaterialSystem::new(&device, &queue);
        let globe_render_pipeline_system = GlobeRenderPipelineSystem::new(&device);
        let point_render_pipeline_system = PointRenderPipelineSystem::new(&device);

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

        // GLOBE

        // mesh
        let globe_matrix = matrix4_to_array(cgmath::Matrix4::identity());
        let globe_matrix_bind_group_layout = mesh_system.create_model_matrix_bind_group_layout();
        let globe_matrix_bind_group = mesh_system
            .create_mode_matrix_bind_group(&globe_matrix_bind_group_layout, globe_matrix.clone());
        let (globe_vertices_vec, globe_indices_vec) = MeshSystem::generate_sphere_mesh(100.0, 90);
        let globe_mesh_component = MeshComponent {
            vertex_buffer: mesh_system.create_vertex_buffer(&globe_vertices_vec.as_slice()),
            index_buffer: mesh_system.create_index_buffer(&globe_indices_vec.as_slice()),
            num_indices: globe_indices_vec.len() as u32,
            model_matrix_bind_group_layout: globe_matrix_bind_group_layout,
            model_matrix_bind_group: globe_matrix_bind_group,
            model_matrix: globe_matrix,
        };
        // material
        let (materal_bind_group, material_bind_group_layout) =
            material_system.create_cube_map_texture(image_data.clone());
        let globe_material_component = MaterialComponent {
            bind_group: materal_bind_group,
            bind_group_layout: material_bind_group_layout,
            uniforms: vec![color, light_direction],
            shader: device.create_shader_module(wgpu::include_wgsl!("./shaders/globe_shader.wgsl")),
        };

        // render_pipeline
        let globe_pipeline_layouts: &[&wgpu::BindGroupLayout] = &[
            &camera_component.camera_bind_group_layout,
            &globe_material_component.bind_group_layout,
            &globe_mesh_component.model_matrix_bind_group_layout,
        ];
        let globe_render_pipeline_layout =
            globe_render_pipeline_system.layout_desc(globe_pipeline_layouts);
        let globe_render_pipeline = globe_render_pipeline_system.pipeline_desc(
            &globe_render_pipeline_layout,
            &globe_material_component.shader,
            config.format,
        );
        let globe_render_pipeline_component = RenderPipelineComponent {
            render_pipeline: globe_render_pipeline,
            render_pipeline_layout: globe_render_pipeline_layout,
        };

        // create entity with components
        let globe_entity = world.new_entity();
        world.add_component_to_entity(globe_entity, globe_mesh_component);
        world.add_component_to_entity(globe_entity, globe_material_component);
        world.add_component_to_entity(globe_entity, globe_render_pipeline_component);

        // POINTS
        // mesh
        let translation2 = cgmath::Vector3::new(-220.0, 0.0, 0.0);
        let point_matrix = matrix4_to_array(cgmath::Matrix4::from_translation(translation2));
        let point_matrix_bind_group_layout = mesh_system.create_model_matrix_bind_group_layout();
        let point_matrix_bind_group = mesh_system
            .create_mode_matrix_bind_group(&point_matrix_bind_group_layout, point_matrix.clone());
        let (point_vertices_vec, point_indices_vec) = MeshSystem::generate_sphere_mesh(10.0, 10);
        let point_mesh_component = MeshComponent {
            vertex_buffer: mesh_system.create_vertex_buffer(&point_vertices_vec.as_slice()),
            index_buffer: mesh_system.create_index_buffer(&point_indices_vec.as_slice()),
            num_indices: point_indices_vec.len() as u32,
            model_matrix_bind_group_layout: point_matrix_bind_group_layout,
            model_matrix_bind_group: point_matrix_bind_group,
            model_matrix: point_matrix,
        };

        // material
        let (materal_bind_group, material_bind_group_layout) =
            material_system.create_cube_map_texture(image_data);
        let point_material_component = MaterialComponent {
            bind_group: materal_bind_group,
            bind_group_layout: material_bind_group_layout,
            uniforms: vec![color, light_direction],
            shader: device.create_shader_module(wgpu::include_wgsl!("./shaders/point_shader.wgsl")),
        };

        // render_pipeline
        let point_pipeline_layouts: &[&wgpu::BindGroupLayout] = &[
            &camera_component.camera_bind_group_layout,
            // &point_material_component.bind_group_layout,
            &point_mesh_component.model_matrix_bind_group_layout,
        ];
        let point_render_pipeline_layout =
            point_render_pipeline_system.layout_desc(point_pipeline_layouts);
        let point_render_pipeline = point_render_pipeline_system.pipeline_desc(
            &point_render_pipeline_layout,
            &point_material_component.shader,
            config.format,
        );
        let point_render_pipeline_component = RenderPipelineComponent {
            render_pipeline: point_render_pipeline,
            render_pipeline_layout: point_render_pipeline_layout,
        };

        // create entity with components
        let point_entity = world.new_entity();
        world.add_component_to_entity(point_entity, point_mesh_component);
        world.add_component_to_entity(point_entity, point_render_pipeline_component);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            camera_component,
            world,
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

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // get the surface to provide a new SurfaceTexture that we will render to.
        let output = self.surface.get_current_texture()?;

        // We need to do this because we want to control how the render code
        //  interacts with the texture.
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Most modern graphics frameworks expect commands to be stored
        // in a command buffer before being sent to the gpu.
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

        render_pass.set_bind_group(0, &self.camera_component.camera_bind_group, &[]);

        // the following queries are mostly just for demonstration...
        // earth is rendered with the first set of entity ids,
        // the points are rendered with the second, because
        // they have no material

        let entity_ids_with_mesh_and_material = self
            .world
            .query_entities_with_material_and_mesh::<MeshComponent, MaterialComponent>();

        for entity_id in entity_ids_with_mesh_and_material {
            // Retrieve components for the current entity
            let render_pipeline = self
                .world
                .get_component::<RenderPipelineComponent>(entity_id);
            let mesh = self.world.get_component::<MeshComponent>(entity_id);
            let material = self.world.get_component::<MaterialComponent>(entity_id);

            // Check if all components are available
            if let (Some(render_pipeline), Some(mesh), Some(material)) =
                (render_pipeline, mesh, material)
            {
                render_pass.set_pipeline(&render_pipeline.render_pipeline);
                render_pass.set_bind_group(1, &material.bind_group, &[]);
                render_pass.set_bind_group(2, &mesh.model_matrix_bind_group, &[]);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }

        let entity_ids_with_mesh_no_material = self
            .world
            .query_entities_with_mesh_but_no_material::<MeshComponent, MaterialComponent>();

        for entity_id in entity_ids_with_mesh_no_material {
            // Retrieve components for the current entity
            let render_pipeline = self
                .world
                .get_component::<RenderPipelineComponent>(entity_id);
            let mesh = self.world.get_component::<MeshComponent>(entity_id);

            // Check if both components are available
            if let (Some(render_pipeline), Some(mesh)) = (render_pipeline, mesh) {
                render_pass.set_pipeline(&render_pipeline.render_pipeline);
                // Material bind group is not set since these entities don't have a material
                render_pass.set_bind_group(1, &mesh.model_matrix_bind_group, &[]);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }

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

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to
        // set the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(1080, 1080));

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
