/// The Mediator coordinates the interaction between the designs and the applications.
/// When a design is modified, it notifies the mediator of its changes and the mediator forwards
/// that information to the applications.
///
/// When an application wants to modify a design, it makes the modification request to the
/// mediator. 
///
/// The mediator also holds data that is common to all applications.

use std::sync::{Arc, Mutex};
use std::collections::HashSet;

use crate::design;

use design::{Design, DesignNotification, DesignRotation, DesignTranslation};

pub type MediatorPtr = Arc<Mutex<Mediator>>;

pub struct Mediator {
    applications: Vec<Arc<Mutex<dyn Application>>>,
    designs: Vec<Arc<Mutex<Design>>>
}

#[derive(Clone)]
pub enum Notification<'a> {
    DesignNotification(DesignNotification),
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

    pub fn clear_designs(&mut self) {
        self.notify_apps(Notification::ClearDesigns)
    }

    pub fn notify_apps(&mut self, notification: Notification) {
        for app_wrapper in self.applications.clone() {
            let mut app = app_wrapper.lock().unwrap(); 
            app.on_notify(notification.clone());
        }
    }

    pub fn notify_all_designs(&mut self, notification: AppNotification) {
        for design_wrapper in self.designs.clone() {
            design_wrapper.lock().unwrap().on_notify(notification.clone())
        }
    }

    pub fn notify_designs(&mut self, designs: &HashSet<u32>, notification: AppNotification) {
        for design_id in designs.iter() {
            self.designs.clone()[*design_id as usize].lock().unwrap().on_notify(notification.clone());
            //design.on_notify(notification.clone(), self);
        }
    }

    pub fn observe_designs(&mut self) {
        for design_wrapper in self.designs.clone() {
            if let Some(notification) = design_wrapper.lock().unwrap().view_was_updated() {
                self.notify_apps(Notification::DesignNotification(notification))
            }
        }
    }
}

#[derive(Clone)]
pub enum AppNotification<'a> {
    MovementEnded,
    Rotation(&'a DesignRotation),
    Translation(&'a DesignTranslation),
}
