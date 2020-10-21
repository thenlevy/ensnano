//! The Mediator coordinates the interaction between the designs and the applications.
//! When a design is modified, it notifies the mediator of its changes and the mediator forwards
//! that information to the applications.
//!
//! When an application wants to modify a design, it makes the modification request to the
//! mediator.
//!
//! The mediator also holds data that is common to all applications.
use crate::Messages;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use native_dialog::{Dialog, MessageAlert};

use crate::design;

use design::{Design, DesignNotification, DesignRotation};

pub type MediatorPtr = Arc<Mutex<Mediator>>;

pub struct Mediator {
    applications: Vec<Arc<Mutex<dyn Application>>>,
    designs: Vec<Arc<Mutex<Design>>>,
    selection: Selection,
    new_strand: bool,
    messages: Arc<Mutex<Messages>>,
}

#[derive(Clone)]
/// A notification that must be send to the application
pub enum Notification<'a> {
    /// A design has been modified
    DesignNotification(DesignNotification),
    #[allow(dead_code)]
    AppNotification(AppNotification<'a>),
    /// A new design has been added
    NewDesign(Arc<Mutex<Design>>),
    /// The application must show/hide the sequences
    ToggleText(bool),
    /// The designs have been deleted
    ClearDesigns,
}

pub trait Application {
    fn on_notify(&mut self, notification: Notification);
}

impl Mediator {
    pub fn new(messages: Arc<Mutex<Messages>>) -> Self {
        Self {
            applications: Vec::new(),
            designs: Vec::new(),
            selection: Selection::Nothing,
            new_strand: false,
            messages,
        }
    }

    pub fn add_application(&mut self, application: Arc<Mutex<dyn Application>>) {
        self.applications.push(application)
    }

    pub fn nb_design(&self) -> usize {
        self.designs.len()
    }

    pub fn add_design(&mut self, design: Arc<Mutex<Design>>) {
        self.designs.push(design.clone());
        self.notify_apps(Notification::NewDesign(design));
    }

    pub fn change_strand_color(&mut self, color: u32) {
        match self.selection {
            Selection::Strand(design_id, strand_id) => self.designs[design_id as usize]
                .lock()
                .unwrap()
                .change_strand_color(strand_id as usize, color),
            _ => (),
        }
    }

    pub fn change_sequence(&mut self, sequence: String) {
        match self.selection {
            Selection::Strand(design_id, strand_id) => self.designs[design_id as usize]
                .lock()
                .unwrap()
                .change_strand_sequence(strand_id as usize, sequence),
            _ => (),
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
        for app_wrapper in self.applications.clone() {
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
}

#[derive(Clone)]
pub enum AppNotification<'a> {
    MovementEnded,
    Rotation(&'a DesignRotation),
    Translation(&'a ultraviolet::Vec3),
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
