//! The Mediator coordinates the interaction between the designs and the applications.
//! When a design is modified, it notifies the mediator of its changes and the mediator forwards
//! that information to the applications.
//!
//! When an application wants to modify a design, it makes the modification request to the
//! mediator.
//!
//! The mediator also holds data that is common to all applications.
use crate::{DrawArea, Duration, ElementType, IcedMessages, Multiplexer, WindowEvent};
use iced_wgpu::wgpu;
use iced_winit::winit::dpi::{PhysicalPosition, PhysicalSize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use native_dialog::{Dialog, MessageAlert};

use crate::design;

use design::{Design, DesignNotification, DesignRotation};

mod operation;
pub use operation::*;

pub type MediatorPtr = Arc<Mutex<Mediator>>;

pub struct Mediator {
    applications: HashMap<ElementType, Arc<Mutex<dyn Application>>>,
    designs: Vec<Arc<Mutex<Design>>>,
    selection: Selection,
    messages: Arc<Mutex<IcedMessages>>,
    current_operation: Option<Arc<dyn Operation>>,
}

pub struct Scheduler {
    applications: HashMap<ElementType, Arc<Mutex<dyn Application>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            applications: HashMap::new(),
        }
    }

    pub fn add_application(
        &mut self,
        application: Arc<Mutex<dyn Application>>,
        element_type: ElementType,
    ) {
        self.applications.insert(element_type, application);
    }

    pub fn forward_event(
        &mut self,
        event: &WindowEvent,
        area: ElementType,
        cursor_position: PhysicalPosition<f64>,
    ) {
        if let Some(app) = self.applications.get_mut(&area) {
            app.lock().unwrap().on_event(event, cursor_position)
        }
    }

    pub fn draw_apps(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        multiplexer: &Multiplexer,
        dt: Duration,
    ) {
        for (area, app) in self.applications.iter_mut() {
            if let Some(target) = multiplexer.get_texture_view(*area) {
                app.lock().unwrap().on_redraw_request(encoder, target, dt);
            }
        }
    }

    pub fn forward_new_size(&mut self, window_size: PhysicalSize<u32>, multiplexer: &Multiplexer) {
        for (area, app) in self.applications.iter_mut() {
            if let Some(draw_area) = multiplexer.get_draw_area(*area) {
                app.lock().unwrap().on_resize(window_size, draw_area);
            }
        }
    }
}

#[derive(Clone)]
/// A notification that must be send to the application
pub enum Notification {
    /// A design has been modified
    DesignNotification(DesignNotification),
    #[allow(dead_code)]
    AppNotification(AppNotification),
    /// A new design has been added
    NewDesign(Arc<Mutex<Design>>),
    /// The application must show/hide the sequences
    ToggleText(bool),
    /// The scroll sensitivity has been modified
    NewSensitivity(f32),
    /// The action mode has been modified
    NewActionMode(ActionMode),
    /// The selection mode has been modified
    NewSelectionMode(SelectionMode),
    FitRequest,
    /// The designs have been deleted
    ClearDesigns,
}

pub trait Application {
    /// For notification about the data
    fn on_notify(&mut self, notification: Notification);
    /// The method must be called when the window is resized or when the drawing area is modified
    fn on_resize(&mut self, window_size: PhysicalSize<u32>, area: DrawArea);
    /// The methods is used to forwards the window events to applications
    fn on_event(&mut self, event: &WindowEvent, position: PhysicalPosition<f64>);
    /// The method is used to forwards redraw_requests to applications
    fn on_redraw_request(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        dt: Duration,
    );
}

impl Mediator {
    pub fn new(messages: Arc<Mutex<IcedMessages>>) -> Self {
        Self {
            applications: HashMap::new(),
            designs: Vec::new(),
            selection: Selection::Nothing,
            messages,
            current_operation: None,
        }
    }

    pub fn add_application(
        &mut self,
        application: Arc<Mutex<dyn Application>>,
        element_type: ElementType,
    ) {
        self.applications.insert(element_type, application);
    }

    pub fn change_sensitivity(&mut self, sensitivity: f32) {
        self.notify_apps(Notification::NewSensitivity(sensitivity));
    }

    pub fn change_action_mode(&mut self, action_mode: ActionMode) {
        self.notify_apps(Notification::NewActionMode(action_mode))
    }

    pub fn change_selection_mode(&mut self, selection_mode: SelectionMode) {
        self.notify_apps(Notification::NewSelectionMode(selection_mode))
    }

    pub fn request_fits(&mut self) {
        self.notify_apps(Notification::FitRequest)
    }

    pub fn nb_design(&self) -> usize {
        self.designs.len()
    }

    pub fn add_design(&mut self, design: Arc<Mutex<Design>>) {
        self.designs.push(design.clone());
        self.notify_apps(Notification::NewDesign(design));
    }

    pub fn change_strand_color(&mut self, color: u32) {
        if let Selection::Strand(design_id, strand_id) = self.selection {
            self.designs[design_id as usize]
                .lock()
                .unwrap()
                .change_strand_color(strand_id as usize, color)
        }
    }

    pub fn change_sequence(&mut self, sequence: String) {
        if let Selection::Strand(design_id, strand_id) = self.selection {
            self.designs[design_id as usize]
                .lock()
                .unwrap()
                .change_strand_sequence(strand_id as usize, sequence)
        }
    }

    pub fn save_design(&mut self, path: &PathBuf) {
        if let Some(d_id) = self.selected_design() {
            self.designs[d_id as usize].lock().unwrap().save_to(path)
        } else {
            let error_msg = MessageAlert {
                title: "Error",
                text: "No design selected",
                typ: native_dialog::MessageType::Error,
            };
            std::thread::spawn(|| {
                error_msg.show().unwrap_or(());
            });
        }
    }

    pub fn clear_designs(&mut self) {
        self.designs = vec![];
        self.notify_apps(Notification::ClearDesigns)
    }

    pub fn notify_selection(&mut self, selection: Selection) {
        self.selection = selection;
        if selection.is_strand() {
            let mut messages = self.messages.lock().unwrap();
            if let Selection::Strand(d_id, s_id) = selection {
                let design = self.designs[d_id as usize].lock().unwrap();
                if let Some(color) = design.get_strand_color(s_id as usize) {
                    messages.push_color(color);
                }
                if let Some(sequence) = design.get_strand_sequence(s_id as usize) {
                    messages.push_sequence(sequence);
                }
            }
        }
    }

    pub fn toggle_text(&mut self, value: bool) {
        self.notify_apps(Notification::ToggleText(value));
    }

    pub fn notify_apps(&mut self, notification: Notification) {
        for app_wrapper in self.applications.values().cloned() {
            let mut app = app_wrapper.lock().unwrap();
            app.on_notify(notification.clone());
        }
    }

    pub fn notify_all_designs(&mut self, notification: AppNotification) {
        for design_wrapper in self.designs.clone() {
            design_wrapper
                .lock()
                .unwrap()
                .on_notify(notification.clone())
        }
    }

    pub fn notify_designs(&mut self, designs: &HashSet<u32>, notification: AppNotification) {
        for design_id in designs.iter() {
            self.designs.clone()[*design_id as usize]
                .lock()
                .unwrap()
                .on_notify(notification.clone());
            //design.on_notify(notification.clone(), self);
        }
    }

    pub fn make_grids(&mut self) {
        self.notify_all_designs(AppNotification::MakeGrids)
    }

    pub fn observe_designs(&mut self) {
        let mut notifications = Vec::new();
        for design_wrapper in self.designs.clone() {
            if let Some(notification) = design_wrapper.lock().unwrap().view_was_updated() {
                notifications.push(Notification::DesignNotification(notification))
            }
            if let Some(notification) = design_wrapper.lock().unwrap().data_was_updated() {
                notifications.push(Notification::DesignNotification(notification))
            }
        }
        for notification in notifications {
            self.notify_apps(notification)
        }
    }

    fn selected_design(&self) -> Option<u32> {
        self.selection.get_design()
    }

    pub fn update_opperation(&mut self, operation: Arc<dyn Operation>, from_app: bool) {
        let target = {
            let mut set = HashSet::new();
            set.insert(operation.target() as u32);
            set
        };
        let effect = operation.effect();
        if let Some(current_op) = self.current_operation.replace(operation.clone()) {
            let rev_op = current_op.reverse();
            let target = {
                let mut set = HashSet::new();
                set.insert(current_op.target() as u32);
                set
            };
            self.notify_designs(&target, rev_op.effect());
        }
        if from_app {
            self.messages.lock().unwrap().push_op(operation);
        }
        self.notify_designs(&target, effect)
    }
}

#[derive(Clone)]
pub enum AppNotification {
    MovementEnded,
    Rotation(DesignRotation),
    Translation(ultraviolet::Vec3),
    MakeGrids,
}

#[derive(Clone, Copy)]
pub enum Selection {
    Nucleotide(u32, u32),
    Design(u32),
    Strand(u32, u32),
    Helix(u32, u32),
    Nothing,
}

impl Selection {
    pub fn is_strand(&self) -> bool {
        match self {
            Selection::Strand(_, _) => true,
            _ => false,
        }
    }

    pub fn get_design(&self) -> Option<u32> {
        match self {
            Selection::Design(d) => Some(*d),
            Selection::Strand(d, _) => Some(*d),
            Selection::Helix(d, _) => Some(*d),
            Selection::Nucleotide(d, _) => Some(*d),
            Selection::Nothing => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Nucleotide,
    Design,
    Strand,
    Helix,
}

impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::Nucleotide
    }
}

impl std::fmt::Display for SelectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SelectionMode::Design => "Design",
                SelectionMode::Nucleotide => "Nucleotide",
                SelectionMode::Strand => "Strand",
                SelectionMode::Helix => "Helix",
            }
        )
    }
}

impl SelectionMode {
    pub const ALL: [SelectionMode; 4] = [
        SelectionMode::Nucleotide,
        SelectionMode::Design,
        SelectionMode::Strand,
        SelectionMode::Helix,
    ];
}

/// Describe the action currently done by the user when they click left
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionMode {
    /// User is moving the camera
    Normal,
    /// User can translate objects and move the camera
    Translate,
    /// User can rotate objects and move the camera
    Rotate,
    /// User can elongate/shorten strands
    Build,
    /// Use can cut strands
    Cut,
}

impl Default for ActionMode {
    fn default() -> Self {
        ActionMode::Normal
    }
}

impl std::fmt::Display for ActionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ActionMode::Normal => "Normal",
                ActionMode::Translate => "Translate",
                ActionMode::Rotate => "Rotate",
                ActionMode::Build => "Build",
                ActionMode::Cut => "Cut",
            }
        )
    }
}
