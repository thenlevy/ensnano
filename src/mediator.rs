//! The Mediator coordinates the interaction between the designs and the applications.
//! When a design is modified, it notifies the mediator of its changes and the mediator forwards
//! that information to the applications.
//!
//! When an application wants to modify a design, it makes the modification request to the
//! mediator.
//!
//! The mediator also holds data that is common to all applications.
use crate::utils::PhantomElement;
use crate::{DrawArea, Duration, ElementType, IcedMessages, Multiplexer, WindowEvent};
use iced_wgpu::wgpu;
use iced_winit::winit::dpi::{PhysicalPosition, PhysicalSize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use native_dialog::{Dialog, MessageAlert};

use crate::design;

use design::{
    Design, DesignNotification, DesignRotation, DesignTranslation, GridDescriptor,
    GridHelixDescriptor,
};

mod operation;
mod selection;
pub use operation::*;
pub use selection::*;

pub type MediatorPtr = Arc<Mutex<Mediator>>;

pub struct Mediator {
    applications: HashMap<ElementType, Arc<Mutex<dyn Application>>>,
    designs: Vec<Arc<Mutex<Design>>>,
    selection: Selection,
    candidate: Option<Option<PhantomElement>>,
    messages: Arc<Mutex<IcedMessages>>,
    /// The operation that is beign modified by the current drag and drop
    current_operation: Option<Arc<dyn Operation>>,
    /// The operation that can currently be eddited via the status bar or in the scene
    last_op: Option<Arc<dyn Operation>>,
    undo_stack: Vec<Arc<dyn Operation>>,
    redo_stack: Vec<Arc<dyn Operation>>,
}

/// The scheduler is responsible for running the different applications
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

    /// Forwards an event to the appropriate application
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

    /// Request an application to draw on a texture
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

    /// Notify all applications that the size of the window has been modified
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
    /// A new element of the design must be highlighted
    NewCandidate(Option<PhantomElement>),
    /// An element has been selected in the 3d view
    Selection3D(Selection),
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
            last_op: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            candidate: None,
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
        self.messages.lock().unwrap().push_action_mode(action_mode);
        self.notify_apps(Notification::NewActionMode(action_mode))
    }

    pub fn change_selection_mode(&mut self, selection_mode: SelectionMode) {
        self.messages
            .lock()
            .unwrap()
            .push_selection_mode(selection_mode);
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

    pub fn set_persistent_phantom(&mut self, persistent: bool) {
        match self.selection {
            Selection::Grid(d_id, g_id) => self.designs[d_id as usize]
                .lock()
                .unwrap()
                .set_persistent_phantom(&g_id, persistent),
            _ => panic!("Selection is not a grid"),
        }
    }

    pub fn save_design(&mut self, path: &PathBuf) {
        if let Some(d_id) = self.selected_design() {
            self.designs[d_id as usize].lock().unwrap().save_to(path)
        } else {
            self.designs[0].lock().unwrap().save_to(path);
            if self.designs.len() > 1 {
                let error_msg = MessageAlert {
                    title: "Warning",
                    text: "No design selected, saved design 0",
                    typ: native_dialog::MessageType::Error,
                };
                std::thread::spawn(|| {
                    error_msg.show().unwrap_or(());
                });
            }
        }
    }

    pub fn clear_designs(&mut self) {
        self.designs = vec![];
        self.notify_apps(Notification::ClearDesigns)
    }

    pub fn notify_selection(&mut self, selection: Selection) {
        self.selection = selection;
        println!("selection {:?}", selection);
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
        if let Some(d_id) = selection.get_design() {
            let values = selection.fetch_values(self.designs[d_id as usize].clone());
            self.messages
                .lock()
                .unwrap()
                .push_selection(selection, values);
        } else {
            self.messages
                .lock()
                .unwrap()
                .push_selection(Selection::Nothing, vec![])
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

    /// Querry designs for modifcations that must be notified to the applications
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
        if let Some(candidate) = self.candidate.take() {
            self.notify_apps(Notification::NewCandidate(candidate))
        }
        self.notify_apps(Notification::Selection3D(self.selection))
    }

    fn selected_design(&self) -> Option<u32> {
        self.selection.get_design()
    }

    /// Update the current operation.
    ///
    /// This method is called when an operation is performed in the scene. If the operation is
    /// compatible with the last operation it is treated as an eddition of the last operation.
    /// Otherwise the last operation is considered finished.
    pub fn update_opperation(&mut self, operation: Arc<dyn Operation>) {
        // If the operation is compatible with the last operation, the last operation is eddited.
        let operation = if let Some(op) = self
            .last_op
            .as_ref()
            .and_then(|op| operation.compose(op.as_ref()))
        {
            op
        } else {
            // Otherwise, the last operation is saved on the undo stack.
            self.finish_pending();
            operation
        };
        let target = {
            let mut set = HashSet::new();
            set.insert(operation.target() as u32);
            set
        };
        let effect = operation.effect();
        if let Some(current_op) = self.current_operation.as_ref() {
            // If there already is a current operation. We test if the current operation is being
            // eddited.
            if current_op.descr() == operation.descr() {
                let rev_op = current_op.reverse();
                let target = {
                    let mut set = HashSet::new();
                    set.insert(current_op.target() as u32);
                    set
                };
                self.notify_designs(&target, rev_op.effect());
            } else {
                self.finish_op();
            }
        }
        self.messages.lock().unwrap().push_op(operation.clone());
        self.current_operation = Some(operation);
        self.notify_designs(&target, effect)
    }

    /// Update the pending operation.
    ///
    /// This method is called when a parameter of the pending operation is modified in the status
    /// bar.
    pub fn update_pending(&mut self, operation: Arc<dyn Operation>) {
        let target = {
            let mut set = HashSet::new();
            set.insert(operation.target() as u32);
            set
        };
        let effect = operation.effect();
        if let Some(current_op) = self.last_op.as_ref() {
            if current_op.descr() == operation.descr() {
                let rev_op = current_op.reverse();
                let target = {
                    let mut set = HashSet::new();
                    set.insert(current_op.target() as u32);
                    set
                };
                self.notify_designs(&target, rev_op.effect());
            } else {
                self.finish_op();
            }
        }
        self.last_op = Some(operation.clone());
        self.notify_designs(&target, effect)
    }

    /// Save the last operation and the pending operation on the undo stack.
    pub fn finish_op(&mut self) {
        if let Some(op) = self.last_op.take() {
            self.messages.lock().unwrap().clear_op();
            self.notify_all_designs(AppNotification::MovementEnded);
            self.undo_stack.push(op);
            self.redo_stack.clear();
        }
        if let Some(op) = self.current_operation.take() {
            self.messages.lock().unwrap().clear_op();
            self.notify_all_designs(AppNotification::MovementEnded);
            self.undo_stack.push(op);
            self.redo_stack.clear();
        }
    }

    /// Save the pending operation on the undo stack.
    fn finish_pending(&mut self) {
        if let Some(op) = self.last_op.take() {
            self.notify_all_designs(AppNotification::MovementEnded);
            self.undo_stack.push(op);
            self.redo_stack.clear();
        }
    }

    /// Suspend the current operation.
    ///
    /// This means that the current drag and drop movement is finished, but the current operation
    /// can still be modified in the satus bar or by initiating a combatible new operation.
    pub fn suspend_op(&mut self) {
        if let Some(op) = self.current_operation.take() {
            self.last_op = Some(op)
        }
    }

    pub fn undo(&mut self) {
        if let Some(op) = self.last_op.take() {
            let rev_op = op.reverse();
            let target = {
                let mut set = HashSet::new();
                set.insert(rev_op.target() as u32);
                set
            };
            self.notify_designs(&target, rev_op.effect());
            self.notify_all_designs(AppNotification::MovementEnded);
            self.redo_stack.push(rev_op);
        } else if let Some(op) = self.current_operation.take() {
            let rev_op = op.reverse();
            let target = {
                let mut set = HashSet::new();
                set.insert(rev_op.target() as u32);
                set
            };
            self.notify_designs(&target, rev_op.effect());
            self.notify_all_designs(AppNotification::MovementEnded);
            self.redo_stack.push(rev_op);
        } else if let Some(op) = self.undo_stack.pop() {
            let rev_op = op.reverse();
            let target = {
                let mut set = HashSet::new();
                set.insert(rev_op.target() as u32);
                set
            };
            self.notify_designs(&target, rev_op.effect());
            self.notify_all_designs(AppNotification::MovementEnded);
            self.redo_stack.push(rev_op);
        }
    }

    pub fn redo(&mut self) {
        if let Some(op) = self.redo_stack.pop() {
            let rev_op = op.reverse();
            let target = {
                let mut set = HashSet::new();
                set.insert(rev_op.target() as u32);
                set
            };
            self.notify_designs(&target, rev_op.effect());
            self.notify_all_designs(AppNotification::MovementEnded);
            self.undo_stack.push(rev_op);
        }
    }

    pub fn set_candidate(&mut self, candidate: Option<PhantomElement>) {
        self.candidate = Some(candidate)
    }
}

#[derive(Debug, Clone)]
pub enum AppNotification {
    MovementEnded,
    Rotation(DesignRotation),
    Translation(DesignTranslation),
    AddGridHelix(GridHelixDescriptor),
    RmGridHelix(GridHelixDescriptor),
    MakeGrids,
    AddGrid(GridDescriptor),
    RmGrid,
}
