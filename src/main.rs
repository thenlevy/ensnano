use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
pub type PhySize = iced_winit::winit::dpi::PhysicalSize<u32>;

use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Debug, Size};

use futures::task::SpawnExt;
use winit::{
    dpi::PhysicalPosition,
    event::{Event, KeyboardInput, ModifiersState, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
mod scene;
//mod model;
//mod camera;
mod consts;
mod controls;
mod design_handler;
//mod design_viewer;
mod instance;
mod light;
mod mesh;
//mod pipeline_handler;
mod texture;
//mod uniforms;
mod utils;

use design_handler::DesignHandler;

use controls::Controls;
use scene::{ Scene, SceneNotification };

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = if args.len() >= 2 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    // Initialize winit
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    let physical_size = window.inner_size();
    let mut viewport = Viewport::with_physical_size(
        Size::new(physical_size.width, physical_size.height),
        window.scale_factor(),
    );
    let mut cursor_position = PhysicalPosition::new(-1.0, -1.0);
    let mut modifiers = ModifiersState::default();
    
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    // Initialize WGPU
     let (mut device, mut queue) = futures::executor::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Request adapter");

        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: false,
                },
                None,
            )
            .await
            .expect("Request device")
    });

    let format = wgpu::TextureFormat::Bgra8UnormSrgb;

    let mut swap_chain = {
        let size = window.inner_size();

        device.create_swap_chain(
            &surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Mailbox,
            },
        )
    };
    let mut resized = false;
    let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
    let mut local_pool = futures::executor::LocalPool::new();

    // Initialize scene and GUI controls
    let mut design_handler = if let Some(path) = path {
        DesignHandler::new_with_path(&path)
    } else {
        DesignHandler::new()
    };
    let mut scene = Scene::new(&device, window.inner_size());
    design_handler.update_scene(&mut scene, true);
    let fitting_request = Arc::new(Mutex::new(false));
    let file_opening_request = Arc::new(Mutex::new(None));
    let controls = Controls::new(&fitting_request, &file_opening_request);

    //let mut cache = Some(Cache::default());
    let mut debug = Debug::new();
    let mut renderer = Renderer::new(Backend::new(&mut device, Settings::default()));
    let mut state = program::State::new(
        controls,
        viewport.logical_size(),
        conversion::cursor_position(cursor_position, viewport.scale_factor()),
        &mut renderer,
        &mut debug,
    );

    //let mut output = (Primitive::None, MouseCursor::OutOfBounds);
    //let clipboard = Clipboard::new(&window);


    // Run event loop
    let mut last_render_time = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        // You should change this if you want to render continuosly
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(40));

        match event {
            Event::WindowEvent { event, .. } => {
                if scene.input(&event, &device, &mut queue) {
                    design_handler.update_scene_selection(&mut scene);
                }
                match event {
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        modifiers = new_modifiers;
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor_position = position;
                    }
                    WindowEvent::Resized(new_size) => {
                        viewport = Viewport::with_physical_size(
                            Size::new(new_size.width, new_size.height),
                            window.scale_factor(),
                        );
                        resized = true;
                        scene.notify(SceneNotification::Resize(new_size));
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::F),
                                ..
                            },
                        ..
                    } => {
                        design_handler.fit_design(&mut scene);
                    }

                    _ => {}
                }

                // Map window event to iced event
                if let Some(event) =
                    iced_winit::conversion::window_event(&event, window.scale_factor(), modifiers)
                {
                   state.queue_event(event);
                }
            }
            Event::MainEventsCleared => {
                if !state.is_queue_empty() {
                    // We update iced
                    let _ = state.update(
                        viewport.logical_size(),
                        conversion::cursor_position(
                            cursor_position,
                            viewport.scale_factor(),
                        ),
                        None,
                        &mut renderer,
                        &mut debug,
                    );
                    {
                        let mut fitting_request_lock = fitting_request.lock().expect("fitting_request");
                        if *fitting_request_lock {
                            design_handler.fit_design(&mut scene);
                            *fitting_request_lock = false;
                        }
                    }
                    {
                        let mut file_opening_request_lock = file_opening_request.lock().expect("fitting_request_lock");
                        if let Some(ref path) = *file_opening_request_lock {
                            design_handler.get_design(path);
                            design_handler.update_scene(&mut scene, true);
                            *file_opening_request_lock = None;
                        }
                    }
                }
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                if resized {
                    let size = window.inner_size();

                    swap_chain = device.create_swap_chain(
                        &surface,
                        &wgpu::SwapChainDescriptor {
                            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                            format,
                            width: size.width,
                            height: size.height,
                            present_mode: wgpu::PresentMode::Mailbox,
                        },
                    );

                    resized = false;
                }

                let frame = swap_chain.get_current_frame().expect("Next frame");

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // We draw the scene first
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                //scene.draw(&mut encoder, &frame.output.view, &device, dt, false);
                scene.draw_view(&mut encoder, &frame.output.view, &device, dt, false, &queue);

                // And then iced on top
                let mouse_interaction = renderer.backend_mut().draw(
                    &mut device,
                    &mut staging_belt,
                    &mut encoder,
                    &frame.output.view,
                    &viewport,
                    state.primitive(),
                    &debug.overlay(),
                );

                // Then we submit the work
                staging_belt.finish();
                queue.submit(Some(encoder.finish()));

                // And update the mouse cursor
                window.set_cursor_icon(
                    iced_winit::conversion::mouse_interaction(
                        mouse_interaction,
                    ),
                );
                local_pool
                    .spawner()
                    .spawn(staging_belt.recall())
                    .expect("Recall staging buffers");

                local_pool.run_until_stalled();
            }
            _ => {}
        }
    })
}
