use std::collections::HashSet;
use std::path::PathBuf;
/// The Mediator coordinates the interaction between the designs and the applications.
/// When a design is modified, it notifies the mediator of its changes and the mediator forwards
/// that information to the applications.
///
/// When an application wants to modify a design, it makes the modification request to the
/// mediator.
///
/// The mediator also holds data that is common to all applications.
use std::sync::{Arc, Mutex};

use native_dialog::{Dialog, MessageAlert};

use crate::design;

use design::{Design, DesignNotification, DesignRotation, DesignTranslation};

pub type MediatorPtr = Arc<Mutex<Mediator>>;

pub struct Mediator {
    applications: Vec<Arc<Mutex<dyn Application>>>,
    designs: Vec<Arc<Mutex<Design>>>,
    selected_design: Option<usize>,
    selected_strand: Option<usize>,
    new_strand: bool,
}

#[derive(Clone)]
pub enum Notification<'a> {
    DesignNotification(DesignNotification),
    #[allow(dead_code)]
    AppNotification(AppNotification<'a>),
    NewDesign(Arc<Mutex<Design>>),
    ClearDesigns,
}

pub trait Application {
    fn on_notify(&mut self, notification: Notification);
}

impl Mediator {
    pub fn new() -> Self {
        Self {
            applications: Vec::new(),
            designs: Vec::new(),
            selected_design: None,
            selected_strand: None,
            new_strand: false,
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
        if let Some(design_id) = self.selected_design {
            if let Some(strand_id) = self.selected_strand {
                self.designs[design_id]
                    .lock()
                    .unwrap()
                    .change_strand_color(strand_id, color);
            }
        }
    }

    pub fn get_strand_color(&mut self) -> Option<u32> {
        if !self.new_strand {
            return None;
        }
        self.new_strand = false;
        let d_id = self.selected_design?;
        let s_id = self.selected_strand?;
        self.designs[d_id].lock().unwrap().get_strand_color(s_id)
    }

    pub fn save_design(&mut self, path: &PathBuf) {
        if let Some(d_id) = self.selected_design {
            self.designs[d_id].lock().unwrap().save_to(path)
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

    pub fn notify_selection(
        &mut self,
        selected_design: Option<u32>,
        selected_strand: Option<usize>,
    ) {
        self.selected_design = selected_design.map(|x| x as usize);
        self.selected_strand = selected_strand;
        self.new_strand = self.selected_strand.is_some();
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
}

#[derive(Clone)]
pub enum AppNotification<'a> {
    MovementEnded,
    Rotation(&'a DesignRotation),
    Translation(&'a DesignTranslation),
}
