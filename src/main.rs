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
//!
//! ## Handling of events
//!
//! The Global state of the program is encoded in an automata defined in the
//! [controller](controller) module. This global state determines weither inputs are handled
//! normally or if the program should wait for the user to interact with dialog windows.
//!
//! When the Global automata is in NormalState, events are forwarded to the
//! [Multiplexer](multiplexer) which decides what application should handle the event. This is usually the
//! application displayed in the active region (the region under the cursor). Special events like resizing of the window are
//! handled by the multiplexer.
//!
//! When GUIs handle an event. They recieve a reference to the state of the main program. This
//! state is encoded in the [AppState](app_state::AppState) data structure. Each GUI components
//! needs to be able to recieve some specific information about the state of the program to handle
//! events and to draw their views. Theese needs are encoded in traits. GUI component typically
//! defines their own `AppState` trait that must be implemented by the concrete `AppState` type.
//!
//! GUI components may interpret event as a request from the user to modify the design or the state
//! of the main application (for example by changing the selection). These requests are stored in
//! the [Requests](requests::Requests) data structure. Each application defines a `Requests` trait
//! that must be implemented by the concrete `Requests` type.
//!
//! On each itteration of the main event loop, if the Global controller is in Normal State,
//! requests are polled and transmitted to the main `AppState` my the main controller. The
//! processing of these requests may have 3 different kind of consequences:
//!
//!  * An undoable action is performed on the main `AppState`, modifiying it. In that case the
//!  current `AppState` is copied on the undo stack and the replaced by the modified one.
//!
//!  * A non-undoable action is perfomed on the main `AppState`, modyfing it. In that case, the
//!  current `AppState` is replaced by the modified one, but not stored on the undo stack.
//!  This typically happens when the `AppState` is in a transient state for example while the user
//!  is performing a drag and drop action. Transient states are not stored on the undo stack
//!  because they are not meant to be restored by undos.
//!   
//!  * An error is returned. In the case the `AppState` is not modified and the user is notified of
//!  the error. Error typically occur when user attempt to make actions on the design that are not
//!  permitted by the current state of the program. For example an error is returned if the user
//!  try to modify the design durring a simulation.
//!
use std::collections::{HashMap, VecDeque};
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
pub type PhySize = iced_winit::winit::dpi::PhysicalSize<u32>;
const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

use controller::{ChanelReader, ChanelReaderUpdate, SimulationRequest};
use ensnano_design::{Camera, Nucl};
use ensnano_interactor::application::{Application, Notification};
use ensnano_interactor::{
    CenterOfSelection, DesignOperation, DesignReader, RigidBodyConstants, SuggestionParameters,
};
use iced_native::Event as IcedEvent;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::winit::event::VirtualKeyCode;
use iced_winit::{conversion, futures, program, winit, Debug, Size};

use futures::task::SpawnExt;
use ultraviolet::{Rotor3, Vec3};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, ModifiersState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

#[allow(unused_imports)]
#[macro_use]
extern crate pretty_env_logger;

#[macro_use]
extern crate paste;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

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
use app_state::{AppState, CopyOperation, ErrOperation, PastingStatus, SimulationTarget};
use controller::Action;
use controller::Controller;

mod requests;
pub use requests::Requests;

mod dialog;

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

#[cfg(not(feature = "log_after_renderer_setup"))]
const EARLY_LOG: bool = true;
#[cfg(feature = "log_after_renderer_setup")]
const EARLY_LOG: bool = false;

#[cfg(not(feature = "dx12_only"))]
const BACKEND: wgpu::Backends = wgpu::Backends::PRIMARY;
#[cfg(feature = "dx12_only")]
const BACKEND: wgpu::Backends = wgpu::Backends::DX12;

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
    if EARLY_LOG {
        pretty_env_logger::init();
    }
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
    let mut windows_title = String::from("ENSnano");
    window.set_title("ENSnano");
    window.set_min_inner_size(Some(PhySize::new(100, 100)));

    log::info!("scale factor {}", window.scale_factor());

    let modifiers = ModifiersState::default();

    let instance = wgpu::Instance::new(BACKEND);
    let surface = unsafe { instance.create_surface(&window) };
    // Initialize WGPU
    let (device, queue) = futures::executor::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
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
    device.on_uncaptured_error(|e| log::error!("wgpu error {:?}", e));

    {
        let size = window.inner_size();

        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: TEXTURE_FORMAT,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Mailbox,
            },
        )
    }

    let settings = Settings {
        antialiasing: Some(iced_graphics::Antialiasing::MSAAx4),
        default_text_size: gui::UiSize::Medium.main_text(),
        default_font: Some(include_bytes!("../font/ensnano2.ttf")),
        ..Default::default()
    };
    let mut renderer = Renderer::new(Backend::new(&device, settings.clone(), TEXTURE_FORMAT));
    let device = Rc::new(device);
    let queue = Rc::new(queue);
    let mut resized = false;
    let mut scale_factor_changed = false;
    let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
    let mut local_pool = futures::executor::LocalPool::new();

    // Initialize the mediator
    let requests = Arc::new(Mutex::new(Requests::default()));
    let messages = Arc::new(Mutex::new(IcedMessages::new()));
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
    let mut multiplexer_cursor = None;

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
    main_state.update();
    main_state.last_saved_state = main_state.app_state.clone();

    let mut controller = Controller::new();

    println!("{}", consts::WELCOME_MSG);
    if !EARLY_LOG {
        pretty_env_logger::init();
    }

    let mut first_iteration = true;

    let mut last_gui_state = (
        main_state.app_state.clone(),
        main_state.gui_state(&multiplexer),
    );
    event_loop.run(move |event, _, control_flow| {
        // Wait for event or redraw a frame every 33 ms (30 frame per seconds)
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(33));

        let mut main_state_view = MainStateView {
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
                event: WindowEvent::Focused(false),
                ..
            } => main_state_view.notify_apps(Notification::WindowFocusLost),
            Event::WindowEvent {
                event: WindowEvent::ModifiersChanged(modifiers),
                ..
            } => {
                main_state_view
                    .multiplexer
                    .update_modifiers(modifiers.clone());
                messages.lock().unwrap().update_modifiers(modifiers.clone());
                main_state_view.notify_apps(Notification::ModifersChanged(modifiers));
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } if input.virtual_keycode == Some(VirtualKeyCode::Escape)
                && window.fullscreen().is_some() =>
            {
                window.set_fullscreen(None)
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
                    let event = multiplexer.event(event, &mut resized, &mut scale_factor_changed);

                    if let Some((event, area)) = event {
                        // pass the event to the area on which it happenened
                        if main_state.focussed_element != Some(area) {
                            if let Some(app) = main_state
                                .focussed_element
                                .as_ref()
                                .and_then(|elt| main_state.applications.get(elt))
                            {
                                app.lock().unwrap().on_notify(Notification::WindowFocusLost)
                            }
                            main_state.focussed_element = Some(area);
                            main_state.update_candidates(vec![]);
                        }
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
                                scheduler.forward_event(&event, area, cursor_position, state);
                                if matches!(event, winit::event::WindowEvent::MouseInput { .. }) {
                                    gui.clear_foccus();
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
            Event::MainEventsCleared => {
                scale_factor_changed |= multiplexer.check_scale_factor(&window);
                let mut redraw =
                    resized || scale_factor_changed || multiplexer.icon != multiplexer_cursor;
                multiplexer_cursor = multiplexer.icon.clone();
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

                if main_state_view.main_state.wants_fit {
                    main_state_view.notify_apps(Notification::FitRequest);
                    main_state_view.main_state.wants_fit = false;
                }
                controller.make_progress(&mut main_state_view);
                resized |= main_state_view.resized;
                resized |= first_iteration;
                first_iteration = false;

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
                        if let Ok(result) = result {
                            main_state.apply_operation(DesignOperation::SetScaffoldShift(
                                result.position,
                            ));
                            let msg = format!(
                                "Scaffold position set to {}\n {}",
                                result.position, result.score
                            );
                            main_state.pending_actions.push_back(Action::ErrorMsg(msg));
                        } else {
                            // unwrap because in this block, result is necessarilly an Err
                            log::warn!("{:?}", result.err().unwrap());
                        }
                    } else if let ChanelReaderUpdate::SimulationUpdate(update) = update {
                        main_state.app_state.apply_simulation_update(update)
                    } else if let ChanelReaderUpdate::SimulationExpired = update {
                        main_state.update_simulation(SimulationRequest::Stop)
                    }
                }

                main_state.update();
                let new_title = if let Some(path) = main_state.get_current_file_name() {
                    let path_str = formated_path_end(path);
                    format!("ENSnano {}", path_str)
                } else {
                    format!("ENSnano {}", crate::consts::NO_DESIGN_TITLE)
                };

                if windows_title != new_title {
                    window.set_title(&new_title);
                    windows_title = new_title;
                }

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
                let new_gui_state = (
                    main_state.app_state.clone(),
                    main_state.gui_state(&multiplexer),
                );
                if new_gui_state != last_gui_state {
                    last_gui_state = new_gui_state;
                    messages.lock().unwrap().push_application_state(
                        main_state.get_app_state().clone(),
                        last_gui_state.1.clone(),
                    );
                    redraw = true;
                };
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

                    surface.configure(
                        &device,
                        &wgpu::SurfaceConfiguration {
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            format: TEXTURE_FORMAT,
                            width: window_size.width,
                            height: window_size.height,
                            present_mode: wgpu::PresentMode::Mailbox,
                        },
                    );

                    gui.resize(&multiplexer, &window);
                    log::trace!(
                        "Will draw on texture of size {}x {}",
                        window_size.width,
                        window_size.height
                    );
                }
                if scale_factor_changed {
                    multiplexer.generate_textures();
                    gui.notify_scale_factor_change(&window, &multiplexer);
                    log::info!("Notified of scale factor change: {}", window.scale_factor());
                    scheduler.forward_new_size(window.inner_size(), &multiplexer);
                    let window_size = window.inner_size();

                    surface.configure(
                        &device,
                        &wgpu::SurfaceConfiguration {
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            format: TEXTURE_FORMAT,
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
                scale_factor_changed = false;

                if let Ok(frame) = surface.get_current_texture() {
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

                    if multiplexer.resize(window.inner_size(), window.scale_factor()) {
                        resized = true;
                        window.request_redraw();
                        return;
                    }
                    log::trace!("window size {:?}", window.inner_size());
                    multiplexer.draw(
                        &mut encoder,
                        &frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                        &window,
                    );
                    //overlay_manager.render(&device, &mut staging_belt, &mut encoder, &frame.output.view, &multiplexer, &window, &mut renderer);

                    // Then we submit the work
                    staging_belt.finish();
                    queue.submit(Some(encoder.finish()));
                    frame.present();

                    // And update the mouse cursor
                    let iced_icon = iced_winit::conversion::mouse_interaction(mouse_interaction);
                    window.set_cursor_icon(multiplexer.icon.unwrap_or(iced_icon));
                    local_pool
                        .spawner()
                        .spawn(staging_belt.recall())
                        .expect("Recall staging buffers");

                    local_pool.run_until_stalled();
                } else {
                    log::warn!("Error getting next frame, attempt to recreate swap chain");
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
                log::error!("recieve event from non existing overlay");
                unreachable!();
            }
            Some(OverlayType::Color) => self.color_state.queue_event(event),
        }
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn update_multiplexer(&self, multiplexer: &mut Multiplexer) {
        multiplexer.set_overlays(self.overlays.clone())
    }

    fn forward_messages(&mut self, _messages: &mut IcedMessages<AppState>) {
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

fn formated_path_end<P: AsRef<Path>>(path: P) -> String {
    let components: Vec<_> = path
        .as_ref()
        .components()
        .map(|comp| comp.as_os_str())
        .collect();
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
    focussed_element: Option<ElementType>,
    last_saved_state: AppState,
    path_to_current_design: Option<PathBuf>,
    file_name: Option<PathBuf>,
    wants_fit: bool,
    last_backup_date: Instant,
}

struct MainStateConstructor {
    messages: Arc<Mutex<IcedMessages<AppState>>>,
}

use controller::SaveDesignError;
impl MainState {
    fn new(constructor: MainStateConstructor) -> Self {
        let app_state = AppState::default();
        Self {
            app_state: app_state.clone(),
            pending_actions: VecDeque::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            chanel_reader: Default::default(),
            messages: constructor.messages,
            applications: Default::default(),
            focussed_element: None,
            last_saved_state: app_state.clone(),
            path_to_current_design: None,
            file_name: None,
            wants_fit: false,
            last_backup_date: Instant::now(),
        }
    }

    fn push_action(&mut self, action: Action) {
        self.pending_actions.push_back(action)
    }

    fn get_app_state(&mut self) -> AppState {
        self.app_state.clone()
    }

    fn new_design(&mut self) {
        self.clear_app_state(Default::default());
        self.path_to_current_design = None;
        self.update_current_file_name();
    }

    fn clear_app_state(&mut self, new_state: AppState) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.app_state = new_state.clone();
        self.last_saved_state = new_state;
    }

    fn update(&mut self) {
        self.app_state.update()
    }

    fn update_candidates(&mut self, candidates: Vec<Selection>) {
        self.modify_state(|s| s.with_candidates(candidates), false);
    }

    fn transfer_selection_pivot_to_group(&mut self, group_id: ensnano_design::GroupId) {
        use scene::AppState;
        let scene_pivot = self
            .applications
            .get(&ElementType::Scene)
            .and_then(|app| app.lock().unwrap().get_current_selection_pivot());
        if let Some(pivot) = self.app_state.get_current_group_pivot().or(scene_pivot) {
            self.apply_operation(DesignOperation::SetGroupPivot { group_id, pivot })
        }
    }

    fn update_selection(
        &mut self,
        selection: Vec<Selection>,
        group_id: Option<ensnano_organizer::GroupId>,
    ) {
        self.modify_state(|s| s.with_selection(selection, group_id), true);
    }

    fn update_center_of_selection(&mut self, center: Option<CenterOfSelection>) {
        self.modify_state(|s| s.with_center_of_selection(center), false)
    }

    fn apply_copy_operation(&mut self, operation: CopyOperation) {
        let result = self.app_state.apply_copy_operation(operation);
        self.apply_operation_result(result);
    }

    fn apply_operation(&mut self, operation: DesignOperation) {
        log::debug!("Applying operation {:?}", operation);
        let result = self.app_state.apply_design_op(operation.clone());
        if let Err(ErrOperation::FinishFirst) = result {
            self.modify_state(
                |s| s.notified(app_state::InteractorNotification::FinishOperation),
                false,
            );
            self.apply_operation(operation);
        } else {
            self.apply_operation_result(result);
        }
    }

    fn start_helix_simulation(&mut self, parameters: RigidBodyConstants) {
        let result = self.app_state.start_simulation(
            parameters,
            &mut self.chanel_reader,
            SimulationTarget::Helices,
        );
        self.apply_operation_result(result)
    }

    fn start_grid_simulation(&mut self, parameters: RigidBodyConstants) {
        let result = self.app_state.start_simulation(
            parameters,
            &mut self.chanel_reader,
            SimulationTarget::Grids,
        );
        self.apply_operation_result(result)
    }

    fn start_roll_simulation(&mut self, target_helices: Option<Vec<usize>>) {
        let result = self.app_state.start_simulation(
            Default::default(),
            &mut self.chanel_reader,
            SimulationTarget::Roll { target_helices },
        );
        self.apply_operation_result(result)
    }

    fn update_simulation(&mut self, request: SimulationRequest) {
        let result = self.app_state.update_simulation(request);
        self.apply_operation_result(result);
    }

    fn apply_silent_operation(&mut self, operation: DesignOperation) {
        match self.app_state.apply_design_op(operation.clone()) {
            Ok(_) => (),
            Err(ErrOperation::FinishFirst) => {
                self.modify_state(
                    |s| s.notified(app_state::InteractorNotification::FinishOperation),
                    false,
                );
                self.apply_silent_operation(operation)
            }
            Err(e) => log::warn!("{:?}", e),
        }
    }

    fn save_old_state(&mut self, old_state: AppState) {
        self.undo_stack.push(old_state);
        self.redo_stack.clear();
    }

    fn set_roll_of_selected_helices(&mut self, roll: f32) {
        if let Some((_, helices)) =
            ensnano_interactor::list_of_helices(self.app_state.get_selection().as_ref())
        {
            self.apply_operation(DesignOperation::SetRollHelices { helices, roll })
        }
    }

    fn undo(&mut self) {
        if let Some(mut state) = self.undo_stack.pop() {
            state.prepare_for_replacement(&self.app_state);
            let mut redo = std::mem::replace(&mut self.app_state, state);
            redo = redo.notified(app_state::InteractorNotification::FinishOperation);
            if redo.is_in_stable_state() {
                self.redo_stack.push(redo);
            }
        }
    }

    fn redo(&mut self) {
        if let Some(mut state) = self.redo_stack.pop() {
            state.prepare_for_replacement(&self.app_state);
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
        if old_state != self.app_state && undoable && old_state.is_in_stable_state() {
            self.undo_stack.push(old_state);
            self.redo_stack.clear();
        }
    }

    fn update_pending_operation(&mut self, operation: Arc<dyn Operation>) {
        let result = self.app_state.update_pending_operation(operation.clone());
        if let Err(ErrOperation::FinishFirst) = result {
            self.modify_state(
                |s| s.notified(app_state::InteractorNotification::FinishOperation),
                false,
            );
            self.update_pending_operation(operation)
        }
        self.apply_operation_result(result);
    }

    fn optimize_shift(&mut self) {
        let reader = &mut self.chanel_reader;
        let result = self.app_state.optimize_shift(reader);
        self.apply_operation_result(result);
    }

    fn apply_operation_result(&mut self, result: Result<Option<AppState>, ErrOperation>) {
        match result {
            Ok(Some(old_state)) => self.save_old_state(old_state),
            Ok(None) => (),
            Err(e) => log::warn!("{:?}", e),
        }
    }

    fn request_copy(&mut self) {
        let reader = self.app_state.get_design_reader();
        if let Some((_, xover_ids)) = ensnano_interactor::list_of_xover_as_nucl_pairs(
            self.app_state.get_selection().as_ref(),
            &reader,
        ) {
            self.apply_copy_operation(CopyOperation::CopyXovers(xover_ids))
        } else {
            let strand_ids = ensnano_interactor::extract_strands_from_selection(
                self.app_state.get_selection().as_ref(),
            );
            self.apply_copy_operation(CopyOperation::CopyStrands(strand_ids))
        }
    }

    fn apply_paste(&mut self) {
        log::info!("apply paste");
        match self.app_state.is_pasting() {
            PastingStatus::Copy => self.apply_copy_operation(CopyOperation::Paste),
            PastingStatus::Duplication => self.apply_copy_operation(CopyOperation::Duplicate),
            _ => (),
        }
    }

    fn request_duplication(&mut self) {
        if self.app_state.can_iterate_duplication() {
            self.apply_copy_operation(CopyOperation::Duplicate)
        } else {
            if let Some((_, nucl_pairs)) = ensnano_interactor::list_of_xover_as_nucl_pairs(
                self.app_state.get_selection().as_ref(),
                &self.app_state.get_design_reader(),
            ) {
                self.apply_copy_operation(CopyOperation::InitXoverDuplication(nucl_pairs))
            } else {
                let strand_ids = ensnano_interactor::extract_strands_from_selection(
                    self.app_state.get_selection().as_ref(),
                );
                self.apply_copy_operation(CopyOperation::InitStrandsDuplication(strand_ids))
            }
        }
    }

    fn save_design(&mut self, path: &PathBuf) -> Result<(), SaveDesignError> {
        let camera = self
            .applications
            .get(&ElementType::Scene)
            .and_then(|s| s.lock().unwrap().get_camera())
            .map(|(position, orientation)| Camera {
                id: Default::default(),
                name: String::from("Saved Camera"),
                position,
                orientation,
            });
        let save_info = ensnano_design::SavingInformation { camera };
        self.app_state
            .get_design_reader()
            .save_design(path, save_info)?;
        self.last_saved_state = self.app_state.clone();
        self.path_to_current_design = Some(path.clone());
        self.update_current_file_name();
        Ok(())
    }

    fn save_backup(&mut self) -> Result<(), SaveDesignError> {
        let camera = self
            .applications
            .get(&ElementType::Scene)
            .and_then(|s| s.lock().unwrap().get_camera())
            .map(|(position, orientation)| Camera {
                id: Default::default(),
                name: String::from("Saved Camera"),
                position,
                orientation,
            });
        let save_info = ensnano_design::SavingInformation { camera };
        let path = if let Some(mut path) = self.path_to_current_design.clone() {
            path.set_extension(crate::consts::ENS_BACKUP_EXTENSION);
            path
        } else {
            let mut ret = dirs::document_dir().or(dirs::home_dir()).ok_or_else(|| {
                self.last_backup_date =
                    Instant::now() + Duration::from_secs(crate::consts::SEC_PER_YEAR);
                SaveDesignError::cannot_open_default_dir()
            })?;
            ret.push(crate::consts::ENS_UNAMED_FILE_NAME);
            ret.set_extension(crate::consts::ENS_BACKUP_EXTENSION);
            ret
        };
        self.app_state
            .get_design_reader()
            .save_design(&path, save_info)?;

        println!("Saved backup to {}", path.to_string_lossy());
        Ok(())
    }

    fn change_selection_mode(&mut self, mode: SelectionMode) {
        self.modify_state(|s| s.with_selection_mode(mode), false)
    }

    fn change_action_mode(&mut self, mode: ActionMode) {
        self.modify_state(|s| s.with_action_mode(mode), false)
    }

    fn change_double_strand_parameters(&mut self, parameters: Option<(isize, usize)>) {
        self.modify_state(|s| s.with_strand_on_helix(parameters), false)
    }

    fn toggle_widget_basis(&mut self) {
        self.modify_state(|s| s.with_toggled_widget_basis(), false)
    }

    fn set_visibility_sieve(&mut self, selection: Vec<Selection>, compl: bool) {
        let result = self.app_state.set_visibility_sieve(selection, compl);
        self.apply_operation_result(result)
    }

    fn need_save(&self) -> bool {
        self.app_state.design_was_modified(&self.last_saved_state)
    }

    fn get_current_file_name(&self) -> Option<&Path> {
        self.file_name.as_ref().map(|p| p.as_ref())
    }

    fn update_current_file_name(&mut self) {
        self.file_name = self
            .path_to_current_design
            .as_ref()
            .filter(|p| p.is_file())
            .map(|p| p.into())
    }

    fn set_suggestion_parameters(&mut self, param: SuggestionParameters) {
        self.modify_state(|s| s.with_suggestion_parameters(param), false)
    }

    fn gui_state(&self, multiplexer: &Multiplexer) -> gui::MainState {
        gui::MainState {
            can_undo: !self.undo_stack.is_empty(),
            can_redo: !self.redo_stack.is_empty(),
            need_save: self.need_save(),
            can_reload: self.get_current_file_name().is_some(),
            can_split2d: multiplexer.is_showing(&ElementType::FlatScene),
            splited_2d: self
                .applications
                .get(&ElementType::FlatScene)
                .map(|app| app.lock().unwrap().is_splited())
                .unwrap_or(false),
        }
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
        if self.main_state.pending_actions.len() > 0 {
            log::debug!("pending actions {:?}", self.main_state.pending_actions);
        }
        self.main_state.pending_actions.pop_front()
    }

    fn need_backup(&self) -> bool {
        Instant::now() - self.main_state.last_backup_date
            > Duration::from_secs(crate::consts::SEC_BETWEEN_BACKUPS)
    }

    fn exit_control_flow(&mut self) {
        *self.control_flow = ControlFlow::Exit
    }

    fn new_design(&mut self) {
        self.main_state.new_design()
    }

    fn oxdna_export(&mut self, path: &PathBuf) -> std::io::Result<(PathBuf, PathBuf)> {
        self.main_state.app_state.oxdna_export(path)
    }

    fn load_design(&mut self, mut path: PathBuf) -> Result<(), LoadDesignError> {
        if let Ok(state) = AppState::import_design(&path) {
            self.main_state.clear_app_state(state);
            if path.extension().map(|s| s.to_string_lossy())
                == Some(crate::consts::ENS_BACKUP_EXTENSION.into())
            {
                path.set_extension(crate::consts::ENS_EXTENSION);
            }
            self.main_state.path_to_current_design = Some(path.clone());
            if let Some((position, orientation)) = self
                .main_state
                .app_state
                .get_design_reader()
                .get_favourite_camera()
            {
                self.notify_apps(Notification::TeleportCamera(position, orientation));
            } else {
                self.main_state.wants_fit = true;
            }
            self.main_state.update_current_file_name();
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

    fn apply_silent_operation(&mut self, operation: DesignOperation) {
        self.main_state.apply_silent_operation(operation)
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
        self.main_state.last_backup_date = Instant::now();
        Ok(())
    }

    fn save_backup(&mut self) -> Result<(), SaveDesignError> {
        self.main_state.save_backup()?;
        self.main_state.last_backup_date = Instant::now();
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

    fn finish_operation(&mut self) {
        self.main_state.modify_state(
            |s| s.notified(app_state::InteractorNotification::FinishOperation),
            false,
        );
        self.main_state.app_state.finish_operation();
    }

    fn request_copy(&mut self) {
        self.main_state.request_copy()
    }

    fn init_paste(&mut self) {
        self.main_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(None));
    }

    fn apply_paste(&mut self) {
        self.main_state.apply_paste();
    }

    fn duplicate(&mut self) {
        self.main_state.request_duplication();
    }

    fn request_pasting_candidate(&mut self, candidate: Option<Nucl>) {
        self.main_state
            .apply_copy_operation(CopyOperation::PositionPastingPoint(candidate))
    }

    fn delete_selection(&mut self) {
        let selection = self.get_selection();
        if let Some((_, nucl_pairs)) = ensnano_interactor::list_of_xover_as_nucl_pairs(
            selection.as_ref().as_ref(),
            self.get_design_reader().as_ref(),
        ) {
            self.main_state.update_selection(vec![], None);
            self.main_state
                .apply_operation(DesignOperation::RmXovers { xovers: nucl_pairs })
        } else if let Some((_, strand_ids)) =
            ensnano_interactor::list_of_strands(selection.as_ref().as_ref())
        {
            self.main_state.update_selection(vec![], None);
            self.main_state
                .apply_operation(DesignOperation::RmStrands { strand_ids })
        } else if let Some((_, h_ids)) =
            ensnano_interactor::list_of_helices(selection.as_ref().as_ref())
        {
            self.main_state.update_selection(vec![], None);
            self.main_state
                .apply_operation(DesignOperation::RmHelices { h_ids })
        }
    }

    fn scaffold_to_selection(&mut self) {
        let scaffold_id = self
            .main_state
            .get_app_state()
            .get_design_reader()
            .get_scaffold_info()
            .map(|info| info.id);
        if let Some(s_id) = scaffold_id {
            self.main_state
                .update_selection(vec![Selection::Strand(0, s_id as u32)], None)
        }
    }

    fn start_helix_simulation(&mut self, parameters: RigidBodyConstants) {
        self.main_state.start_helix_simulation(parameters);
    }

    fn start_grid_simulation(&mut self, parameters: RigidBodyConstants) {
        self.main_state.start_grid_simulation(parameters);
    }

    fn start_roll_simulation(&mut self, target_helices: Option<Vec<usize>>) {
        self.main_state.start_roll_simulation(target_helices);
    }

    fn update_simulation(&mut self, request: SimulationRequest) {
        self.main_state.update_simulation(request)
    }

    fn set_roll_of_selected_helices(&mut self, roll: f32) {
        self.main_state.set_roll_of_selected_helices(roll)
    }

    fn turn_selection_into_anchor(&mut self) {
        let selection = self.get_selection();
        let nucls = ensnano_interactor::extract_nucls_from_selection(selection.as_ref().as_ref());

        self.main_state
            .apply_operation(DesignOperation::FlipAnchors { nucls });
    }

    fn set_visibility_sieve(&mut self, compl: bool) {
        let selection = self.get_selection().as_ref().as_ref().to_vec();
        self.main_state.set_visibility_sieve(selection, compl);
    }

    fn clear_visibility_sieve(&mut self) {
        self.main_state.set_visibility_sieve(vec![], true);
    }

    fn need_save(&self) -> bool {
        self.main_state.need_save()
    }

    fn get_current_design_directory(&self) -> Option<&Path> {
        let mut ancestors = self
            .main_state
            .path_to_current_design
            .as_ref()
            .map(|p| p.ancestors())?;
        let first_ancestor = ancestors.next()?;
        if first_ancestor.is_dir() {
            Some(first_ancestor)
        } else {
            let second_ancestor = ancestors.next()?;
            if second_ancestor.is_dir() {
                Some(second_ancestor)
            } else {
                None
            }
        }
    }

    fn get_current_file_name(&self) -> Option<&Path> {
        self.main_state.get_current_file_name()
    }

    fn set_current_group_pivot(&mut self, pivot: ensnano_design::group_attributes::GroupPivot) {
        if let Some(group_id) = self.main_state.app_state.get_current_group_id() {
            self.apply_operation(DesignOperation::SetGroupPivot { group_id, pivot })
        } else {
            self.main_state.app_state.set_current_group_pivot(pivot);
        }
    }

    fn translate_group_pivot(&mut self, translation: Vec3) {
        use ensnano_interactor::{DesignTranslation, IsometryTarget};
        if let Some(group_id) = self.main_state.app_state.get_current_group_id() {
            self.apply_operation(DesignOperation::Translation(DesignTranslation {
                target: IsometryTarget::GroupPivot(group_id),
                translation,
                group_id: None,
            }))
        } else {
            self.main_state.app_state.translate_group_pivot(translation);
        }
    }

    fn rotate_group_pivot(&mut self, rotation: Rotor3) {
        use ensnano_interactor::{DesignRotation, IsometryTarget};
        if let Some(group_id) = self.main_state.app_state.get_current_group_id() {
            self.apply_operation(DesignOperation::Rotation(DesignRotation {
                target: IsometryTarget::GroupPivot(group_id),
                rotation,
                origin: Vec3::zero(),
                group_id: None,
            }))
        } else {
            self.main_state.app_state.rotate_group_pivot(rotation);
        }
    }

    fn create_new_camera(&mut self) {
        if let Some((position, orientation)) = self
            .main_state
            .applications
            .get(&ElementType::Scene)
            .and_then(|s| s.lock().unwrap().get_camera())
        {
            self.main_state
                .apply_operation(DesignOperation::CreateNewCamera {
                    position,
                    orientation,
                })
        } else {
            log::error!("Could not get current camera position");
        }
    }

    fn select_camera(&mut self, camera_id: ensnano_design::CameraId) {
        let reader = self.main_state.app_state.get_design_reader();
        if let Some((position, orientation)) = reader.get_camera_with_id(camera_id) {
            self.notify_apps(Notification::TeleportCamera(position, orientation))
        } else {
            log::error!("Could not get camera {:?}", camera_id)
        }
    }

    fn update_camera(&mut self, camera_id: ensnano_design::CameraId) {
        if let Some((position, orientation)) = self
            .main_state
            .applications
            .get(&ElementType::Scene)
            .and_then(|s| s.lock().unwrap().get_camera())
        {
            self.main_state
                .apply_operation(DesignOperation::UpdateCamera {
                    camera_id,
                    position,
                    orientation,
                })
        } else {
            log::error!("Could not get current camera position");
        }
    }

    fn select_favorite_camera(&mut self, n_camera: u32) {
        let reader = self.main_state.app_state.get_design_reader();
        if let Some((position, orientation)) = reader.get_nth_camera(n_camera) {
            self.notify_apps(Notification::TeleportCamera(position, orientation))
        } else {
            log::error!("Design has less than {} cameras", n_camera + 1);
        }
    }

    fn flip_split_views(&mut self) {
        self.notify_apps(Notification::FlipSplitViews)
    }
}

use controller::{SetScaffoldSequenceError, SetScaffoldSequenceOk};
impl<'a> controller::ScaffoldSetter for MainStateView<'a> {
    fn set_scaffold_sequence(
        &mut self,
        sequence: String,
        shift: usize,
    ) -> Result<SetScaffoldSequenceOk, SetScaffoldSequenceError> {
        match self
            .main_state
            .app_state
            .apply_design_op(DesignOperation::SetScaffoldSequence { sequence, shift })
        {
            Ok(Some(old_state)) => self.main_state.save_old_state(old_state),
            Ok(None) => (),
            Err(e) => return Err(SetScaffoldSequenceError(format!("{:?}", e))),
        };
        let default_shift = self.get_staple_downloader().default_shift();
        Ok(SetScaffoldSequenceOk { default_shift })
    }

    fn optimize_shift(&mut self) {
        self.main_state.optimize_shift();
    }
}

fn apply_update<T: Default, F>(obj: &mut T, update_func: F)
where
    F: FnOnce(T) -> T,
{
    let tmp = std::mem::take(obj);
    *obj = update_func(tmp);
}
