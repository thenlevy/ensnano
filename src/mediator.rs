//! The Mediator coordinates the interaction between the designs and the applications.
//! When a design is modified, it notifies the mediator of its changes and the mediator forwards
//! that information to the applications.
//!
//! When an application wants to modify a design, it makes the modification request to the
//! mediator.
//!
//! The mediator also holds data that is common to all applications.
use crate::gui::RigidBodyParametersRequest;
use crate::gui::{HyperboloidRequest, KeepProceed, Requests, SimulationRequest};
use crate::utils::{message, yes_no_dialog, PhantomElement};
use crate::{DrawArea, Duration, ElementType, IcedMessages, Multiplexer, WindowEvent};
use iced_wgpu::wgpu;
use iced_winit::winit::dpi::{PhysicalPosition, PhysicalSize};
use simple_excel_writer::{row, Row, Workbook};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use ultraviolet::Vec3;

use crate::design;

use design::{
    Design, DesignNotification, DesignRotation, DesignTranslation, GridDescriptor,
    GridHelixDescriptor, Helix, Hyperboloid, Nucl, RigidBodyConstants, Stapple, Strand,
    StrandBuilder, StrandState,
};

mod operation;
mod selection;
pub use operation::*;
pub use selection::*;

pub type MediatorPtr = Arc<Mutex<Mediator>>;

pub struct Mediator {
    applications: HashMap<ElementType, Arc<Mutex<dyn Application>>>,
    designs: Vec<Arc<RwLock<Design>>>,
    selection: Vec<Selection>,
    candidate: Option<Option<PhantomElement>>,
    last_selection: Option<Vec<Selection>>,
    messages: Arc<Mutex<IcedMessages>>,
    /// The operation that is beign modified by the current drag and drop
    current_operation: Option<Arc<dyn Operation>>,
    /// The operation that can currently be eddited via the status bar or in the scene
    last_op: Option<Arc<dyn Operation>>,
    undo_stack: Vec<Arc<dyn Operation>>,
    redo_stack: Vec<Arc<dyn Operation>>,
    computing: Arc<Mutex<bool>>,
    centring: Option<(Nucl, usize)>,
    pasting: PastingMode,
    last_selected_design: usize,
    pasting_attempt: Option<Nucl>,
    duplication_attempt: bool,
}

/// The scheduler is responsible for running the different applications
pub struct Scheduler {
    applications: HashMap<ElementType, Arc<Mutex<dyn Application>>>,
    needs_redraw: Vec<ElementType>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            applications: HashMap::new(),
            needs_redraw: Vec::new(),
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

    pub fn check_redraw(&mut self, multiplexer: &Multiplexer, dt: Duration) -> bool {
        self.needs_redraw.clear();
        for (area, app) in self.applications.iter_mut() {
            if multiplexer.is_showing(area) && app.lock().unwrap().needs_redraw(dt) {
                self.needs_redraw.push(*area)
            }
        }
        self.needs_redraw.len() > 0
    }

    /// Request an application to draw on a texture
    pub fn draw_apps(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        multiplexer: &Multiplexer,
        dt: Duration,
    ) {
        for area in self.needs_redraw.iter() {
            let app = self.applications.get_mut(area).unwrap();
            if let Some(target) = multiplexer.get_texture_view(*area) {
                app.lock().unwrap().on_redraw_request(encoder, target, dt);
            }
        }
    }

    /// Notify all applications that the size of the window has been modified
    pub fn forward_new_size(&mut self, window_size: PhysicalSize<u32>, multiplexer: &Multiplexer) {
        if window_size.height > 0 && window_size.width > 0 {
            for (area, app) in self.applications.iter_mut() {
                if let Some(draw_area) = multiplexer.get_draw_area(*area) {
                    app.lock().unwrap().on_resize(window_size, draw_area);
                    self.needs_redraw.push(*area);
                }
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
    NewDesign(Arc<RwLock<Design>>),
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
    Selection3D(Vec<Selection>),
    /// A save request has been filled
    Save(usize),
    /// The 3d camera must face a given target
    CameraTarget((Vec3, Vec3)),
    CameraRotation(f32, f32),
    Centering(Nucl, usize),
    Pasting(bool),
    ShowTorsion(bool),
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
    fn needs_redraw(&mut self, dt: Duration) -> bool;
}

impl Mediator {
    pub fn new(messages: Arc<Mutex<IcedMessages>>, computing: Arc<Mutex<bool>>) -> Self {
        Self {
            applications: HashMap::new(),
            designs: Vec::new(),
            selection: vec![],
            messages,
            current_operation: None,
            last_op: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            candidate: None,
            last_selection: None,
            computing,
            centring: None,
            pasting: PastingMode::Nothing,
            last_selected_design: 0,
            pasting_attempt: None,
            duplication_attempt: false,
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

    pub fn add_design(&mut self, design: Arc<RwLock<Design>>) {
        self.designs.push(design.clone());
        self.notify_apps(Notification::NewDesign(design));
    }

    pub fn change_strand_color(&mut self, color: u32) {
        for s in self.selection.iter() {
            if let Selection::Strand(design_id, strand_id) = s {
                self.designs[*design_id as usize]
                    .write()
                    .unwrap()
                    .change_strand_color(*strand_id as usize, color)
            }
        }
    }

    pub fn change_sequence(&mut self, sequence: String) {
        for s in self.selection.iter() {
            if let Selection::Strand(design_id, strand_id) = s {
                self.designs[*design_id as usize]
                    .write()
                    .unwrap()
                    .change_strand_sequence(*strand_id as usize, sequence.clone())
            }
        }
    }

    pub fn set_scaffold(&mut self, scaffold_id: Option<usize>) {
        let d_id = if let Some(d_id) = self.selected_design() {
            d_id as usize
        } else {
            if self.designs.len() > 1 {
                message(
                    "No design selected, setting scaffold for design 0".into(),
                    rfd::MessageLevel::Warning,
                );
            }
            0
        };
        self.designs[d_id]
            .write()
            .unwrap()
            .set_scaffold_id(scaffold_id)
    }

    pub fn set_scaffold_sequence(&mut self, sequence: String, requests: Arc<Mutex<Requests>>) {
        let d_id = if let Some(d_id) = self.selected_design() {
            d_id as usize
        } else {
            if self.designs.len() > 1 {
                message(
                    "No design selected, setting sequence for design 0".into(),
                    rfd::MessageLevel::Warning,
                );
            }
            0
        };
        self.designs[d_id]
            .write()
            .unwrap()
            .set_scaffold_sequence(sequence);
        if self.designs[d_id].read().unwrap().scaffold_is_set() {
            let message = "Optimize the scaffold position ?\n
            If you chose \"Yes\", icednano will position the scaffold in a way that minimizes the number of anti-patern (G^4, C^4 (A|T)^7) in the stapples sequence. If you chose \"No\", the scaffold sequence will begin at position 0";
            yes_no_dialog(
                message.into(),
                requests.clone(),
                KeepProceed::OptimizeShift(d_id as usize),
                None,
            )
        }
    }

    pub fn optimize_shift(&mut self, d_id: usize) {
        let computing = self.computing.clone();
        let design = self.designs[d_id].clone();
        let messages = self.messages.clone();
        std::thread::spawn(move || {
            let (send, rcv) = std::sync::mpsc::channel::<f32>();
            std::thread::spawn(move || {
                *computing.lock().unwrap() = true;
                let score = design.read().unwrap().optimize_shift(send);
                let msg = format!("Number of anti-patern: {}", score);
                message(msg.into(), rfd::MessageLevel::Info);
                *computing.lock().unwrap() = false;
            });
            while let Ok(progress) = rcv.recv() {
                messages
                    .lock()
                    .unwrap()
                    .push_progress("Optimizing position".to_string(), progress)
            }
            messages.lock().unwrap().finish_progess();
        });
    }

    pub fn download_stapples(&self, requests: Arc<Mutex<Requests>>) {
        let d_id = if let Some(d_id) = self.selected_design() {
            d_id as usize
        } else {
            if self.designs.len() > 1 {
                message(
                    "No design selected, Downloading stapples design 0".into(),
                    rfd::MessageLevel::Warning,
                );
            }
            0
        };
        if !self.designs[d_id].read().unwrap().scaffold_is_set() {
            message(
                "No scaffold set. \n
                    Chose a strand and set it as the scaffold by checking the scaffold checkbox\
                    in the status bar"
                    .into(),
                rfd::MessageLevel::Error,
            );
            return;
        }
        if !self.designs[d_id].read().unwrap().scaffold_sequence_set() {
            message(
                "No sequence uploaded for scaffold. \n
                Upload a sequence for the scaffold by pressing the \"Load scaffold\" button"
                    .into(),
                rfd::MessageLevel::Error,
            );
            return;
        }
        if let Some(nucl) = self.designs[d_id].read().unwrap().get_stapple_mismatch() {
            let msg = format!(
                "All stapples are not paired \n
                first unpaired nucleotide {:?}",
                nucl
            );
            message(msg.into(), rfd::MessageLevel::Error);
            return;
        }

        let scaf_len = self.designs[d_id]
            .read()
            .unwrap()
            .get_scaffold_len()
            .unwrap();
        let scaf_seq_len = self.designs[d_id]
            .read()
            .unwrap()
            .get_scaffold_sequence_len()
            .unwrap();
        if scaf_len != scaf_seq_len {
            let msg = format!(
                "The scaffod length does not match its sequence\n
                Length of the scaffold {}\n
                Length of the sequence {}\n
                Proceed anyway ?",
                scaf_len, scaf_seq_len
            );

            yes_no_dialog(
                msg.into(),
                requests.clone(),
                KeepProceed::Stapples(d_id),
                None,
            );
        } else {
            requests.lock().unwrap().keep_proceed = Some(KeepProceed::Stapples(d_id))
        }
    }

    pub fn proceed_stapples(&mut self, design_id: usize, path: PathBuf) {
        let stapples = self.designs[design_id].read().unwrap().get_stapples();
        /*
        let path = if cfg!(target_os = "windows") {
            let (snd, rcv) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let xls_file = FileDialog::new()
                    .add_filter("Excel file", &["xlsx"])
                    .show_save_single_file();
                snd.send(xls_file.ok().and_then(|x| x)).unwrap()
            });
            rcv.recv().unwrap()
        } else {
            use nfd2::Response;
            let result = match nfd2::open_save_dialog(Some("xlsx"), None).expect("oh no") {
                Response::Okay(file_path) => Some(file_path),
                Response::OkayMultiple(_) => {
                    println!("Please open only one file");
                    None
                }
                Response::Cancel => None,
            };
            result
        };*/
        write_stapples(stapples, path);
    }

    pub fn set_persistent_phantom(&mut self, persistent: bool) {
        match self.selection.get(0) {
            Some(Selection::Grid(d_id, g_id)) => self.designs[*d_id as usize]
                .read()
                .unwrap()
                .set_persistent_phantom(&g_id, persistent),
            _ => println!("Selection is not a grid"),
        }
    }

    pub fn set_small_spheres(&mut self, small: bool) {
        match self.selection.get(0) {
            Some(Selection::Grid(d_id, g_id)) => self.designs[*d_id as usize]
                .read()
                .unwrap()
                .set_small_spheres(&g_id, small),
            _ => println!("Selection is not a grid"),
        }
    }

    pub fn save_design(&mut self, path: &PathBuf) {
        if let Some(d_id) = self.selected_design() {
            self.notify_apps(Notification::Save(d_id as usize));
            self.designs[d_id as usize].read().unwrap().save_to(path)
        } else {
            self.notify_apps(Notification::Save(0));
            self.designs[0].read().unwrap().save_to(path);
            if self.designs.len() > 1 {
                message(
                    "No design selected, saved design 0".into(),
                    rfd::MessageLevel::Warning,
                );
            }
        }
    }

    pub fn clear_designs(&mut self) {
        for d in self.designs.iter() {
            d.read().unwrap().notify_death()
        }
        self.designs = vec![];
        self.notify_apps(Notification::ClearDesigns)
    }

    pub fn notify_multiple_selection(&mut self, selection: Vec<Selection>) {
        self.selection = selection.clone();
        self.last_selection = Some(selection);
        self.pasting = PastingMode::Nothing;
        self.notify_all_designs(AppNotification::ResetCopyPaste);
    }

    pub fn notify_unique_selection(&mut self, selection: Selection) {
        self.pasting = PastingMode::Nothing;
        self.notify_all_designs(AppNotification::ResetCopyPaste);
        self.selection = vec![selection];
        self.last_selection = Some(vec![selection]);
        if selection.is_strand() {
            let mut messages = self.messages.lock().unwrap();
            if let Selection::Strand(d_id, s_id) = selection {
                let design = self.designs[d_id as usize].read().unwrap();
                if let Some(color) = design.get_strand_color(s_id as usize) {
                    messages.push_color(color);
                }
                if let Some(sequence) = design.get_strand_sequence(s_id as usize) {
                    messages.push_sequence(sequence);
                }
            }
        }
        if let Selection::Helix(d_id, h_id) = selection {
            let roll = self.designs[d_id as usize]
                .read()
                .unwrap()
                .get_roll_helix(h_id as usize);
            if let Some(roll) = roll {
                self.messages.lock().unwrap().push_roll(roll)
            }
        } else if let Selection::Nucleotide(d_id, nucl) = selection {
            self.designs[d_id as usize]
                .write()
                .unwrap()
                .shake_nucl(nucl)
        }
        if let Some(d_id) = selection.get_design() {
            let values = selection.fetch_values(self.designs[d_id as usize].clone());
            self.last_selected_design = d_id as usize;
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

    /// Show/Hide the DNA sequences
    pub fn toggle_text(&mut self, value: bool) {
        self.notify_apps(Notification::ToggleText(value));
    }

    pub fn notify_apps(&mut self, notification: Notification) {
        for app_wrapper in self.applications.values().cloned() {
            let mut app = app_wrapper.lock().unwrap();
            app.on_notify(notification.clone());
        }
    }

    fn notify_all_designs(&mut self, notification: AppNotification) {
        for design_wrapper in self.designs.clone() {
            design_wrapper
                .write()
                .unwrap()
                .on_notify(notification.clone())
        }
    }

    fn notify_designs(&mut self, designs: &HashSet<u32>, notification: AppNotification) {
        for design_id in designs.iter() {
            self.designs.clone()[*design_id as usize]
                .write()
                .unwrap()
                .on_notify(notification.clone());
            //design.on_notify(notification.clone(), self);
        }
    }

    pub fn make_grids(&mut self) {
        self.notify_all_designs(AppNotification::MakeGrids)
    }

    /// Querry designs for modifcations that must be notified to the applications
    pub fn observe_designs(&mut self) -> bool {
        let mut ret = false;
        let mut notifications = Vec::new();
        for design_wrapper in self.designs.clone() {
            if let Some(notification) = design_wrapper.read().unwrap().view_was_updated() {
                ret = true;
                notifications.push(Notification::DesignNotification(notification))
            }
            if let Some(notification) = design_wrapper.read().unwrap().data_was_updated() {
                ret = true;
                notifications.push(Notification::DesignNotification(notification))
            }
        }
        for notification in notifications {
            self.notify_apps(notification)
        }
        if let Some(candidate) = self.candidate.take() {
            ret = true;
            if let Some(pe) = candidate {
                let design_id = pe.design_id as usize;
                let nucl = Nucl {
                    helix: pe.helix_id as usize,
                    position: pe.position as isize,
                    forward: pe.forward,
                };
                let strand_opt = self.designs[design_id]
                    .read()
                    .unwrap()
                    .get_strand_nucl(&nucl);
                if let Some(strand) = strand_opt {
                    let selection = Selection::Strand(design_id as u32, strand as u32);
                    let values = selection.fetch_values(self.designs[design_id].clone());
                    self.messages
                        .lock()
                        .unwrap()
                        .push_selection(selection, values);
                }
            }
            self.notify_apps(Notification::NewCandidate(candidate))
        }
        if let Some(nucl) = self.pasting_attempt.take() {
            match self.pasting {
                PastingMode::Pasting | PastingMode::FirstDulplication => {
                    let result = self.designs[self.last_selected_design]
                        .write()
                        .unwrap()
                        .paste(nucl);

                    if let Some((initial_state, final_state)) = result {
                        self.finish_op();
                        self.undo_stack.push(Arc::new(BigStrandModification {
                            initial_state,
                            final_state,
                            reverse: false,
                            design_id: self.last_selected_design,
                        }));
                        self.pasting.place_paste();
                        self.notify_apps(Notification::Pasting(self.pasting.is_placing_paste()));
                    }
                }
                _ => {
                    let result = self.designs[self.last_selected_design]
                        .write()
                        .unwrap()
                        .paste_xover(nucl);
                    if let Some((initial_state, final_state)) = result {
                        self.finish_op();
                        self.undo_stack.push(Arc::new(BigStrandModification {
                            initial_state,
                            final_state,
                            reverse: false,
                            design_id: self.last_selected_design,
                        }));
                        self.pasting.place_paste();
                        self.notify_apps(Notification::Pasting(self.pasting.is_placing_paste()));
                    }
                }
            }
        }
        if self.duplication_attempt {
            if self.pasting.strand() {
                let paste_result = self.designs[self.last_selected_design]
                    .write()
                    .unwrap()
                    .apply_duplication();
                if let Some((initial_state, final_state)) = paste_result {
                    self.finish_op();
                    self.undo_stack.push(Arc::new(BigStrandModification {
                        initial_state,
                        final_state,
                        reverse: false,
                        design_id: self.last_selected_design,
                    }));
                } else {
                    self.pasting = PastingMode::FirstDulplication;
                }
            } else if self.pasting.xover() {
                let result = self.designs[self.last_selected_design]
                    .write()
                    .unwrap()
                    .apply_duplication_xover();
                if let Some((initial_state, final_state)) = result {
                    self.finish_op();
                    self.undo_stack.push(Arc::new(BigStrandModification {
                        initial_state,
                        final_state,
                        reverse: false,
                        design_id: self.last_selected_design,
                    }));
                } else {
                    self.pasting = PastingMode::FirstDulplicationXover;
                }
            }
            self.notify_apps(Notification::Pasting(self.pasting.is_placing_paste()));
            self.duplication_attempt = false;
        }
        if let Some(selection) = self.last_selection.take() {
            ret = true;
            self.notify_apps(Notification::Selection3D(selection))
        }

        if let Some(centring) = self.centring.take() {
            ret = true;
            self.notify_apps(Notification::NewSelectionMode(SelectionMode::Nucleotide));
            self.notify_apps(Notification::Centering(centring.0, centring.1))
        }
        ret
    }

    fn selected_design(&self) -> Option<u32> {
        self.selection.get(0).and_then(Selection::get_design)
    }

    /// Update the current operation.
    ///
    /// This method is called when an operation is performed in the scene. If the operation is
    /// compatible with the last operation it is treated as an eddition of the last operation.
    /// Otherwise the last operation is considered finished.
    pub fn update_opperation(&mut self, operation: Arc<dyn Operation>) {
        // If the operation is compatible with the last operation, the last operation is eddited.
        if *self.computing.lock().unwrap() {
            return;
        }
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
            if current_op.descr() == operation.descr() && current_op.must_reverse() {
                let rev_op = current_op.reverse();
                let target = {
                    let mut set = HashSet::new();
                    set.insert(current_op.target() as u32);
                    set
                };
                self.notify_designs(&target, rev_op.effect());
            } else if current_op.descr() != operation.descr() {
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
        if *self.computing.lock().unwrap() {
            return;
        }
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
        if *self.computing.lock().unwrap() {
            return;
        }
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
        /*
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
        } else         */
        self.suspend_op();
        self.finish_pending();
        if let Some(op) = self.undo_stack.pop() {
            let rev_op = op.reverse();
            let target = {
                let mut set = HashSet::new();
                set.insert(rev_op.target() as u32);
                set
            };
            println!("effet {:?}", rev_op.effect());
            self.notify_designs(&target, rev_op.effect());
            self.notify_all_designs(AppNotification::MovementEnded);
            self.redo_stack.push(rev_op);
        }
    }

    pub fn redo(&mut self) {
        if let Some(op) = self.redo_stack.pop() {
            let rev_op = op.reverse();
            println!("{:?}", rev_op);
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
        let nucl = candidate.map(|c| c.to_nucl());
        if self.pasting.is_placing_paste() {
            if self.pasting.strand() {
                self.designs[self.last_selected_design]
                    .write()
                    .unwrap()
                    .request_paste_candidate(nucl)
            } else if self.pasting.xover() {
                self.designs[self.last_selected_design]
                    .write()
                    .unwrap()
                    .request_paste_candidate_xover(nucl);
            }
        }
        self.candidate = Some(candidate)
    }

    pub fn set_paste_candidate(&mut self, candidate: Option<Nucl>) {
        if self.pasting.is_placing_paste() {
            if self.pasting.strand() {
                self.designs[self.last_selected_design]
                    .write()
                    .unwrap()
                    .request_paste_candidate(candidate)
            } else if self.pasting.xover() {
                self.designs[self.last_selected_design]
                    .write()
                    .unwrap()
                    .request_paste_candidate_xover(candidate);
            }
        }
    }

    pub fn request_centering(&mut self, nucl: Nucl, design_id: usize) {
        self.centring = Some((nucl, design_id))
    }

    pub fn request_camera_rotation(&mut self, rotation: (f32, f32)) {
        self.notify_apps(Notification::CameraRotation(rotation.0, rotation.1))
    }

    pub fn set_camera_target(&mut self, target: (Vec3, Vec3)) {
        self.notify_apps(Notification::CameraTarget(target))
    }

    pub fn recolor_stapples(&mut self) {
        for d in self.designs.iter() {
            d.write().unwrap().recolor_stapples();
        }
    }

    pub fn clean_designs(&mut self) {
        if !*self.computing.lock().unwrap() {
            for d in self.designs.iter() {
                d.write().unwrap().clean_up_domains()
            }
        }
    }

    pub fn roll_request(&mut self, request: SimulationRequest) {
        for d in self.designs.iter() {
            d.write()
                .unwrap()
                .roll_request(request.clone(), self.computing.clone());
        }
    }

    pub fn rigid_grid_request(&mut self, request: RigidBodyParametersRequest) {
        let parameters = rigid_parameters(request);
        for d in self.designs.iter() {
            d.write().unwrap().grid_simulation(
                (0., 1.),
                self.computing.clone(),
                parameters.clone(),
            );
        }
    }

    pub fn rigid_helices_request(&mut self, request: RigidBodyParametersRequest) {
        let parameters = rigid_parameters(request);
        for d in self.designs.iter() {
            d.write().unwrap().rigid_helices_simulation(
                (0., 0.1),
                self.computing.clone(),
                parameters.clone(),
            );
        }
        println!("self.computing {:?}", self.computing);
    }

    pub fn rigid_parameters_request(&mut self, request: RigidBodyParametersRequest) {
        let parameters = rigid_parameters(request);
        for d in self.designs.iter() {
            d.write()
                .unwrap()
                .rigid_body_parameters_update(parameters.clone());
        }
    }

    pub fn hyperboloid_update(&mut self, request: HyperboloidRequest) {
        if let Some(design) = self.designs.get(0) {
            design.write().unwrap().update_hyperboloid(
                request.radius,
                request.shift,
                request.length,
                request.radius_shift,
            );
        }
    }

    pub fn finalize_hyperboloid(&mut self) {
        if let Some(design) = self.designs.get(0) {
            design.write().unwrap().finalize_hyperboloid()
        }
    }

    pub fn roll_helix(&mut self, roll: f32) {
        for h in self.selection.iter() {
            if let Selection::Helix(d_id, h_id) = h {
                self.designs[*d_id as usize]
                    .write()
                    .unwrap()
                    .roll_helix(*h_id as usize, roll);
            }
        }
    }

    /// Request a cross-over between source and nucl.
    /// The design chose to accept the request depending on the rules defined in
    /// `design::operation::general_cross_over`
    pub fn xover_request(&mut self, source: Nucl, target: Nucl, design_id: usize) {
        let states = self.designs[design_id]
            .read()
            .unwrap()
            .general_cross_over(source, target);

        if let Some((initial_state, final_state)) = states {
            self.finish_op();
            self.undo_stack.push(Arc::new(BigStrandModification {
                initial_state,
                final_state,
                reverse: false,
                design_id: self.last_selected_design,
            }));
        }
    }

    pub fn show_torsion_request(&mut self, show: bool) {
        self.notify_apps(Notification::ShowTorsion(show))
    }

    pub fn request_copy(&mut self) {
        self.pasting = PastingMode::Nothing;
        self.notify_all_designs(AppNotification::ResetCopyPaste);
        println!("selection : {:?}", self.selection);
        if let Some((d_id, s_ids)) = list_of_strands(&self.selection, self.designs.clone()) {
            self.designs[d_id as usize]
                .write()
                .unwrap()
                .request_copy_strands(s_ids);
        } else if let Some((d_id, bounds)) = list_of_xovers(&self.selection) {
            let copy = self.designs[d_id as usize]
                .write()
                .unwrap()
                .request_copy_xovers(bounds);
            println!("copy success: {}", copy);
        }
    }

    pub fn request_pasting_mode(&mut self) {
        if self.designs[self.last_selected_design]
            .read()
            .unwrap()
            .has_template()
        {
            self.pasting = PastingMode::Pasting;
        } else if self.designs[self.last_selected_design]
            .read()
            .unwrap()
            .has_xovers_copy()
        {
            self.pasting = PastingMode::PastingXover
        }
        println!("{:?}", self.pasting);
        if self.pasting.is_placing_paste() {
            self.change_selection_mode(SelectionMode::Nucleotide);
        }
        self.notify_apps(Notification::Pasting(self.pasting.is_placing_paste()));
    }

    pub fn request_duplication(&mut self) {
        match self.pasting {
            PastingMode::Nothing => {
                if self.designs[self.last_selected_design]
                    .read()
                    .unwrap()
                    .has_template()
                {
                    self.pasting = PastingMode::FirstDulplication;
                } else if self.designs[self.last_selected_design]
                    .read()
                    .unwrap()
                    .has_xovers_copy()
                {
                    self.pasting = PastingMode::FirstDulplicationXover;
                }
            }
            PastingMode::Pasting => {
                self.pasting = PastingMode::FirstDulplication;
            }
            PastingMode::Duplicating => {
                self.duplication_attempt = true;
            }
            PastingMode::PastingXover => {
                self.pasting = PastingMode::FirstDulplicationXover;
            }
            PastingMode::DuplicatingXover => {
                self.duplication_attempt = true;
            }
            PastingMode::FirstDulplicationXover => (),
            PastingMode::FirstDulplication => (),
        }
        if self.pasting.is_placing_paste() {
            self.change_selection_mode(SelectionMode::Nucleotide);
        }
        self.notify_apps(Notification::Pasting(self.pasting.is_placing_paste()));
    }

    pub fn attempt_paste(&mut self, nucl: Nucl) {
        println!("Attempt paste {:?}", nucl);
        if self.pasting.is_placing_paste() {
            self.pasting_attempt = Some(nucl);
        }
    }

    pub fn request_anchor(&mut self) {
        let selection = self.selection.get(0).cloned();
        if let Some(Selection::Nucleotide(d_id, nucl)) = selection {
            self.designs[d_id as usize]
                .write()
                .unwrap()
                .add_anchor(nucl);
            self.notify_unique_selection(selection.unwrap());
        }
    }

    pub fn new_shift_hyperboloid(&mut self, shift: f32) {
        if let Some(Selection::Grid(d_id, g_id)) = self.selection.get(0) {
            self.designs[*d_id as usize]
                .write()
                .unwrap()
                .set_new_shift(*g_id, shift)
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppNotification {
    MovementEnded,
    Rotation(DesignRotation),
    Translation(DesignTranslation),
    AddGridHelix(GridHelixDescriptor, isize, usize),
    RmGridHelix(GridHelixDescriptor, isize, usize),
    RawHelixCreation {
        helix: Helix,
        delete: bool,
        h_id: usize,
    },
    Cut {
        strand: Strand,
        nucl: Nucl,
        undo: bool,
        s_id: usize,
    },
    Xover {
        strand_5prime: Strand,
        strand_3prime: Strand,
        undo: bool,
        prime5_id: usize,
        prime3_id: usize,
    },
    CrossCut {
        source_strand: Strand,
        target_strand: Strand,
        target_3prime: bool,
        source_id: usize,
        target_id: usize,
        nucl: Nucl,
        undo: bool,
    },
    RmStrand {
        strand: Strand,
        strand_id: usize,
        undo: bool,
    },
    MakeGrids,
    AddGrid(GridDescriptor),
    MoveBuilder(Box<StrandBuilder>, Option<(usize, u32)>),
    ResetBuilder(Box<StrandBuilder>),
    RmGrid,
    NewHyperboloid {
        position: Vec3,
        orientation: ultraviolet::Rotor3,
        hyperboloid: Hyperboloid,
    },
    ClearHyperboloid,
    NewStrandState(StrandState),
    ResetCopyPaste,
}

fn write_stapples(stapples: Vec<Stapple>, path: PathBuf) {
    use std::collections::BTreeMap;
    let mut wb = Workbook::create(path.to_str().unwrap());
    let mut sheets = BTreeMap::new();

    for stapple in stapples.iter() {
        let sheet = sheets
            .entry(stapple.plate)
            .or_insert_with(|| vec![vec!["Well Position", "Name", "Sequence"]]);
        sheet.push(vec![&stapple.well, &stapple.name, &stapple.sequence]);
    }

    for (sheet_id, rows) in sheets.iter() {
        let mut sheet = wb.create_sheet(&format!("Plate {}", sheet_id));
        wb.write_sheet(&mut sheet, |sw| {
            for row in rows {
                sw.append_row(row![row[0], row[1], row[2]])?;
            }
            Ok(())
        })
        .expect("write excel error!");
    }
    wb.close().expect("close excel error!");
}

#[derive(Debug)]
enum PastingMode {
    /// No pasting beeing made
    Nothing,
    /// First duplication, being positioned by the mouse
    FirstDulplication,
    /// Repeating last duplication
    Duplicating,
    /// One time duplication
    Pasting,
    PastingXover,
    FirstDulplicationXover,
    DuplicatingXover,
}

impl PastingMode {
    fn is_placing_paste(&self) -> bool {
        match self {
            Self::FirstDulplication
            | Self::Pasting
            | Self::FirstDulplicationXover
            | Self::PastingXover => true,
            Self::Nothing | Self::Duplicating | Self::DuplicatingXover => false,
        }
    }

    fn place_paste(&mut self) {
        match self {
            Self::FirstDulplication => *self = Self::Duplicating,
            Self::Pasting => *self = Self::Nothing,
            Self::FirstDulplicationXover => *self = Self::DuplicatingXover,
            Self::PastingXover => *self = Self::Nothing,
            _ => unreachable!(),
        }
    }

    fn xover(&self) -> bool {
        match self {
            Self::FirstDulplicationXover | Self::DuplicatingXover | Self::PastingXover => true,
            _ => false,
        }
    }

    fn strand(&self) -> bool {
        match self {
            Self::Duplicating | Self::Pasting | Self::FirstDulplication => true,
            _ => false,
        }
    }
}

fn rigid_parameters(parameters: RigidBodyParametersRequest) -> RigidBodyConstants {
    let ret = RigidBodyConstants {
        k_spring: 10f32.powf(parameters.k_springs),
        k_friction: 10f32.powf(parameters.k_friction),
        mass: 10f32.powf(parameters.mass_factor),
        volume_exclusion: parameters.volume_exclusion,
    };
    println!("{:?}", ret);
    ret
}
