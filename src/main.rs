/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
//! `ensnano` is a software for designing 3D DNA nanostructures.
//!
//! # Organization of the software
//!
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
use std::collections::{HashMap, VecDeque};
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
pub type PhySize = iced_winit::winit::dpi::PhysicalSize<u32>;

use controller::{ChanelReader, ChanelReaderUpdate};
use ensnano_interactor::application::{Application, Notification};
use ensnano_interactor::{DesignOperation, DesignReader};
use iced_native::Event as IcedEvent;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Debug, Size};

use futures::task::SpawnExt;
use ultraviolet::{Rotor3, Vec3};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, ModifiersState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

#[macro_use]
extern crate serde_derive;
extern crate serde;

#[cfg(not(test))]
const MUST_TEST: bool = false;

#[cfg(test)]
const MUST_TEST: bool = true;

mod consts;
/// Design handling
//mod design;
/// Graphical interface drawing
mod gui;
//use design::Design;
//mod mediator;
/// Separation of the window into drawing regions
mod multiplexer;
/// 3D scene drawing
mod scene;
use ensnano_interactor::{
    graphics::{DrawArea, ElementType, SplitMode},
    operation::Operation,
    ActionMode, Selection, SelectionMode,
};
mod flatscene;
mod scheduler;
mod text;
mod utils;
use scheduler::Scheduler;

#[cfg(test)]
mod main_tests;
// mod grid_panel; We don't use the grid panel atm

mod app_state;
mod controller;
use app_state::AppState;
use controller::Action;
use controller::Controller;

mod requests;
pub use requests::Requests;

mod dialog;
use dialog::*;

use flatscene::FlatScene;
use gui::{ColorOverlay, Gui, IcedMessages, OverlayType, UiSize};
use multiplexer::{Multiplexer, Overlay};
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
    window.set_title("ENSnano");
    window.set_min_inner_size(Some(PhySize::new(100, 100)));

    println!("scale factor {}", window.scale_factor());

    let modifiers = ModifiersState::default();

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    // Initialize WGPU
    let (device, queue) = futures::executor::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Could not get adapter\n
                This might be because gpu drivers are missing. \n
                You need Vulkan, Metal (for MacOS) or DirectX (for Windows) drivers to run this software");

        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
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
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Mailbox,
            },
        )
    };
    let settings = Settings {
        antialiasing: Some(iced_graphics::Antialiasing::MSAAx4),
        default_text_size: gui::UiSize::Medium.main_text(),
        default_font: Some(include_bytes!("../font/ensnano2.ttf")),
        ..Default::default()
    };
    let mut renderer = Renderer::new(Backend::new(&device, settings.clone()));
    let device = Rc::new(device);
    let queue = Rc::new(queue);
    let mut resized = false;
    let mut scale_factor_changed = false;
    let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
    let mut local_pool = futures::executor::LocalPool::new();

    // Initialize the mediator
    let requests = Arc::new(Mutex::new(Requests::default()));
    let messages = Arc::new(Mutex::new(IcedMessages::new()));
    let computing = Arc::new(Mutex::new(false));
    let mut scheduler = Scheduler::new();

    // Initialize the layout
    let mut multiplexer = Multiplexer::new(
        window.inner_size(),
        window.scale_factor(),
        device.clone(),
        requests.clone(),
    );
    multiplexer.change_split(SplitMode::Both);

    // Initialize the scenes
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let scene_area = multiplexer.get_element_area(ElementType::Scene).unwrap();
    let scene = Arc::new(Mutex::new(Scene::new(
        device.clone(),
        queue.clone(),
        window.inner_size(),
        scene_area,
        requests.clone(),
        &mut encoder,
        Default::default(),
    )));
    queue.submit(Some(encoder.finish()));
    scheduler.add_application(scene.clone(), ElementType::Scene);

    let flat_scene = Arc::new(Mutex::new(FlatScene::new(
        device.clone(),
        queue.clone(),
        window.inner_size(),
        scene_area,
        requests.clone(),
        Default::default(),
    )));
    scheduler.add_application(flat_scene.clone(), ElementType::FlatScene);

    // Initialize the UI

    let mut gui = gui::Gui::new(
        device.clone(),
        &window,
        &multiplexer,
        requests.clone(),
        settings,
    );

    let mut overlay_manager = OverlayManager::new(requests.clone(), &window, &mut renderer);

    // Run event loop
    let mut last_render_time = std::time::Instant::now();
    let mut mouse_interaction = iced::mouse::Interaction::Pointer;
    let mut icon = None;

    let main_state_constructor = MainStateConstructor {
        messages: messages.clone(),
    };

    let mut main_state = MainState::new(main_state_constructor);
    main_state
        .applications
        .insert(ElementType::Scene, scene.clone());
    main_state
        .applications
        .insert(ElementType::FlatScene, flat_scene.clone());

    // Add a design to the scene if one was given as a command line arguement
    if path.is_some() {
        main_state.push_action(Action::LoadDesign(path))
    }

    let mut controller = Controller::new();

    event_loop.run(move |event, _, control_flow| {
        // Wait for event or redraw a frame every 33 ms (30 frame per seconds)
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(33));

        let main_state_view = MainStateView {
            main_state: &mut main_state,
            control_flow,
            multiplexer: &mut multiplexer,
            gui: &mut gui,
            scheduler: &mut scheduler,
            window: &window,
            resized: false,
        };

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => main_state_view
                .main_state
                .pending_actions
                .push_back(Action::Exit),
            Event::WindowEvent {
                event: WindowEvent::ModifiersChanged(modifiers),
                ..
            } => {
                multiplexer.update_modifiers(modifiers.clone());
                messages.lock().unwrap().update_modifiers(modifiers.clone());
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { .. },
                ..
            }
            | Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(_),
                ..
            } if gui.has_keyboard_priority() => {
                if let Event::WindowEvent { event, .. } = event {
                    if let Some(event) = event.to_static() {
                        let event = iced_winit::conversion::window_event(
                            &event,
                            window.scale_factor(),
                            modifiers,
                        );
                        if let Some(event) = event {
                            gui.forward_event_all(event);
                        }
                    }
                }
            }
            Event::WindowEvent { event, .. } => {
                //let modifiers = multiplexer.modifiers();
                if let Some(event) = event.to_static() {
                    // Feed the event to the multiplexer
                    let (event, icon_opt) =
                        multiplexer.event(event, &mut resized, &mut scale_factor_changed);
                    icon = icon.or(icon_opt);

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
                                let state = main_state.get_app_state();
                                scheduler.forward_event(&event, area, cursor_position, state)
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
            Event::MainEventsCleared => {
                scale_factor_changed |= multiplexer.check_scale_factor(&window);
                let mut redraw = resized | scale_factor_changed | icon.is_some();
                redraw |= gui.fetch_change(&window, &multiplexer);

                // When there is no more event to deal with
                requests::poll_all(requests.lock().unwrap(), &mut main_state);

                let mut main_state_view = MainStateView {
                    main_state: &mut main_state,
                    control_flow,
                    multiplexer: &mut multiplexer,
                    gui: &mut gui,
                    scheduler: &mut scheduler,
                    window: &window,
                    resized: false,
                };

                controller.make_progress(&mut main_state_view);
                resized |= main_state_view.resized;

                for update in main_state.chanel_reader.get_updates() {
                    if let ChanelReaderUpdate::ScaffoldShiftOptimizationProgress(x) = update {
                        main_state
                            .messages
                            .lock()
                            .unwrap()
                            .push_progress("Optimizing: ".to_string(), x);
                    } else if let ChanelReaderUpdate::ScaffoldShiftOptimizationResult(result) =
                        update
                    {
                        main_state.messages.lock().unwrap().finish_progess();
                        main_state
                            .apply_operation(DesignOperation::SetScaffoldShift(result.position));
                        let msg = format!(
                            "Scaffold position set to {}\n {}",
                            result.position, result.score
                        );
                        main_state.pending_actions.push_back(Action::ErrorMsg(msg));
                    }
                }

                main_state.update();

                // Treat eventual event that happenend in the gui left panel.
                let _overlay_change =
                    overlay_manager.fetch_change(&multiplexer, &window, &mut renderer);
                {
                    let mut messages = messages.lock().unwrap();
                    gui.forward_messages(&mut messages);
                    overlay_manager.forward_messages(&mut messages);
                }

                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                redraw |= scheduler.check_redraw(&multiplexer, dt, main_state.get_app_state());
                last_render_time = now;

                if redraw {
                    window.request_redraw();
                }
            }
            Event::RedrawRequested(_)
                if window.inner_size().width > 0 && window.inner_size().height > 0 =>
            {
                if resized {
                    multiplexer.generate_textures();
                    scheduler.forward_new_size(window.inner_size(), &multiplexer);
                    let window_size = window.inner_size();

                    swap_chain = device.create_swap_chain(
                        &surface,
                        &wgpu::SwapChainDescriptor {
                            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                            format,
                            width: window_size.width,
                            height: window_size.height,
                            present_mode: wgpu::PresentMode::Mailbox,
                        },
                    );

                    gui.resize(&multiplexer, &window);
                }
                if scale_factor_changed {
                    multiplexer.generate_textures();
                    gui.notify_scale_factor_change(&window, &multiplexer);
                    println!("lolz");
                    scheduler.forward_new_size(window.inner_size(), &multiplexer);
                    let window_size = window.inner_size();

                    swap_chain = device.create_swap_chain(
                        &surface,
                        &wgpu::SwapChainDescriptor {
                            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
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
                messages.lock().unwrap().push_application_state(
                    main_state.get_app_state().clone(),
                    !main_state.undo_stack.is_empty(),
                    !main_state.redo_stack.is_empty(),
                );
                gui.update(&multiplexer, &window);

                overlay_manager.process_event(&mut renderer, resized, &multiplexer, &window);

                resized = false;
                scale_factor_changed = false;

                if let Ok(frame) = swap_chain.get_current_frame() {
                    let mut encoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    // We draw the applications first
                    let now = std::time::Instant::now();
                    let dt = now - last_render_time;
                    scheduler.draw_apps(&mut encoder, &multiplexer, dt);

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
                    window.set_cursor_icon(iced_winit::conversion::mouse_interaction(
                        mouse_interaction,
                    ));
                    if let Some(icon) = icon.take() {
                        window.set_cursor_icon(icon);
                    }
                    local_pool
                        .spawner()
                        .spawn(staging_belt.recall())
                        .expect("Recall staging buffers");

                    local_pool.run_until_stalled();
                } else {
                    println!("Error getting next frame, attempt to recreate swap chain");
                    resized = true;
                }
            }
            _ => {}
        }
    })
}

pub struct OverlayManager {
    color_state: iced_native::program::State<ColorOverlay<Requests>>,
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
            let mut clipboard = iced_native::clipboard::Null;
            match overlay {
                OverlayType::Color => {
                    if !self.color_state.is_queue_empty() || resized {
                        let _ = self.color_state.update(
                            convert_size(PhysicalSize::new(250, 250)),
                            conversion::cursor_position(cursor_position, window.scale_factor()),
                            renderer,
                            &mut clipboard,
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

    fn forward_messages(&mut self, messages: &mut IcedMessages<AppState>) {
        ()
        /*
        for m in messages.color_overlay.drain(..) {
            self.color_state.queue_message(m);
        }*/
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
            let mut clipboard = iced_native::clipboard::Null;
            match overlay {
                OverlayType::Color => {
                    if !self.color_state.is_queue_empty() {
                        ret = true;
                        let _ = self.color_state.update(
                            convert_size(PhysicalSize::new(250, 250)),
                            conversion::cursor_position(cursor_position, window.scale_factor()),
                            renderer,
                            &mut clipboard,
                            &mut self.color_debug,
                        );
                    }
                }
            }
        }
        ret
    }
}

fn formated_path_end(path: &PathBuf) -> String {
    let components: Vec<_> = path.components().map(|comp| comp.as_os_str()).collect();
    let mut ret = if components.len() > 3 {
        vec!["..."]
    } else {
        vec![]
    };
    let mut iter = components.iter().rev().take(3).rev();
    for _ in 0..3 {
        if let Some(comp) = iter.next().and_then(|s| s.to_str()) {
            ret.push(comp.clone());
        }
    }
    ret.join("/")
}

/// The state of the main event loop.
pub(crate) struct MainState {
    app_state: AppState,
    pending_actions: VecDeque<Action>,
    undo_stack: Vec<AppState>,
    redo_stack: Vec<AppState>,
    chanel_reader: ChanelReader,
    messages: Arc<Mutex<IcedMessages<AppState>>>,
    applications: HashMap<ElementType, Arc<Mutex<dyn Application<AppState = AppState>>>>,
}

struct MainStateConstructor {
    messages: Arc<Mutex<IcedMessages<AppState>>>,
}

use controller::SaveDesignError;
impl MainState {
    fn new(constructor: MainStateConstructor) -> Self {
        Self {
            app_state: Default::default(),
            pending_actions: VecDeque::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            chanel_reader: Default::default(),
            messages: constructor.messages,
            applications: Default::default(),
        }
    }

    fn push_action(&mut self, action: Action) {
        self.pending_actions.push_back(action)
    }

    fn get_app_state(&mut self) -> AppState {
        self.app_state.update();
        self.app_state.clone()
    }

    fn clear_app_state(&mut self, new_state: AppState) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.app_state = new_state;
    }

    fn update(&mut self) {
        self.app_state.update()
    }

    fn update_candidates(&mut self, candidates: Vec<Selection>) {
        self.modify_state(|s| s.with_candidates(candidates), false);
    }

    fn update_selection(&mut self, selection: Vec<Selection>) {
        self.modify_state(|s| s.with_selection(selection), true);
    }

    fn apply_operation(&mut self, operation: DesignOperation) {
        match self.app_state.apply_design_op(operation) {
            Ok(Some(old_state)) => self.save_old_state(old_state),
            Ok(None) => (),
            Err(e) => println!("{:?}", e),
        }
    }

    fn save_old_state(&mut self, old_state: AppState) {
        self.undo_stack.push(old_state);
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(state) = self.undo_stack.pop() {
            let redo = std::mem::replace(&mut self.app_state, state);
            self.redo_stack.push(redo);
        }
    }

    fn redo(&mut self) {
        if let Some(state) = self.redo_stack.pop() {
            let undo = std::mem::replace(&mut self.app_state, state);
            self.undo_stack.push(undo);
        }
    }

    fn modify_state<F>(&mut self, modification: F, undoable: bool)
    where
        F: FnOnce(AppState) -> AppState,
    {
        let state = std::mem::take(&mut self.app_state);
        let old_state = state.clone();
        self.app_state = modification(state);
        if old_state != self.app_state && undoable {
            self.undo_stack.push(old_state);
            self.redo_stack.clear();
        }
    }

    fn update_pending_operation(&mut self, operation: Arc<dyn Operation>) {
        todo!()
    }

    fn request_copy(&mut self) {
        println!("TODO copy is not yet implemented");
    }

    fn request_paste(&mut self) {
        println!("TODO copy is not yet implemented");
    }

    fn request_duplication(&mut self) {
        println!("TODO copy is not yet implemented");
    }

    fn split_2d_view(&mut self) {
        println!("TODO split2d view is not yet implemented");
    }

    fn clear_visibility_sieve(&mut self) {
        println!("TODO");
    }

    fn redim_2d_helices(&mut self, all: bool) {
        println!("TODO");
    }

    fn stop_roll(&mut self) {
        println!("TODO")
    }

    fn save_design(&self, path: &PathBuf) -> Result<(), SaveDesignError> {
        self.app_state.get_design_reader().save_design(path)
    }

    fn change_selection_mode(&mut self, mode: SelectionMode) {
        self.modify_state(|s| s.with_selection_mode(mode), false)
    }

    fn change_action_mode(&mut self, mode: ActionMode) {
        self.modify_state(|s| s.with_action_mode(mode), false)
    }

    fn toggle_widget_basis(&mut self) {
        self.modify_state(|s| s.with_toggled_widget_basis(), false)
    }
}

/// A temporary view of the main state and the control flow.
struct MainStateView<'a> {
    main_state: &'a mut MainState,
    control_flow: &'a mut ControlFlow,
    multiplexer: &'a mut Multiplexer,
    scheduler: &'a mut Scheduler,
    gui: &'a mut Gui<Requests, AppState>,
    window: &'a Window,
    resized: bool,
}

use controller::{LoadDesignError, MainState as MainStateInteface, StaplesDownloader};
impl<'a> MainStateInteface for MainStateView<'a> {
    fn pop_action(&mut self) -> Option<Action> {
        self.main_state.pending_actions.pop_front()
    }

    fn exit_control_flow(&mut self) {
        *self.control_flow = ControlFlow::Exit
    }

    fn new_design(&mut self) {
        self.main_state.clear_app_state(Default::default())
    }

    fn oxdna_export(&mut self, path: &PathBuf) -> std::io::Result<(PathBuf, PathBuf)> {
        self.main_state.app_state.oxdna_export(path)
    }

    fn load_design(&mut self, path: PathBuf) -> Result<(), LoadDesignError> {
        if let Ok(state) = AppState::import_design(&path) {
            self.main_state.clear_app_state(state);
            Ok(())
        } else {
            Err(LoadDesignError::from("\"Oh No\"".to_string()))
        }
    }

    fn get_chanel_reader(&mut self) -> &mut ChanelReader {
        &mut self.main_state.chanel_reader
    }

    fn apply_operation(&mut self, operation: DesignOperation) {
        self.main_state.apply_operation(operation)
    }

    fn undo(&mut self) {
        self.main_state.undo();
    }

    fn redo(&mut self) {
        self.main_state.redo();
    }

    fn get_staple_downloader(&self) -> Box<dyn StaplesDownloader> {
        Box::new(self.main_state.app_state.get_design_reader())
    }

    fn save_design(&mut self, path: &PathBuf) -> Result<(), SaveDesignError> {
        self.main_state.save_design(path)?;
        Ok(())
    }

    fn toggle_split_mode(&mut self, mode: SplitMode) {
        self.multiplexer.change_split(mode);
        self.scheduler
            .forward_new_size(self.window.inner_size(), self.multiplexer);
        self.gui.resize(self.multiplexer, self.window);
    }

    fn change_ui_size(&mut self, ui_size: UiSize) {
        self.gui
            .new_ui_size(ui_size.clone(), self.window, self.multiplexer);
        self.multiplexer
            .change_ui_size(ui_size.clone(), self.window);
        self.main_state
            .messages
            .lock()
            .unwrap()
            .new_ui_size(ui_size);
        self.resized = true;
        //messages.lock().unwrap().new_ui_size(ui_size);
    }

    fn invert_scroll_y(&mut self, inverted: bool) {
        self.multiplexer.invert_y_scroll = inverted;
    }

    fn notify_apps(&mut self, notificiation: Notification) {
        for app in self.main_state.applications.values_mut() {
            app.lock().unwrap().on_notify(notificiation.clone())
        }
    }

    fn get_selection(&mut self) -> Box<dyn AsRef<[Selection]>> {
        Box::new(self.main_state.app_state.get_selection())
    }

    fn get_design_reader(&mut self) -> Box<dyn DesignReader> {
        Box::new(self.main_state.app_state.get_design_reader())
    }

    fn get_grid_creation_position(&self) -> Option<(Vec3, Rotor3)> {
        self.main_state
            .applications
            .get(&ElementType::Scene)
            .and_then(|s| s.lock().unwrap().get_position_for_new_grid())
    }

    fn finish_changing_color(&mut self) {
        self.main_state.modify_state(
            |s| s.notified(app_state::InteractorNotification::FinishChangingColor),
            false,
        )
    }
}

use controller::{SetScaffoldSequenceError, SetScaffoldSequenceOk};
impl<'a> controller::ScaffoldSetter for MainStateView<'a> {
    fn set_scaffold_sequence(
        &mut self,
        sequence: String,
    ) -> Result<SetScaffoldSequenceOk, SetScaffoldSequenceError> {
        match self
            .main_state
            .app_state
            .apply_design_op(DesignOperation::SetScaffoldSequence(sequence))
        {
            Ok(Some(old_state)) => self.main_state.save_old_state(old_state),
            Ok(None) => (),
            Err(e) => return Err(SetScaffoldSequenceError(format!("{:?}", e))),
        };
        let default_shift = self.get_staple_downloader().default_shift();
        Ok(SetScaffoldSequenceOk { default_shift })
    }

    fn optimize_shift(&mut self) {
        todo!()
    }
}

fn apply_update<T: Default, F>(obj: &mut T, update_func: F)
where
    F: FnOnce(T) -> T,
{
    let tmp = std::mem::take(obj);
    *obj = update_func(tmp);
}
