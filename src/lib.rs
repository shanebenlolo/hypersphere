mod components;
mod systems;
mod world;

use components::{
    camera::CameraComponent, material::MaterialComponent, mesh::MeshComponent,
    render_pipelines::RenderPipelineComponent,
};
use systems::{billboard::BillboardSystem, camera::CameraSystem, window::WindowSystem};
use tracing_subscriber::fmt::time::UtcTime;
use wgpu::Surface;
use winit::{
    dpi::PhysicalPosition,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use anise::{
    constants::frames::{self},
    prelude::*,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, KeyboardEvent};
use world::World;

use crate::{
    components::{earth::EarthComponent, moon::MoonComponent},
    systems::{earth::EarthSystem, moon::MoonSystem},
};

use chrono::{DateTime, Local, Utc};

pub const WGS84_A: f32 = 6_378.0; // Semi-major axis (equatorial radius) in kilometers
pub const WGS84_B: f32 = 6_357.0; // Semi-minor axis (polar radius) in kilometers

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
    window_size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // scene
    world: World,
    camera_component: CameraComponent,
    depth_texture: (wgpu::Texture, wgpu::TextureView, wgpu::Sampler),
    screen_coords: Option<PhysicalPosition<f64>>,

    earth_radius: f32,
    earth_entity: usize,
    moon_entity: usize,

    current_time: DateTime<Utc>,
}
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float; // 1.
impl State {
    async fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface,
        config: wgpu::SurfaceConfiguration,
        window_size: winit::dpi::PhysicalSize<u32>,
    ) -> Self {
        let mut world = World::new();

        // CAMERA
        let (
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            camera_controller,
        ) = CameraSystem::create_camera(&device, config.width, config.height);
        let camera_component = CameraComponent {
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            camera_controller,
        };

        // DEPTH TEXTURE (sadly wasm only for now)
        pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float; // 1.
        let depth_size = wgpu::Extent3d {
            // 2.
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("depth texture"),
            size: depth_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // 4.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::Always), // 5.
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });
        let depth_texture = (texture, view, sampler);

        let (earth_mesh_component, earth_material_component, earth_render_pipeline_component) =
            EarthSystem::new(&device, &queue, config.format.clone(), &camera_component);
        let earth_component = EarthComponent {
            mesh_component: earth_mesh_component,
            material_component: earth_material_component,
            render_pipeline_component: earth_render_pipeline_component,
        };
        let earth_entity = world.new_entity();
        world.add_component_to_entity(earth_entity, earth_component.mesh_component);
        world.add_component_to_entity(earth_entity, earth_component.material_component);
        world.add_component_to_entity(earth_entity, earth_component.render_pipeline_component);

        let (moon_mesh_component, moon_material_component, moon_render_pipeline_component) =
            MoonSystem::new(&device, &queue, config.format.clone(), &camera_component);
        let moon_component = MoonComponent {
            mesh_component: moon_mesh_component,
            material_component: moon_material_component,
            render_pipeline_component: moon_render_pipeline_component,
        };
        let moon_entity = world.new_entity();
        world.add_component_to_entity(moon_entity, moon_component.mesh_component);
        world.add_component_to_entity(moon_entity, moon_component.material_component);
        world.add_component_to_entity(moon_entity, moon_component.render_pipeline_component);

        // Test run of anise, will be moving out in next couple of commits
        let spk = SPK::load("./data/de440s.bsp").unwrap();
        let ctx = Almanac::from_spk(spk).unwrap();
        // Define an Epoch in the dynamical barycentric time scale
        let epoch = Epoch::from_str("2020-11-15 12:34:56.789 TDB").unwrap();
        let state = ctx
            .translate_from_to(
                frames::LUNA_J2000,  // Target
                frames::EARTH_J2000, // Observer
                epoch,
                Aberration::None,
            )
            .unwrap();
        println!("{state}");

        Self {
            surface,
            device,
            queue,
            config,
            window_size,
            camera_component,
            world,
            screen_coords: None,
            earth_radius: WGS84_A,
            depth_texture,
            current_time: Utc::now(),
            earth_entity,
            moon_entity,
        }
    }

    pub fn create_instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Dxc {
                dxil_path: None,
                dxc_path: None,
            },
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
            self.window_size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { button, state, .. } => {
                let screen_width = self.config.width as f32;
                let screen_height = self.config.height as f32;
                let position_x = self.screen_coords.unwrap().x as f32;
                let position_y = self.screen_coords.unwrap().y as f32;

                if button == &MouseButton::Left && state == &ElementState::Pressed {
                    if let Some((lat, lon)) = WindowSystem::handle_left_click(
                        screen_width,
                        screen_height,
                        position_x,
                        position_y,
                        self.earth_radius,
                        &self.camera_component,
                    ) {
                        let size = 500.0;
                        let billboard_mesh = BillboardSystem::create_billboard_mesh(
                            &self.device,
                            size,
                            lat,
                            lon,
                            self.earth_radius,
                        );
                        let billboard_material =
                            BillboardSystem::create_billboard_material(&self.device, &self.queue);
                        let billboard_render_pipeline = BillboardSystem::create_render_pipeline(
                            &self.device,
                            &self.camera_component,
                            &billboard_material,
                            &billboard_mesh,
                            &self.config.format,
                        );

                        let billboard_entity = self.world.new_entity();
                        self.world
                            .add_component_to_entity(billboard_entity, billboard_mesh);
                        self.world
                            .add_component_to_entity(billboard_entity, billboard_render_pipeline);
                        self.world
                            .add_component_to_entity(billboard_entity, billboard_material);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.screen_coords = Some(*position);
            }
            _ => {}
        }

        CameraSystem::process_key_events(&mut self.camera_component.camera_controller, event)
    }

    fn update(&mut self) {
        let now = Utc::now();
        let duration = self.current_time - Utc::now();
        if duration.num_seconds().abs() >= 1 {
            self.current_time = now;

            // this isn't working. I need to revisit the ECS and
            // get mutable borrows of components working. Might just
            // throw away the ECS at this point or grab a 3rd party one...
            // let moon = self.world.get_component(self.moon_entity);
            // MoonSystem::update_position()
        }

        CameraSystem::update_camera(
            &mut self.camera_component.camera_controller,
            &mut self.camera_component.camera,
        );
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

            // depth stencil only working on wasm :(
            #[cfg(not(target_arch = "wasm32"))]
            depth_stencil_attachment: None,
            #[cfg(target_arch = "wasm32")]
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.1,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_bind_group(0, &self.camera_component.camera_bind_group, &[]);

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
        // set the window_size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(800, 600));

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

    let window_size = window.inner_size();

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
        width: window_size.width,
        height: window_size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    let mut state = State::new(device, queue, surface, config, window_size).await;

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
                Err(wgpu::SurfaceError::Lost) => state.resize(state.window_size),
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
