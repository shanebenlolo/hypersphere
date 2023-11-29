mod components;
mod systems;

use components::{
    camera::CameraComponent,
    global_uniform::{self, GlobalUniformComponent},
    material::MaterialComponent,
    mesh::{MeshComponent, Vertex},
    render_pipeline::{self, RenderPipelineComponent},
};
use image::{ImageBuffer, Rgba};

use systems::{
    camera::CameraSystem,
    global_uniform::{GlobalUniformSystem, UNIFORMS},
    material::MaterialSystem,
    mesh::MeshSystem,
    render_pipeline::RenderPipelineSystem,
};
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

pub trait Uniform {
    // only allow (multiples of?) 16 bytes of buffer
    // data to be compliant with WebGL2.
    // should probably make T enforce this.
    fn create_uniform_buffer<T: bytemuck::Pod>(device: &wgpu::Device, data: &T) -> wgpu::Buffer;

    fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout;

    fn create_uniform_bind_group(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup;
}

struct World {
    entities_count: usize,
    component_vecs: Vec<Box<dyn ComponentVec>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities_count: 0,
            component_vecs: Vec::new(),
        }
    }

    fn new_entity(&mut self) -> usize {
        let entity_id = self.entities_count;
        for component_vec in self.component_vecs.iter_mut() {
            component_vec.push_none();
        }
        self.entities_count += 1;
        entity_id
    }

    pub fn add_component_to_entity<ComponentType: 'static>(
        &mut self,
        entity: usize,
        component: ComponentType,
    ) {
        for component_vec in self.component_vecs.iter_mut() {
            if let Some(component_vec) = component_vec
                .as_any_mut()
                .downcast_mut::<Vec<Option<ComponentType>>>()
            {
                component_vec[entity] = Some(component);
                return;
            }
        }

        // No matching component storage exists yet, so we have to make one.
        let mut new_component_vec: Vec<Option<ComponentType>> =
            Vec::with_capacity(self.entities_count);

        // All existing entities don't have this component, so we give them `None`
        for _ in 0..self.entities_count {
            new_component_vec.push(None);
        }

        // Give this Entity the Component.
        new_component_vec[entity] = Some(component);
        self.component_vecs.push(Box::new(new_component_vec));
    }

    //  finds and borrows the ComponentVec that matches a type
    fn borrow_component_vec<ComponentType: 'static>(&self) -> Option<&Vec<Option<ComponentType>>> {
        for component_vec in self.component_vecs.iter() {
            if let Some(component_vec) = component_vec
                .as_any()
                .downcast_ref::<Vec<Option<ComponentType>>>()
            {
                return Some(component_vec);
            }
        }
        None
    }
}

trait ComponentVec {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn push_none(&mut self);
}

impl<T: 'static> ComponentVec for Vec<Option<T>> {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
    fn push_none(&mut self) {
        self.push(None)
    }
}

struct State {
    // Renderer
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // Components (these will go inside world next)
    // sphere_mesh_component: MeshComponent,
    camera_component: CameraComponent,
    // globe_material_component: MaterialComponent,
    // global_uniforms_component: GlobalUniformComponent,
    render_pipeline_component: RenderPipelineComponent,

    world: World,
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
        let (vertices_vec, indices_vec) = MeshSystem::generate_sphere_mesh();
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
        let uniforms_system = GlobalUniformSystem::new(&device);
        let (uniforms_bind_group, uniforms_bind_group_layout) =
            uniforms_system.create_global_uniforms(UNIFORMS);
        let global_uniforms_component = GlobalUniformComponent {
            bind_group: uniforms_bind_group,
            bind_group_layout: uniforms_bind_group_layout,
        };

        // RENDER PIPELINE (TODO)
        let render_pipeline_system = RenderPipelineSystem::new(&device);
        // this list feels like it should be generated in world to be dynamic
        // but I am leaving it outside for now and hardcoding.
        let render_pipeline_layout = render_pipeline_system.create_render_pipeline_layout(&[
            &global_uniforms_component.bind_group_layout,
            &camera_component.camera_bind_group_layout,
            &globe_material_component.bind_group_layout,
        ]);
        let render_pipeline = render_pipeline_system.create_render_pipeline(
            &render_pipeline_layout,
            &shader,
            config.format,
        );

        let render_pipeline_component = RenderPipelineComponent {
            render_pipeline,
            render_pipeline_layout,
        };

        let mut world = World::new();
        let globe_entity = world.new_entity();
        world.add_component_to_entity(globe_entity, sphere_mesh_component);
        world.add_component_to_entity(globe_entity, global_uniforms_component); //this doesn't really make sense to place into a single entity... it just shouldn't be global and a system should act on the uniform per entity!
        world.add_component_to_entity(globe_entity, globe_material_component);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline_component,
            // sphere_mesh_component,
            camera_component,
            // globe_material_component,
            // global_uniforms_component,
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

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

        let zip = self
            .world
            .borrow_component_vec::<MeshComponent>()
            .unwrap()
            .iter()
            .zip(
                self.world
                    .borrow_component_vec::<MaterialComponent>()
                    .unwrap()
                    .iter()
                    .zip(
                        self.world
                            .borrow_component_vec::<GlobalUniformComponent>()
                            .unwrap()
                            .iter(),
                    ),
            );

        render_pass.set_pipeline(&self.render_pipeline_component.render_pipeline);
        render_pass.set_bind_group(1, &self.camera_component.camera_bind_group, &[]);

        for (mesh, material, uniforms) in zip.filter_map(|(mesh, (material, uniforms))| {
            Some((mesh.as_ref()?, material.as_ref()?, uniforms.as_ref()?))
        }) {
            render_pass.set_bind_group(0, &uniforms.bind_group, &[]);

            render_pass.set_bind_group(2, &material.bind_group, &[]);

            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
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
