//! `icednano` is a software for designing 3D DNA nanostructures.
//!
//! # Organization of the software
//!
//! ![Organization
//! chart](https://perso.ens-lyon.fr/nicolas.levy/doc_icednano/img/main_chart.jpg)
//!
//! The [main](main) function owns the event_loop and the framebuffer. It recieves window events
//! and handles the framebuffer.
//!
//! ## Drawing process
//!
//! On each redraw request, the [main](main) funtion generates a new frame, and ask the
//! [Multiplexer](multiplexer) to draw on a view of that texture.
//!
//! The [Multiplexer](multiplexer) knows how the window is devided into several regions. For each
//! of these region it knows what application or gui component should draw on it.
//! For each region the [Multiplexer](multiplexer) holds a texture, and at each draw request, it
//! will request the corresponding app or gui element to possibly update the texture.
//!
//!  Applications are notified when the design that they display have been modified and may request
//!  from the [Design](design) the data that they need to display it.
//!
//!  ## Handling of events
//!
//!  Window events are recieved by the `main` function that forwards them to the
//!  [Multiplexer](multiplexer). The [Multiplexer](multiplexer) then forwards the event to the last
//!  active region (the region under the cursor). Special events like resizing of the window are
//!  handled by the multiplexer.
//!
//!  When applications and GUI component handle an event. This event might have consequences that
//!  must be known by the other components of the software.
//!
//!  In the case of an application, the
//!  consequences is transmitted to the [Mediator](mediator). The [Mediator](mediator) may then
//!  request appropriate modifications of the [Designs](design) or forward messages for the GUI
//!  components.
//!
//!  In the case of a GUI component, consequences are transmitted to the [main](main) function that
//!  will consequently send the appropriate request to the [Mediator](mediator).
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
use mediator::{Mediator, Operation, Scheduler};
mod flatscene;
mod text;
mod utils;
// mod grid_panel; We don't use the grid panel atm

use flatscene::FlatScene;
use gui::{ColorOverlay, OverlayType, Requests};
use multiplexer::{DrawArea, ElementType, Multiplexer, Overlay, SplitMode};
use scene::Scene;

fn convert_size(size: PhySize) -> Size<f32> {
    Size::new(size.width as f32, size.height as f32)
}

fn convert_size_u32(size: PhySize) -> Size<u32> {
    Size::new(size.width, size.height)
}

/// Main function. Runs the event loop and holds the framebuffer.
///
/// # Intialization
///
/// Before running the event loop, the main fuction do the following
///
/// * It request a connection to the GPU and crates a framebuffer
/// * It initializes a multiplexer.
/// * It initializes applications and GUI component, and associate region of the screen to these
/// components
/// * It initialized the [Mediator](mediator), the [Scheduler](mediator::Scheduler) and the [Gui
/// manager](gui::Gui)
///
/// # EventLoop
///
/// * The event loop wait for an event. If no event is recieved during 33ms, a new redraw is
/// requested.
/// * When a event is recieved, it is forwareded to the multiplexer. The Multiplexer may then
/// convert this event into a event for a specific screen region.
/// * When all window event have been handled, the main function reads messages that it recieved
/// from the [Gui Manager](gui::Gui).  The consequence of these messages are forwarded to the
/// applications.
/// * The main loops then reads the messages that it recieved from the [Mediator](mediator) and
/// forwards their consequences to the Gui components.
/// * Finally, a redraw is requested.
///
///
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
    let mut renderer = Renderer::new(Backend::new(
        &device,
        Settings {
            antialiasing: Some(iced_graphics::Antialiasing::MSAAx4),
            ..Default::default()
        },
    ));
    let device = Rc::new(device);
    let queue = Rc::new(queue);
    let mut resized = false;
    let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
    let mut local_pool = futures::executor::LocalPool::new();

    // Initialize the mediator
    let requests = Arc::new(Mutex::new(Requests::new()));
    let messages = Arc::new(Mutex::new(IcedMessages::new()));
    let mediator = Arc::new(Mutex::new(Mediator::new(messages.clone())));
    let scheduler = Arc::new(Mutex::new(Scheduler::new()));

    // Initialize the layout
    let mut multiplexer = Multiplexer::new(
        window.inner_size(),
        window.scale_factor(),
        device.clone(),
        requests.clone(),
    );

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
        mediator.clone(),
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
        let design = Design::new_with_path(0, path).unwrap_or_else(|| Design::new(0));
        mediator
            .lock()
            .unwrap()
            .add_design(Arc::new(Mutex::new(design)));
    } else {
        let design = Design::new(0);
        mediator
            .lock()
            .unwrap()
            .add_design(Arc::new(Mutex::new(design)));
    }

    // Initialize the UI

    let mut gui = gui::Gui::new(device.clone(), &window, &multiplexer, requests.clone());

    let mut overlay_manager = OverlayManager::new(requests.clone(), &window, &mut renderer);

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
                }
                if let Some(event) = event.to_static() {
                    // Feed the event to the multiplexer
                    let event = multiplexer.event(event);

                    if let Some((event, area)) = event {
                        // pass the event to the area on which it happenened
                        match area {
                            area if area.is_gui() => {
                                let event = iced_winit::conversion::window_event(
                                    &event,
                                    window.scale_factor(),
                                    modifiers,
                                );
                                if let Some(event) = event {
                                    gui.forward_event(area, event);
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
                            area if area.is_scene() => {
                                let cursor_position = multiplexer.get_cursor_position();
                                scheduler.lock().unwrap().forward_event(
                                    &event,
                                    area,
                                    cursor_position,
                                )
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
            Event::MainEventsCleared => {
                gui.fetch_change(&window, &multiplexer);
                // When there is no more event to deal with
                {
                    let mut requests = requests.lock().expect("requests");
                    if requests.fitting {
                        mediator.lock().unwrap().request_fits();
                        requests.fitting = false;
                    }

                    if let Some(ref path) = requests.file_add {
                        let design = Design::new_with_path(0, path);
                        if let Some(design) = design {
                            mediator.lock().unwrap().clear_designs();
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
                        gui.resize(&multiplexer, &window);
                        requests.toggle_scene = None;
                    }

                    if requests.make_grids {
                        mediator.lock().unwrap().make_grids();
                        requests.make_grids = false
                    }

                    if requests.new_grid {
                        scene.lock().unwrap().make_new_grid();
                        requests.new_grid = false;
                        messages
                            .lock()
                            .unwrap()
                            .push_action_mode(mediator::ActionMode::Build(false));
                        messages
                            .lock()
                            .unwrap()
                            .push_selection_mode(mediator::SelectionMode::Grid);
                    }

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

                    if let Some(op) = requests.operation_update.take() {
                        mediator.lock().unwrap().update_pending(op)
                    }

                    if let Some(b) = requests.toggle_persistent_helices.take() {
                        mediator.lock().unwrap().set_persistent_phantom(b)
                    }
                }

                // Treat eventual event that happenend in the gui left panel.
                let _overlay_change =
                    overlay_manager.fetch_change(&multiplexer, &window, &mut renderer);
                {
                    let mut messages = messages.lock().unwrap();
                    gui.forward_messages(&mut messages);
                    overlay_manager.forward_messages(&mut messages);
                }

                mediator.lock().unwrap().observe_designs();
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                if resized {
                    scheduler
                        .lock()
                        .unwrap()
                        .forward_new_size(window.inner_size(), &multiplexer);
                    let window_size = window.inner_size();

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

                    gui.resize(&multiplexer, &window);
                }
                // Get viewports from the partition

                // If there are events pending
                gui.update(&multiplexer, &window);

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

                gui.render(
                    &mut encoder,
                    &window,
                    &multiplexer,
                    &mut staging_belt,
                    &mut mouse_interaction,
                );

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
    status_bar: VecDeque<gui::status_bar::Message>,
}

impl IcedMessages {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            left_panel: VecDeque::new(),
            top_bar: VecDeque::new(),
            color_overlay: VecDeque::new(),
            status_bar: VecDeque::new(),
        }
    }

    pub fn push_color(&mut self, color: u32) {
        let bytes = color.to_be_bytes();
        // bytes is [A, R, G, B]
        let color = iced::Color::from_rgb8(bytes[1], bytes[2], bytes[3]);
        self.color_overlay
            .push_front(gui::left_panel::ColorMessage::StrandColorChanged(color));
        self.left_panel
            .push_front(gui::left_panel::Message::StrandColorChanged(color));
    }

    pub fn push_sequence(&mut self, sequence: String) {
        self.left_panel
            .push_front(gui::left_panel::Message::SequenceChanged(sequence));
    }

    pub fn push_op(&mut self, operation: Arc<dyn Operation>) {
        self.status_bar
            .push_front(gui::status_bar::Message::Operation(operation));
    }

    pub fn push_selection(&mut self, selection: mediator::Selection, values: Vec<String>) {
        self.status_bar
            .push_front(gui::status_bar::Message::Selection(selection, values))
    }

    pub fn clear_op(&mut self) {
        self.status_bar
            .push_front(gui::status_bar::Message::ClearOp);
    }

    pub fn push_action_mode(&mut self, action_mode: mediator::ActionMode) {
        self.left_panel
            .push_front(gui::left_panel::Message::ActionModeChanged(action_mode))
    }

    pub fn push_selection_mode(&mut self, selection_mode: mediator::SelectionMode) {
        self.left_panel
            .push_front(gui::left_panel::Message::SelectionModeChanged(
                selection_mode,
            ))
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
                    let color_viewport = Viewport::with_physical_size(
                        convert_size_u32(multiplexer.window_size),
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
