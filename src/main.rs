use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
pub type PhySize = iced_winit::winit::dpi::PhysicalSize<u32>;

use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Debug, Size};

use futures::task::SpawnExt;
use winit::{
    dpi::PhysicalPosition,
    event::{Event, ModifiersState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

#[macro_use]
extern crate serde_derive;
extern crate serde;

mod consts;
mod controls;
mod design;
mod scene;
//mod design_handler;
mod instance;
mod light;
mod mesh;
mod multiplexer;
mod texture;
mod utils;

//use design_handler::DesignHandler;

use controls::Controls;
use multiplexer::{DrawArea, Multiplexer};
use scene::{Scene, SceneNotification};

fn convert_size(size: PhySize) -> Size<f32> {
    Size::new(size.width as f32, size.height as f32)
}

fn convert_size_u32(size: PhySize) -> Size<u32> {
    Size::new(size.width, size.height)
}

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

    let cursor_position = PhysicalPosition::new(-1.0, -1.0);
    let modifiers = ModifiersState::default();

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
    let mut multiplexer = Multiplexer::new(window.inner_size());
    let scene_area = multiplexer.get_scene_area();
    let mut scene = Scene::new(&device, window.inner_size(), scene_area);
    if let Some(ref path) = path {
        scene.add_design(path)
    }
    scene.fit_design();
    let fitting_request = Arc::new(Mutex::new(false));
    let file_add_request = Arc::new(Mutex::new(None));
    let file_replace_request = Arc::new(Mutex::new(None));
    let controls = Controls::new(
        fitting_request.clone(),
        file_add_request.clone(),
        file_replace_request.clone(),
    );

    //let mut cache = Some(Cache::default());
    let mut debug = Debug::new();
    let mut renderer = Renderer::new(Backend::new(&mut device, Settings::default()));
    let top_bar_area = multiplexer.get_top_bar_area();
    let mut state = program::State::new(
        controls,
        convert_size(top_bar_area.size),
        conversion::cursor_position(cursor_position, window.scale_factor()),
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
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { event, .. } => {
                //let modifiers = multiplexer.modifiers();
                if let WindowEvent::Resized(_) = event {
                    resized = true;
                }
                if let Some(event) = event.to_static() {
                    let event = multiplexer.event(event);

                    // Iced panel
                    if let Some((event, area)) = event {
                        if area == multiplexer.top_bar {
                            let event = iced_winit::conversion::window_event(
                                &event,
                                window.scale_factor(),
                                modifiers,
                            );
                            if let Some(event) = event {
                                state.queue_event(event);
                            }
                        } else if area == multiplexer.scene {
                            let cursor_position = multiplexer.scene_cursor_position;
                            scene.input(&event, &device, &mut queue, cursor_position);
                        }
                    }
                    // 3D view
                }
            }
            Event::MainEventsCleared => {
                let top_bar_area = multiplexer.get_top_bar_area();
                let top_bar_cursor = multiplexer.top_bar_cursor_position;

                if !state.is_queue_empty() {
                    // We update iced
                    let _ = state.update(
                        convert_size(top_bar_area.size),
                        conversion::cursor_position(top_bar_cursor, window.scale_factor()),
                        None,
                        &mut renderer,
                        &mut debug,
                    );
                    {
                        let mut fitting_request_lock =
                            fitting_request.lock().expect("fitting_request");
                        if *fitting_request_lock {
                            //design_handler.fit_design(&mut scene);
                            scene.fit_design();
                            *fitting_request_lock = false;
                        }
                    }
                    {
                        let mut file_add_request_lock =
                            file_add_request.lock().expect("fitting_request_lock");
                        if let Some(ref path) = *file_add_request_lock {
                            //design_handler.get_design(path);
                            //design_handler.update_scene(&mut scene, true);
                            scene.add_design(path);
                            *file_add_request_lock = None;
                        }
                    }
                    {
                        let mut file_replace_request_lock =
                            file_replace_request.lock().expect("fitting_request_lock");
                        if let Some(ref path) = *file_replace_request_lock {
                            //design_handler.get_design(path);
                            //design_handler.update_scene(&mut scene, true);
                            scene.clear_design(path);
                            *file_replace_request_lock = None;
                        }
                    }
                }
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let top_bar_area = multiplexer.get_top_bar_area();
                let top_bar_cursor = multiplexer.top_bar_cursor_position;
                if resized {
                    let window_size = window.inner_size();
                    let scene_area = multiplexer.get_scene_area();
                    scene.notify(SceneNotification::NewSize(window_size, scene_area));

                    swap_chain = device.create_swap_chain(
                        &surface,
                        &wgpu::SwapChainDescriptor {
                            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                            format,
                            width: window_size.width,
                            height: window_size.height,
                            present_mode: wgpu::PresentMode::Mailbox,
                        },
                    );
                }
                // Get viewports from the partition

                // If there are events pending
                if !state.is_queue_empty() || resized {
                    // We update iced
                    let _ = state.update(
                        convert_size(top_bar_area.size),
                        conversion::cursor_position(top_bar_cursor, window.scale_factor()),
                        None,
                        &mut renderer,
                        &mut debug,
                    );
                }

                resized = false;

                let frame = swap_chain.get_current_frame().expect("Next frame");

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // We draw the scene first
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                //scene.draw(&mut encoder, &frame.output.view, &device, dt, false);
                scene.draw_view(&mut encoder, &frame.output.view, &device, dt, false, &queue);

                let viewport = Viewport::with_physical_size(
                    convert_size_u32(multiplexer.window_size),
                    window.scale_factor(),
                );

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
                window
                    .set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
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
