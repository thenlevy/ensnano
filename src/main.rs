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
use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
pub type PhySize = iced_winit::winit::dpi::PhysicalSize<u32>;

use chanel_reader::ChanelReader;
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

#[cfg(not(test))]
const MUST_TEST: bool = false;

#[cfg(test)]
const MUST_TEST: bool = true;

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
use mediator::{ActionMode, Mediator, Operation, ParameterPtr, Scheduler, SelectionMode};
mod flatscene;
mod text;
mod utils;
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

mod chanel_reader;

use flatscene::FlatScene;
use gui::{ColorOverlay, OverlayType};
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
    let mediator = Arc::new(Mutex::new(Mediator::new(
        messages.clone(),
        computing.clone(),
    )));
    let scheduler = Arc::new(Mutex::new(Scheduler::new()));

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
        mediator.clone(),
        &mut encoder,
        mediator.lock().unwrap().get_state(),
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
        mediator.lock().unwrap().get_state(),
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
        let design = Design::new_with_path(0, path).unwrap_or_else(|_| Design::new(0)); // TODO print error
        if let Some(tree) = design.get_organizer_tree() {
            messages.lock().unwrap().push_new_tree(tree)
        }
        mediator
            .lock()
            .unwrap()
            .add_design(Arc::new(RwLock::new(design)));
    } else {
        let design = Design::new(0);
        mediator
            .lock()
            .unwrap()
            .add_design(Arc::new(RwLock::new(design)));
    }

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
        scene: scene.clone(),
        flatscene: flat_scene.clone(),
        messages: messages.clone(),
    };

    let mut main_state = MainState::new(main_state_constructor);

    let mut controller = Controller::new(mediator.clone());

    event_loop.run(move |event, _, control_flow| {
        // Wait for event or redraw a frame every 33 ms (30 frame per seconds)
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(33));

        let main_state_view = MainStateView {
            main_state: &mut main_state,
            control_flow,
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
                mediator.lock().unwrap().update_modifiers(modifiers.clone());
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
                                let state = mediator.lock().unwrap().get_state();
                                scheduler.lock().unwrap().forward_event(
                                    &event,
                                    area,
                                    cursor_position,
                                    state,
                                )
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
                requests::poll_all(requests.lock().unwrap(), &mut main_state, mediator.clone());

                let mut main_state_view = MainStateView {
                    main_state: &mut main_state,
                    control_flow,
                };

                controller.make_progress(&mut main_state_view);

                /*
                let keep_proceed = requests.lock().unwrap().keep_proceed.pop_front();
                if let Some(proceed) = keep_proceed {
                    match proceed {
                        Action::CustomScaffold => {
                            messages.lock().unwrap().push_custom_scaffold()
                        }
                        Action::DefaultScaffold => {
                            messages.lock().unwrap().push_default_scaffold()
                        }
                        Action::OptimizeShift(d_id) => {
                            // start a shift optimization using Mediator::optimize_shift;
                            unimplemented!();
                            //mediator.lock().unwrap().optimize_shift(d_id);
                        }
                        Action::AskStaplesPath { d_id } => {
                            // Get the path in which to save the staples
                            // and proceed to download
                            let requests = requests.clone();
                            let dialog = rfd::AsyncFileDialog::new().save_file();
                            std::thread::spawn(move || {
                                let save_op = async move {
                                    let file = dialog.await;
                                    if let Some(handle) = file {
                                        let mut path_buf: std::path::PathBuf =
                                            handle.path().clone().into();
                                        path_buf.set_extension("xlsx");
                                        requests.lock().unwrap().keep_proceed.push_back(
                                            Action::DownloadStaples {
                                                target_file: path_buf,
                                                design_id: d_id,
                                            },
                                        );
                                    }
                                };
                                futures::executor::block_on(save_op);
                            });
                        }
                        Action::Quit => {
                            *control_flow = ControlFlow::Exit;
                        }
                        Action::SaveBeforeOpen => {
                            unimplemented!()
                            /*
                            messages
                                .lock()
                                .unwrap()
                                .push_save(Some(Action::LoadDesignAfterSave));*/
                        }
                        Action::SaveBeforeNew => {
                            unimplemented!();
                            /*messages
                            .lock()
                            .unwrap()
                            .push_save(Some(Action::NewDesignAfterSave));*/
                        }
                        Action::SaveBeforeQuit => {
                            unimplemented!();
                            //messages.lock().unwrap().push_save(Some(Action::Quit));
                        }
                        Action::LoadDesign => {
                            unimplemented!();
                            //messages.lock().unwrap().push_open();
                        }
                        Action::LoadDesignAfterSave => {
                            requests.lock().unwrap().keep_proceed.push_back(
                                Action::BlockingInfo(
                                    "Save successfully".into(),
                                    Box::new(Action::LoadDesign),
                                ),
                            );
                        }
                        Action::NewDesign => {
                            let design = Design::new(0);
                            messages.lock().unwrap().notify_new_design();
                            mediator.lock().unwrap().clear_designs();
                            mediator
                                .lock()
                                .unwrap()
                                .add_design(Arc::new(RwLock::new(design)));
                        }
                        Action::NewDesignAfterSave => {
                            requests.lock().unwrap().keep_proceed.push_back(
                                Action::BlockingInfo(
                                    "Save successfully".into(),
                                    Box::new(Action::NewDesign),
                                ),
                            );
                        }
                        Action::BlockingInfo(msg, keep_proceed) => blocking_message(
                            msg.into(),
                            rfd::MessageLevel::Info,
                            requests.clone(),
                            *keep_proceed,
                        ),
                        _ => (),
                    }
                }*/

                // Treat eventual event that happenend in the gui left panel.
                let _overlay_change =
                    overlay_manager.fetch_change(&multiplexer, &window, &mut renderer);
                {
                    let mut messages = messages.lock().unwrap();
                    gui.forward_messages(&mut messages);
                    overlay_manager.forward_messages(&mut messages);
                }

                mediator.lock().unwrap().observe_designs();
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                redraw |= scheduler.lock().unwrap().check_redraw(
                    &multiplexer,
                    dt,
                    mediator.lock().unwrap().get_state(),
                );
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
                    scheduler
                        .lock()
                        .unwrap()
                        .forward_new_size(window.inner_size(), &multiplexer);
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
                    scheduler
                        .lock()
                        .unwrap()
                        .forward_new_size(window.inner_size(), &multiplexer);
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
                    scheduler
                        .lock()
                        .unwrap()
                        .draw_apps(&mut encoder, &multiplexer, dt);

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

/// Message sent to the gui component
pub struct IcedMessages {
    left_panel: VecDeque<gui::left_panel::Message>,
    top_bar: VecDeque<gui::top_bar::Message>,
    color_overlay: VecDeque<gui::left_panel::ColorMessage>,
    status_bar: VecDeque<gui::status_bar::Message>,
    application_state: ApplicationState,
}

#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct ApplicationState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub simulation_state: crate::design::SimulationState,
    pub parameter_ptr: ParameterPtr,
    pub axis_aligned: bool,
    pub action_mode: ActionMode,
    pub selection_mode: SelectionMode,
}

impl IcedMessages {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            left_panel: VecDeque::new(),
            top_bar: VecDeque::new(),
            color_overlay: VecDeque::new(),
            status_bar: VecDeque::new(),
            application_state: Default::default(),
        }
    }

    pub fn push_scaffold_info(&mut self, info: Option<crate::design::ScaffoldInfo>) {
        self.left_panel
            .push_back(gui::left_panel::Message::NewScaffoldInfo(info));
    }

    pub fn push_color(&mut self, color: u32) {
        let bytes = color.to_be_bytes();
        // bytes is [A, R, G, B]
        let color = iced::Color::from_rgb8(bytes[1], bytes[2], bytes[3]);
        self.color_overlay
            .push_back(gui::left_panel::ColorMessage::StrandColorChanged(color));
        self.left_panel
            .push_back(gui::left_panel::Message::StrandColorChanged(color));
    }

    pub fn push_sequence(&mut self, sequence: String) {
        self.left_panel
            .push_back(gui::left_panel::Message::SequenceChanged(sequence));
    }

    pub fn push_op(&mut self, operation: Arc<dyn Operation>) {
        self.status_bar
            .push_back(gui::status_bar::Message::Operation(operation));
    }

    pub fn push_selection(&mut self, selection: mediator::Selection, values: Vec<String>) {
        self.left_panel
            .push_back(gui::left_panel::Message::Selection(
                selection,
                values.clone(),
            ))
    }

    pub fn push_candidate(&mut self, selection: mediator::Selection, values: Vec<String>) {
        self.status_bar
            .push_back(gui::status_bar::Message::Selection(
                selection,
                values.clone(),
            ));
    }

    pub fn push_organizer_selection(&mut self, selection: Vec<crate::design::DnaElementKey>) {
        self.left_panel
            .push_back(gui::left_panel::Message::NewSelection(selection))
    }

    pub fn clear_op(&mut self) {
        self.status_bar.push_back(gui::status_bar::Message::ClearOp);
    }

    pub fn push_action_mode(&mut self, action_mode: mediator::ActionMode) {
        self.left_panel
            .push_back(gui::left_panel::Message::ActionModeChanged(action_mode))
    }

    pub fn push_selection_mode(&mut self, selection_mode: mediator::SelectionMode) {
        self.left_panel
            .push_back(gui::left_panel::Message::SelectionModeChanged(
                selection_mode,
            ))
    }

    pub fn push_progress(&mut self, progress_name: String, progress: f32) {
        self.status_bar
            .push_back(gui::status_bar::Message::Progress(Some((
                progress_name,
                progress,
            ))))
    }

    pub fn finish_progess(&mut self) {
        self.status_bar
            .push_back(gui::status_bar::Message::Progress(None))
    }

    pub fn notify_new_design(&mut self) {
        self.left_panel
            .push_back(gui::left_panel::Message::NewDesign)
    }

    pub fn push_roll(&mut self, roll: f32) {
        self.left_panel
            .push_back(gui::left_panel::Message::HelixRoll(roll))
    }

    pub fn push_dna_elements(&mut self, elements: Vec<crate::design::DnaElement>) {
        self.left_panel
            .push_back(gui::left_panel::Message::NewDnaElement(elements))
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        self.left_panel
            .push_back(gui::left_panel::Message::ModifiersChanged(modifiers))
    }

    pub fn push_new_tree(
        &mut self,
        tree: ensnano_organizer::OrganizerTree<crate::design::DnaElementKey>,
    ) {
        self.left_panel
            .push_back(gui::left_panel::Message::NewTreeApp(tree))
    }

    pub fn new_ui_size(&mut self, ui_size: gui::UiSize) {
        self.left_panel
            .push_back(gui::left_panel::Message::UiSizeChanged(ui_size.clone()));
        self.top_bar
            .push_back(gui::top_bar::Message::UiSizeChanged(ui_size.clone()));
    }

    pub fn push_can_make_grid(&mut self, can_make_grid: bool) {
        self.left_panel
            .push_back(gui::left_panel::Message::CanMakeGrid(can_make_grid));
    }

    pub fn push_show_tutorial(&mut self) {
        self.left_panel
            .push_back(gui::left_panel::Message::ShowTutorial);
    }

    pub fn show_help(&mut self) {
        self.left_panel
            .push_back(gui::left_panel::Message::ForceHelp);
    }

    pub(crate) fn push_application_state(&mut self, state: ApplicationState) {
        let must_update = self.application_state != state;
        self.application_state = state.clone();
        if must_update {
            self.left_panel
                .push_back(gui::left_panel::Message::NewApplicationState(state.clone()));
            self.top_bar
                .push_back(gui::top_bar::Message::NewApplicationState(state))
        }
    }
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

use std::ops::{Deref, DerefMut};
fn download_stapples<R: DerefMut<Target = Requests>, M: Deref<Target = Mediator>>(
    mut requests: R,
    mediator: M,
) {
    use mediator::{DownloadStappleError, DownloadStappleOk};
    let result = mediator.download_stapples();
    match result {
        Ok(DownloadStappleOk {
            design_id,
            warnings,
        }) => {
            for warn in warnings {
                requests
                    .keep_proceed
                    .push_back(Action::Warning(warn.dialog()))
            }
            requests
                .keep_proceed
                .push_back(Action::AskStaplesPath { d_id: design_id })
        }
        Err(DownloadStappleError::NoScaffoldSet) => {
            message(
                "No scaffold set. \n
                    Chose a strand and set it as the scaffold by checking the scaffold checkbox\
                    in the status bar"
                    .into(),
                rfd::MessageLevel::Error,
            );
        }
        Err(DownloadStappleError::ScaffoldSequenceNotSet) => {
            message(
                "No sequence uploaded for scaffold. \n
                Upload a sequence for the scaffold by pressing the \"Load scaffold\" button"
                    .into(),
                rfd::MessageLevel::Error,
            );
        }
        Err(DownloadStappleError::SeveralDesignNoneSelected) => {
            message(
                "No design selected, select a design by selecting one of its elements".into(),
                rfd::MessageLevel::Error,
            );
        }
    }
}

/// The state of the main event loop.
pub(crate) struct MainState {
    app_state: AppState,
    pending_actions: VecDeque<Action>,
    undo_stack: Vec<AppState>,
    redo_stack: Vec<AppState>,
    chanel_reader: ChanelReader,
    scene: Arc<Mutex<Scene<AppState>>>,
    flatscene: Arc<Mutex<FlatScene<AppState>>>,
    messages: Arc<Mutex<IcedMessages>>,
}

struct MainStateConstructor {
    scene: Arc<Mutex<Scene<AppState>>>,
    flatscene: Arc<Mutex<FlatScene<AppState>>>,
    messages: Arc<Mutex<IcedMessages>>,
}

impl MainState {
    fn new(constructor: MainStateConstructor) -> Self {
        Self {
            app_state: Default::default(),
            pending_actions: VecDeque::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            chanel_reader: Default::default(),
            scene: constructor.scene,
            flatscene: constructor.flatscene,
            messages: constructor.messages,
        }
    }

    fn push_action(&mut self, action: Action) {
        self.pending_actions.push_back(action)
    }
}

/// A temporary view of the main state and the control flow.
struct MainStateView<'a> {
    main_state: &'a mut MainState,
    control_flow: &'a mut ControlFlow,
}

use controller::{LoadDesignError, MainState as MainStateInteface};
impl<'a> MainStateInteface for MainStateView<'a> {
    fn pop_action(&mut self) -> Option<Action> {
        self.main_state.pending_actions.pop_front()
    }

    fn exit_control_flow(&mut self) {
        *self.control_flow = ControlFlow::Exit
    }

    fn new_design(&mut self) {
        todo!()
    }

    fn load_design(&mut self, path: PathBuf) -> Result<(), LoadDesignError> {
        todo!()
    }

    fn get_chanel_reader(&mut self) -> &mut ChanelReader {
        &mut self.main_state.chanel_reader
    }
}
