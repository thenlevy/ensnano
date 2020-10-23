use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
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
/// Design handling
mod design;
/// Graphical interface drawing
mod gui;
use design::Design;
/// Message passing between applications
mod mediator;
/// Separation of the window into drawing regions
mod multiplexer;
/// 3D scene drawing
mod scene;
use mediator::Mediator;
mod flatscene;
mod text;
mod utils;

use flatscene::FlatScene;
use gui::{LeftPanel, Requests, TopBar};
use multiplexer::{DrawArea, ElementType, Multiplexer};
use scene::{Scene, SceneNotification};

fn convert_size(size: PhySize) -> Size<f32> {
    Size::new(size.width as f32, size.height as f32)
}

fn convert_size_u32(size: PhySize) -> Size<u32> {
    Size::new(size.width, size.height)
}

fn main() {
    // parse arugments, if an argument was given it is treated as a file to open
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
    let (device, queue) = futures::executor::block_on(async {
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
    let mut renderer = Renderer::new(Backend::new(&device, Settings::default()));
    let device = Rc::new(device);
    let queue = Rc::new(queue);
    let mut resized = false;
    let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
    let mut local_pool = futures::executor::LocalPool::new();

    // Initialize the mediator
    let messages = Arc::new(Mutex::new(Messages::new()));
    let mediator = Arc::new(Mutex::new(Mediator::new(messages.clone())));

    // Initialize the layout
    let mut multiplexer = Multiplexer::new(window.inner_size(), window.scale_factor());

    // Initialize the scenes
    let scene_area = multiplexer.get_element_area(ElementType::Scene);
    let scene = Arc::new(Mutex::new(Scene::new(
        device.clone(),
        queue.clone(),
        window.inner_size(),
        scene_area,
        mediator.clone(),
    )));
    mediator.lock().unwrap().add_application(scene.clone());

    let mut draw_flat = false;
    let flat_scene = Arc::new(Mutex::new(FlatScene::new(
        device.clone(),
        queue.clone(),
        window.inner_size(),
        scene_area,
    )));
    mediator.lock().unwrap().add_application(flat_scene.clone());

    // Add a design to the scene if one was given as a command line arguement
    if let Some(ref path) = path {
        let design = Design::new_with_path(0, path);
        if let Some(design) = design {
            let design = Arc::new(Mutex::new(design));
            mediator.lock().unwrap().add_design(design);
            scene.lock().unwrap().fit_design();
        }
    }

    // Initialize the UI
    let requests = Arc::new(Mutex::new(Requests::new()));

    // Top bar
    let top_bar_area = multiplexer.get_element_area(ElementType::TopBar);
    let top_bar = TopBar::new(
        requests.clone(),
        top_bar_area.size.to_logical(window.scale_factor()),
    );
    let mut top_bar_debug = Debug::new();
    let mut top_bar_state = program::State::new(
        top_bar,
        convert_size(top_bar_area.size),
        conversion::cursor_position(cursor_position, window.scale_factor()),
        &mut renderer,
        &mut top_bar_debug,
    );

    // Left panel
    let left_panel_area = multiplexer.get_element_area(ElementType::LeftPanel);
    let left_panel = LeftPanel::new(
        requests.clone(),
        left_panel_area.size.to_logical(window.scale_factor()),
        left_panel_area.position.to_logical(window.scale_factor()),
    );
    let mut left_panel_debug = Debug::new();
    let mut left_panel_state = program::State::new(
        left_panel,
        convert_size(left_panel_area.size),
        conversion::cursor_position(cursor_position, window.scale_factor()),
        &mut renderer,
        &mut left_panel_debug,
    );

    // Run event loop
    let mut last_render_time = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        // Wait for event or redraw a frame every 33 ms (30 frame per seconds)
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(33));

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
                    // Feed the event to the multiplexer
                    let event = multiplexer.event(event);

                    if let Some((event, area)) = event {
                        // pass the event to the area on which it happenened
                        match area {
                            ElementType::TopBar => {
                                let event = iced_winit::conversion::window_event(
                                    &event,
                                    window.scale_factor(),
                                    modifiers,
                                );
                                if let Some(event) = event {
                                    top_bar_state.queue_event(event);
                                }
                            }
                            ElementType::Scene => {
                                let cursor_position = multiplexer.get_cursor_position();
                                if draw_flat {
                                    flat_scene.lock().unwrap().input(&event, cursor_position);
                                } else {
                                    scene.lock().unwrap().input(&event, cursor_position);
                                }
                            }
                            ElementType::LeftPanel => {
                                let event = iced_winit::conversion::window_event(
                                    &event,
                                    window.scale_factor(),
                                    modifiers,
                                );
                                if let Some(event) = event {
                                    left_panel_state.queue_event(event);
                                }
                            }
                            ElementType::Unattributed => unreachable!(),
                        }
                    }
                }
            }
            Event::MainEventsCleared => {
                // When there is no more event to deal with
                let top_bar_area = multiplexer.get_element_area(ElementType::TopBar);

                // Treat eventual event that happenened in the gui top bar
                let top_bar_cursor = if multiplexer.foccused_element() == Some(ElementType::TopBar)
                {
                    multiplexer.get_cursor_position()
                } else {
                    PhysicalPosition::new(-1., -1.)
                };
                let mut redraw = false;
                if !top_bar_state.is_queue_empty() {
                    // We update iced
                    redraw = true;
                    let _ = top_bar_state.update(
                        convert_size(top_bar_area.size),
                        conversion::cursor_position(top_bar_cursor, window.scale_factor()),
                        None,
                        &mut renderer,
                        &mut top_bar_debug,
                    );
                    {
                        let mut requests = requests.lock().expect("requests");
                        if requests.fitting {
                            scene.lock().unwrap().fit_design();
                            requests.fitting = false;
                        }

                        if let Some(ref path) = requests.file_add {
                            let d_id = mediator.lock().unwrap().nb_design();
                            let design = Design::new_with_path(d_id, path);
                            if let Some(design) = design {
                                let design = Arc::new(Mutex::new(design));
                                mediator.lock().unwrap().add_design(design);
                            }
                            requests.file_add = None;
                        }

                        if requests.file_clear {
                            mediator.lock().unwrap().clear_designs();
                            requests.file_clear = false;
                        }

                        if let Some(ref path) = requests.file_save {
                            mediator.lock().unwrap().save_design(path);
                            requests.file_save = None;
                        }

                        if let Some(value) = requests.toggle_text {
                            mediator.lock().unwrap().toggle_text(value);
                            requests.toggle_text = None;
                        }

                        if let Some(value) = requests.toggle_scene {
                            draw_flat = value;
                            requests.toggle_scene = None;
                        }
                    }
                }

                // Treat eventual event that happenend in the gui left panel.
                let left_panel_cursor =
                    if multiplexer.foccused_element() == Some(ElementType::LeftPanel) {
                        multiplexer.get_cursor_position()
                    } else {
                        PhysicalPosition::new(-1., -1.)
                    };
                if !left_panel_state.is_queue_empty() {
                    redraw = true;
                    let _ = left_panel_state.update(
                        convert_size(window.inner_size()),
                        conversion::cursor_position(left_panel_cursor, window.scale_factor()),
                        None,
                        &mut renderer,
                        &mut top_bar_debug,
                    );
                    {
                        let mut requests = requests.lock().unwrap();
                        if let Some(selection_mode) = requests.selection_mode {
                            scene.lock().unwrap().change_selection_mode(selection_mode);
                            requests.selection_mode = None;
                        }

                        if let Some(action_mode) = requests.action_mode {
                            scene.lock().unwrap().change_action_mode(action_mode);
                            flat_scene.lock().unwrap().change_action_mode(action_mode);
                            requests.action_mode = None;
                        }

                        if let Some(sequence) = requests.sequence_change.take() {
                            mediator.lock().unwrap().change_sequence(sequence);
                        }
                        if let Some(color) = requests.strand_color_change {
                            mediator.lock().unwrap().change_strand_color(color);
                            requests.strand_color_change = None;
                        }
                        if let Some(sensitivity) = requests.scroll_sensitivity.take() {
                            scene.lock().unwrap().change_sensitivity(sensitivity);
                            //flat_scene.lock().unwrap().change_sensitivity(sensitivity);
                        }
                    }
                }
                {
                    let mut messages = messages.lock().unwrap();
                    for m in messages.left_panel.drain(..) {
                        left_panel_state.queue_message(m);
                    }
                    for m in messages.top_bar.drain(..) {
                        top_bar_state.queue_message(m);
                    }
                }
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

                mediator.lock().unwrap().observe_designs();
                if redraw
                    | scene.lock().unwrap().need_redraw(dt)
                    | flat_scene.lock().unwrap().needs_redraw()
                {
                    window.request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                let top_bar_area = multiplexer.get_element_area(ElementType::TopBar);
                let top_bar_cursor = if multiplexer.foccused_element() == Some(ElementType::TopBar)
                {
                    multiplexer.get_cursor_position()
                } else {
                    PhysicalPosition::new(-1., -1.)
                };
                let left_panel_cursor =
                    if multiplexer.foccused_element() == Some(ElementType::LeftPanel) {
                        multiplexer.get_cursor_position()
                    } else {
                        PhysicalPosition::new(-1., -1.)
                    };
                if resized {
                    let window_size = window.inner_size();
                    let scene_area = multiplexer.get_element_area(ElementType::Scene);
                    scene
                        .lock()
                        .unwrap()
                        .notify(SceneNotification::NewSize(window_size, scene_area));
                    flat_scene.lock().unwrap().resize(window_size, scene_area);

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

                    let top_bar_area = multiplexer.get_element_area(ElementType::TopBar);
                    top_bar_state.queue_message(gui::top_bar::Message::Resize(
                        top_bar_area.size.to_logical(window.scale_factor()),
                    ));

                    let left_panel_area = multiplexer.get_element_area(ElementType::LeftPanel);
                    left_panel_state.queue_message(gui::left_panel::Message::Resized(
                        left_panel_area.size.to_logical(window.scale_factor()),
                        left_panel_area.position.to_logical(window.scale_factor()),
                    ));
                }
                // Get viewports from the partition

                // If there are events pending
                if !top_bar_state.is_queue_empty() || resized {
                    // We update iced
                    let _ = top_bar_state.update(
                        convert_size(top_bar_area.size),
                        conversion::cursor_position(top_bar_cursor, window.scale_factor()),
                        None,
                        &mut renderer,
                        &mut top_bar_debug,
                    );
                }

                if !left_panel_state.is_queue_empty() || resized {
                    let _ = left_panel_state.update(
                        convert_size(left_panel_area.size),
                        conversion::cursor_position(left_panel_cursor, window.scale_factor()),
                        None,
                        &mut renderer,
                        &mut left_panel_debug,
                    );
                }

                resized = false;

                let frame = swap_chain.get_current_frame().expect("Next frame");

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // We draw the scene first
                if draw_flat {
                    flat_scene
                        .lock()
                        .unwrap()
                        .draw_view(&mut encoder, &frame.output.view);
                } else {
                    scene
                        .lock()
                        .unwrap()
                        .draw_view(&mut encoder, &frame.output.view);
                }

                let viewport = Viewport::with_physical_size(
                    convert_size_u32(multiplexer.window_size),
                    window.scale_factor(),
                );

                let _left_panel_interaction = renderer.backend_mut().draw(
                    &device,
                    &mut staging_belt,
                    &mut encoder,
                    &frame.output.view,
                    &viewport,
                    left_panel_state.primitive(),
                    &left_panel_debug.overlay(),
                );

                // And then iced on top
                let mouse_interaction = renderer.backend_mut().draw(
                    &device,
                    &mut staging_belt,
                    &mut encoder,
                    &frame.output.view,
                    &viewport,
                    top_bar_state.primitive(),
                    &top_bar_debug.overlay(),
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

pub struct Messages {
    left_panel: VecDeque<gui::left_panel::Message>,
    #[allow(dead_code)]
    top_bar: VecDeque<gui::top_bar::Message>,
}

impl Messages {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            left_panel: VecDeque::new(),
            top_bar: VecDeque::new(),
        }
    }

    pub fn push_color(&mut self, color: u32) {
        let bytes = color.to_be_bytes();
        // bytes is [A, R, G, B]
        let color = iced::Color::from_rgb8(bytes[1], bytes[2], bytes[3]);
        self.left_panel
            .push_front(gui::left_panel::Message::StrandColorChanged(color));
    }

    pub fn push_sequence(&mut self, sequence: String) {
        self.left_panel
            .push_front(gui::left_panel::Message::SequenceChanged(sequence));
    }
}
