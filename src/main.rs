use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
pub type PhySize = iced_winit::winit::dpi::PhysicalSize<u32>;

use iced_native::Event as IcedEvent;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Debug, Size};

use futures::task::SpawnExt;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, ModifiersState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
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
use mediator::{Mediator, Scheduler};
mod flatscene;
mod text;
mod utils;
// mod grid_panel; We don't use the grid panel atm

use flatscene::FlatScene;
use gui::{ColorOverlay, LeftPanel, OverlayType, Requests, TopBar};
use multiplexer::{DrawArea, ElementType, Multiplexer, Overlay, SplitMode};
use scene::Scene;

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
    let messages = Arc::new(Mutex::new(IcedMessages::new()));
    let mediator = Arc::new(Mutex::new(Mediator::new(messages.clone())));
    let scheduler = Arc::new(Mutex::new(Scheduler::new()));

    // Initialize the layout
    let mut multiplexer =
        Multiplexer::new(window.inner_size(), window.scale_factor(), device.clone());

    // Initialize the scenes
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let scene_area = multiplexer.get_element_area(ElementType::Scene).unwrap();
    let scene = Arc::new(Mutex::new(Scene::new(
        device.clone(),
        queue.clone(),
        window.inner_size(),
        scene_area,
        mediator.clone(),
        &mut encoder,
    )));
    queue.submit(Some(encoder.finish()));
    mediator
        .lock()
        .unwrap()
        .add_application(scene.clone(), ElementType::Scene);
    scheduler
        .lock()
        .unwrap()
        .add_application(scene.clone(), ElementType::Scene);

    let flat_scene = Arc::new(Mutex::new(FlatScene::new(
        device.clone(),
        queue.clone(),
        window.inner_size(),
        scene_area,
    )));
    mediator
        .lock()
        .unwrap()
        .add_application(flat_scene.clone(), ElementType::FlatScene);
    scheduler
        .lock()
        .unwrap()
        .add_application(flat_scene.clone(), ElementType::FlatScene);

    // Add a design to the scene if one was given as a command line arguement
    if let Some(ref path) = path {
        let design = Design::new_with_path(0, path);
        if let Some(design) = design {
            let design = Arc::new(Mutex::new(design));
            mediator.lock().unwrap().add_design(design);
        }
    }

    // Initialize the UI
    let requests = Arc::new(Mutex::new(Requests::new()));

    // Top bar
    let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();
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
    let left_panel_area = multiplexer
        .get_element_area(ElementType::LeftPanel)
        .unwrap();
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

    let mut overlay_manager = OverlayManager::new(requests.clone(), &window, &mut renderer);
    let mut redraw_top_bar = true;
    let mut redraw_left_panel = true;

    // Run event loop
    let mut last_render_time = std::time::Instant::now();
    let mut mouse_interaction = iced::mouse::Interaction::Pointer;
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
                    redraw_top_bar = true;
                    redraw_left_panel = true;
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
                                    redraw_top_bar = true;
                                }
                            }
                            ElementType::Overlay(n) => {
                                let event = iced_winit::conversion::window_event(
                                    &event,
                                    window.scale_factor(),
                                    modifiers,
                                );
                                if let Some(event) = event {
                                    overlay_manager.forward_event(event, n);
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
                                    redraw_left_panel = true;
                                }
                            }
                            _ => {
                                let cursor_position = multiplexer.get_cursor_position();
                                scheduler.lock().unwrap().forward_event(
                                    &event,
                                    area,
                                    cursor_position,
                                )
                            }
                        }
                    }
                }
            }
            Event::MainEventsCleared => {
                // When there is no more event to deal with
                let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();

                // Treat eventual event that happenened in the gui top bar
                let top_bar_cursor = if multiplexer.foccused_element() == Some(ElementType::TopBar)
                {
                    multiplexer.get_cursor_position()
                } else {
                    PhysicalPosition::new(-1., -1.)
                };
                if !top_bar_state.is_queue_empty() {
                    // We update iced
                    redraw_top_bar = true;
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
                            mediator.lock().unwrap().request_fits();
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
                            multiplexer.change_split(value);
                            scheduler
                                .lock()
                                .unwrap()
                                .forward_new_size(window.inner_size(), &multiplexer);
                            redraw_left_panel = true;
                            redraw_top_bar = true;
                            requests.toggle_scene = None;
                        }

                        if requests.make_grids {
                            mediator.lock().unwrap().make_grids();
                            requests.make_grids = false
                        }
                    }
                }

                // Treat eventual event that happenend in the gui left panel.
                let overlay_change =
                    overlay_manager.fetch_change(&multiplexer, &window, &mut renderer);
                let left_panel_cursor =
                    if multiplexer.foccused_element() == Some(ElementType::LeftPanel) {
                        multiplexer.get_cursor_position()
                    } else {
                        PhysicalPosition::new(-1., -1.)
                    };
                if !left_panel_state.is_queue_empty() || overlay_change {
                    redraw_left_panel |= !left_panel_state.is_queue_empty();
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
                            mediator
                                .lock()
                                .unwrap()
                                .change_selection_mode(selection_mode);
                            requests.selection_mode = None;
                        }

                        if let Some(action_mode) = requests.action_mode {
                            mediator.lock().unwrap().change_action_mode(action_mode);
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
                            mediator.lock().unwrap().change_sensitivity(sensitivity);
                            //flat_scene.lock().unwrap().change_sensitivity(sensitivity);
                        }

                        if let Some(overlay_type) = requests.overlay_closed.take() {
                            overlay_manager.rm_overlay(overlay_type, &mut multiplexer);
                        }

                        if let Some(overlay_type) = requests.overlay_opened.take() {
                            overlay_manager.add_overlay(overlay_type, &mut multiplexer);
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
                    overlay_manager.forward_messages(&mut messages);
                }

                mediator.lock().unwrap().observe_designs();
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();
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
                    scheduler
                        .lock()
                        .unwrap()
                        .forward_new_size(window.inner_size(), &multiplexer);
                    let window_size = window.inner_size();
                    /*
                    let scene_area = multiplexer.get_element_area(ElementType::Scene);
                    scene
                        .lock()
                        .unwrap()
                        .notify(SceneNotification::NewSize(window_size, scene_area));
                    flat_scene.lock().unwrap().resize(window_size, scene_area);
                    */

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

                    let top_bar_area = multiplexer.get_element_area(ElementType::TopBar).unwrap();
                    top_bar_state.queue_message(gui::top_bar::Message::Resize(
                        top_bar_area.size.to_logical(window.scale_factor()),
                    ));

                    let left_panel_area = multiplexer
                        .get_element_area(ElementType::LeftPanel)
                        .unwrap();
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
                        convert_size(window.inner_size()),
                        conversion::cursor_position(left_panel_cursor, window.scale_factor()),
                        None,
                        &mut renderer,
                        &mut left_panel_debug,
                    );
                }

                overlay_manager.process_event(&mut renderer, resized, &multiplexer, &window);

                resized = false;

                let frame = swap_chain.get_current_frame().expect("Next frame");

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // We draw the applications first
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                scheduler
                    .lock()
                    .unwrap()
                    .draw_apps(&mut encoder, &multiplexer, dt);
                last_render_time = now;

                if redraw_left_panel {
                    let viewport_left_panel = Viewport::with_physical_size(
                        convert_size_u32(
                            multiplexer
                                .get_element_area(ElementType::LeftPanel)
                                .unwrap()
                                .size,
                        ),
                        window.scale_factor(),
                    );
                    let _left_panel_interaction = renderer.backend_mut().draw(
                        &device,
                        &mut staging_belt,
                        &mut encoder,
                        multiplexer
                            .get_texture_view(ElementType::LeftPanel)
                            .unwrap(),
                        &viewport_left_panel,
                        left_panel_state.primitive(),
                        &left_panel_debug.overlay(),
                    );
                    redraw_left_panel = false;
                }

                if redraw_top_bar {
                    let viewport_top_bar = Viewport::with_physical_size(
                        convert_size_u32(
                            multiplexer
                                .get_element_area(ElementType::TopBar)
                                .unwrap()
                                .size,
                        ),
                        window.scale_factor(),
                    );
                    mouse_interaction = renderer.backend_mut().draw(
                        &device,
                        &mut staging_belt,
                        &mut encoder,
                        multiplexer.get_texture_view(ElementType::TopBar).unwrap(),
                        &viewport_top_bar,
                        top_bar_state.primitive(),
                        &top_bar_debug.overlay(),
                    );
                    redraw_top_bar = false;
                }

                multiplexer.draw(&mut encoder, &frame.output.view);
                //overlay_manager.render(&device, &mut staging_belt, &mut encoder, &frame.output.view, &multiplexer, &window, &mut renderer);

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

/// Message sent to the gui component
pub struct IcedMessages {
    left_panel: VecDeque<gui::left_panel::Message>,
    #[allow(dead_code)]
    top_bar: VecDeque<gui::top_bar::Message>,
    color_overlay: VecDeque<gui::left_panel::ColorMessage>,
}

impl IcedMessages {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            left_panel: VecDeque::new(),
            top_bar: VecDeque::new(),
            color_overlay: VecDeque::new(),
        }
    }

    pub fn push_color(&mut self, color: u32) {
        let bytes = color.to_be_bytes();
        // bytes is [A, R, G, B]
        let color = iced::Color::from_rgb8(bytes[1], bytes[2], bytes[3]);
        self.color_overlay
            .push_front(gui::left_panel::ColorMessage::StrandColorChanged(color));
    }

    pub fn push_sequence(&mut self, sequence: String) {
        self.left_panel
            .push_front(gui::left_panel::Message::SequenceChanged(sequence));
    }
}

pub struct OverlayManager {
    color_state: iced_native::program::State<ColorOverlay>,
    color_debug: Debug,
    overlay_types: Vec<OverlayType>,
    overlays: Vec<Overlay>,
}

impl OverlayManager {
    pub fn new(requests: Arc<Mutex<Requests>>, window: &Window, renderer: &mut Renderer) -> Self {
        let color = ColorOverlay::new(
            requests.clone(),
            PhysicalSize::new(250., 250.).to_logical(window.scale_factor()),
        );
        let mut color_debug = Debug::new();
        let color_state = program::State::new(
            color,
            convert_size(PhysicalSize::new(250, 250)),
            conversion::cursor_position(PhysicalPosition::new(-1f64, -1f64), window.scale_factor()),
            renderer,
            &mut color_debug,
        );
        Self {
            color_state,
            color_debug,
            overlay_types: Vec::new(),
            overlays: Vec::new(),
        }
    }

    fn forward_event(&mut self, event: IcedEvent, n: usize) {
        match self.overlay_types.get(n) {
            None => {
                println!("recieve event from non existing overlay");
                unreachable!();
            }
            Some(OverlayType::Color) => self.color_state.queue_event(event),
        }
    }

    fn add_overlay(&mut self, overlay_type: OverlayType, multiplexer: &mut Multiplexer) {
        match overlay_type {
            OverlayType::Color => self.overlays.push(Overlay {
                position: PhysicalPosition::new(500, 500),
                size: PhysicalSize::new(250, 250),
            }),
        }
        self.overlay_types.push(overlay_type);
        self.update_multiplexer(multiplexer);
    }

    fn process_event(
        &mut self,
        renderer: &mut Renderer,
        resized: bool,
        multiplexer: &Multiplexer,
        window: &Window,
    ) {
        for (n, overlay) in self.overlay_types.iter().enumerate() {
            let cursor_position = if multiplexer.foccused_element() == Some(ElementType::Overlay(n))
            {
                multiplexer.get_cursor_position()
            } else {
                PhysicalPosition::new(-1., -1.)
            };
            match overlay {
                OverlayType::Color => {
                    if !self.color_state.is_queue_empty() || resized {
                        let _ = self.color_state.update(
                            convert_size(PhysicalSize::new(250, 250)),
                            conversion::cursor_position(cursor_position, window.scale_factor()),
                            None,
                            renderer,
                            &mut self.color_debug,
                        );
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn render(
        &self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        multiplexer: &Multiplexer,
        window: &Window,
        renderer: &mut Renderer,
    ) {
        for overlay_type in self.overlay_types.iter() {
            match overlay_type {
                OverlayType::Color => {
                    let color_viewport = Viewport::with_physical_size_and_position(
                        convert_size_u32(multiplexer.window_size),
                        (500, 500),
                        window.scale_factor(),
                    );
                    let _color_interaction = renderer.backend_mut().draw(
                        &device,
                        staging_belt,
                        encoder,
                        &target,
                        &color_viewport,
                        self.color_state.primitive(),
                        &self.color_debug.overlay(),
                    );
                }
            }
        }
    }

    fn rm_overlay(&mut self, overlay_type: OverlayType, multiplexer: &mut Multiplexer) {
        let mut rm_idx = Vec::new();
        for (idx, overlay_type_) in self.overlay_types.iter().rev().enumerate() {
            if *overlay_type_ == overlay_type {
                rm_idx.push(idx);
            }
        }
        for idx in rm_idx.iter() {
            self.overlays.remove(*idx);
            self.overlay_types.remove(*idx);
        }
        self.update_multiplexer(multiplexer);
    }

    fn update_multiplexer(&self, multiplexer: &mut Multiplexer) {
        multiplexer.set_overlays(self.overlays.clone())
    }

    fn forward_messages(&mut self, messages: &mut IcedMessages) {
        for m in messages.color_overlay.drain(..) {
            self.color_state.queue_message(m);
        }
    }

    fn fetch_change(
        &mut self,
        multiplexer: &Multiplexer,
        window: &Window,
        renderer: &mut Renderer,
    ) -> bool {
        let mut ret = false;
        for (n, overlay) in self.overlay_types.iter().enumerate() {
            let cursor_position = if multiplexer.foccused_element() == Some(ElementType::Overlay(n))
            {
                multiplexer.get_cursor_position()
            } else {
                PhysicalPosition::new(-1., -1.)
            };
            match overlay {
                OverlayType::Color => {
                    if !self.color_state.is_queue_empty() {
                        ret = true;
                        let _ = self.color_state.update(
                            convert_size(PhysicalSize::new(250, 250)),
                            conversion::cursor_position(cursor_position, window.scale_factor()),
                            None,
                            renderer,
                            &mut self.color_debug,
                        );
                    }
                }
            }
        }
        ret
    }
}
