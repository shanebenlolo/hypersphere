mod components;
mod depth_buffer;
mod systems;

use anise::prelude::*;
use components::{
    camera::CameraComponent, material::MaterialComponent, mesh::MeshComponent,
    render_pipelines::RenderPipelineComponent,
};
use depth_buffer::Texture;
use systems::{billboard::BillboardSystem, camera::CameraSystem, window::WindowSystem};

use wgpu::Surface;
use winit::{
    dpi::PhysicalPosition,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use bevy_ecs::{entity::Entity, world::World};

#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, KeyboardEvent, Request, RequestInit, RequestMode, Response};

use crate::systems::{earth::EarthSystem, moon::MoonSystem};

pub const MOON_APPROX: f32 = 1_737.4; // kilometers
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
    _depth_texture: Texture,
    screen_coords: Option<PhysicalPosition<f64>>,

    // geospatial
    almanac: Almanac,
    earth_radius: f32,

    // scene
    world: World,
    _earth_entity: Entity,
    moon_entity: Entity,
    camera_entity: Entity,
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

        // init components via systems
        let camera_component = CameraSystem::create_camera(&device, config.width, config.height);
        let (earth_mesh_component, earth_material_component, earth_render_pipeline_component) =
            EarthSystem::new(&device, &queue, config.format.clone(), &camera_component);
        let (moon_mesh_component, moon_material_component, moon_render_pipeline_component) =
            MoonSystem::new(&device, &queue, config.format.clone(), &camera_component);

        // init entities
        let camera_entity = world.spawn(camera_component).id();
        let earth_entity = world
            .spawn((
                earth_mesh_component,
                earth_material_component,
                earth_render_pipeline_component,
            ))
            .id();
        let moon_entity = world
            .spawn((
                moon_mesh_component,
                moon_material_component,
                moon_render_pipeline_component,
            ))
            .id();

        // remove this await and store the Future in state
        // place this into an ECS paradigm and move above
        let bsp_data = get_bsp_data().await;
        let almanac = Almanac::from_spk(SPK::parse(bsp_data).unwrap()).unwrap();

        // wasm only
        let _depth_texture =
            depth_buffer::Texture::create_depth_texture(&device, &config, "depth texture");

        Self {
            // wgpu-specific
            surface,
            device,
            queue,
            config,
            window_size,
            _depth_texture,

            // screen
            screen_coords: None,

            // math
            almanac,
            earth_radius: WGS84_A,

            // visualization
            world,
            _earth_entity: earth_entity,
            moon_entity,
            camera_entity,
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
                    let camera_component = self
                        .world
                        .get::<CameraComponent>(self.camera_entity)
                        .unwrap();

                    if let Some((lat, lon)) = WindowSystem::handle_left_click(
                        screen_width,
                        screen_height,
                        position_x,
                        position_y,
                        self.earth_radius,
                        camera_component,
                    ) {
                        let size = 500.0;
                        let billboard_mesh = BillboardSystem::create_billboard_mesh(
                            &self.device,
                            size,
                            lat,
                            lon,
                            self.earth_radius + 10.0,
                        );
                        let billboard_material =
                            BillboardSystem::create_billboard_material(&self.device, &self.queue);
                        let billboard_render_pipeline = BillboardSystem::create_render_pipeline(
                            &self.device,
                            camera_component,
                            &billboard_material,
                            &billboard_mesh,
                            &self.config.format,
                        );

                        let _billboard_entity = self.world.spawn((
                            billboard_mesh,
                            billboard_render_pipeline,
                            billboard_material,
                        ));
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.screen_coords = Some(*position);
            }
            _ => {}
        }

        if let Some(mut camera_component) =
            self.world.get_mut::<CameraComponent>(self.camera_entity)
        {
            CameraSystem::process_key_events(&mut camera_component.camera_controller, event);
            return true;
        } else {
            return false;
        }
    }

    fn update(&mut self) {
        MoonSystem::update_position(
            &self.queue,
            &self
                .world
                .get_mut::<MeshComponent>(self.moon_entity)
                .unwrap(),
            &self.almanac,
        );

        CameraSystem::update_camera(
            &self.queue,
            self.world
                .get_mut::<CameraComponent>(self.camera_entity)
                .unwrap(),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

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
                view: &self._depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        let mut objects_query =
            self.world
                .query::<(&RenderPipelineComponent, &MeshComponent, &MaterialComponent)>();

        let camera_component = self
            .world
            .get::<CameraComponent>(self.camera_entity)
            .unwrap();
        render_pass.set_bind_group(0, &camera_component.camera_bind_group, &[]);

        for (render_pipeline, mesh, material) in objects_query.iter(&self.world) {
            render_pass.set_pipeline(&render_pipeline.render_pipeline);
            render_pass.set_bind_group(1, &material.bind_group, &[]);
            render_pass.set_bind_group(2, &mesh.model_matrix_bind_group, &[]);

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

async fn get_bsp_data() -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);

        let url = format!("http://localhost:3000/de440s.bsp");

        let request = Request::new_with_str_and_init(&url, &opts).unwrap();

        request.headers().set("Accept", "*/*").unwrap();

        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .unwrap();

        // `resp_value` is a `Response` object.
        assert!(resp_value.is_instance_of::<Response>());
        let resp: Response = resp_value.dyn_into().unwrap();

        let buffer: JsValue = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
        // Convert JsValue (ArrayBuffer) into Vec<u8>
        let uint8_array: Uint8Array = Uint8Array::new(&buffer);

        return uint8_array.to_vec();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // URL of the web server you want to fetch data from
        let url = "http://localhost:3000/de440s.bsp";

        // Send a GET request to the specified URL
        let response = reqwest::get(url).await.unwrap();

        // Retrieve the response body as a vector of bytes
        let bytes = response.bytes().await.unwrap();

        // Return the vector of bytes
        return bytes.to_vec();
    }
}

// Define a helper function `print`
fn _print(message: &str) {
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
