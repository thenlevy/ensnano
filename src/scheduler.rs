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

use super::{AppState, Multiplexer};
use ensnano_interactor::application::Application;
use ensnano_interactor::graphics::ElementType;
use iced_wgpu::wgpu;
use iced_winit::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The scheduler is responsible for running the different applications
pub struct Scheduler {
    applications: HashMap<ElementType, Arc<Mutex<dyn Application<AppState = AppState>>>>,
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
        application: Arc<Mutex<dyn Application<AppState = AppState>>>,
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
        app_state: AppState,
    ) -> Option<ensnano_interactor::CursorIcon> {
        if let Some(app) = self.applications.get_mut(&area) {
            app.lock()
                .unwrap()
                .on_event(event, cursor_position, &app_state)
        } else {
            None
        }
    }

    pub fn check_redraw(
        &mut self,
        multiplexer: &Multiplexer,
        dt: Duration,
        app_state: AppState,
    ) -> bool {
        log::debug!("Scheduler checking redraw");
        self.needs_redraw.clear();
        for (area, app) in self.applications.iter_mut() {
            if multiplexer.is_showing(area)
                && app.lock().unwrap().needs_redraw(dt, app_state.clone())
            {
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
