//! This modules defines a 2D camera for the FlatScene.
//!
//! The `Globals` struct contains the value that must be send to the GPU to compute the view
//! matrix. The `Camera` struct modifies a `Globals` attribute and perform some view <-> world
//! coordinate conversion.

use crate::consts::*;
use iced_winit::winit;
use ultraviolet::Vec2;
use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};
pub struct Camera {
    globals: Globals,
    was_updated: bool,
    old_globals: Globals,
    pub bottom: bool,
}

impl Camera {
    pub fn new(globals: Globals, bottom: bool) -> Self {
        Self {
            old_globals: globals,
            globals,
            was_updated: true,
            bottom,
        }
    }

    /// Return true if the globals have been modified since the last time `self.get_update()` was
    /// called.
    pub fn was_updated(&self) -> bool {
        self.was_updated
    }

    /// Return the globals
    pub fn get_globals(&self) -> &Globals {
        &self.globals
    }

    /// Return the globals if self was updated,
    pub fn update(&mut self) -> Option<&Globals> {
        if self.was_updated {
            self.was_updated = false;
            Some(&self.globals)
        } else {
            None
        }
    }

    /// Moves the camera, according to a mouse movement expressed in *normalized screen
    /// coordinates*
    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        let (x, y) = self.transform_vec(delta_x, delta_y);
        self.globals.scroll_offset[0] = self.old_globals.scroll_offset[0] - x;
        self.globals.scroll_offset[1] = self.old_globals.scroll_offset[1] - y;
        self.was_updated = true;
    }

    /// Perform a zoom so that the point under the cursor stays at the same position on display
    pub fn process_scroll(
        &mut self,
        delta: &MouseScrollDelta,
        cursor_position: PhysicalPosition<f64>,
    ) {
        let scroll = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => *scroll,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                (*scroll as f32) / 100.
            }
        }
        .min(1.)
        .max(-1.);
        let mult_const = 1.25_f32.powf(scroll);
        let fixed_point =
            Vec2::from(self.screen_to_world(cursor_position.x as f32, cursor_position.y as f32));
        self.globals.zoom *= mult_const;
        self.globals.zoom = self.globals.zoom.min(MAX_ZOOM_2D);
        let delta = fixed_point
            - Vec2::from(self.screen_to_world(cursor_position.x as f32, cursor_position.y as f32));
        self.globals.scroll_offset[0] += delta.x;
        self.globals.scroll_offset[1] += delta.y;
        self.end_movement();
        self.was_updated = true;
    }

    /// Descrete zoom on the scene
    pub fn zoom_in(&mut self) {
        self.globals.zoom *= 1.25;
        self.was_updated = true;
    }

    /// Descrete zoom out of the scene
    pub fn zoom_out(&mut self) {
        self.globals.zoom *= 0.8;
        self.was_updated = true;
    }

    /// Notify the camera that the current movement is over.
    pub fn end_movement(&mut self) {
        self.old_globals = self.globals;
    }

    /// Notify the camera that the size of the drawing area has been modified
    pub fn resize(&mut self, res_x: f32, res_y: f32) {
        self.globals.resolution[0] = res_x;
        self.globals.resolution[1] = res_y;
        self.was_updated = true;
    }

    pub fn set_center(&mut self, center: Vec2) {
        self.globals.scroll_offset = center.into();
        self.was_updated = true;
        self.end_movement();
    }

    /// Convert a *vector* in screen coordinate to a vector in world coordinate. (Does not apply
    /// the translation)
    fn transform_vec(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.globals.resolution[0] * x / self.globals.zoom,
            self.globals.resolution[1] * y / self.globals.zoom,
        )
    }

    /// Convert a *point* in screen ([0, x_res] * [0, y_res]) coordinate to a point in world coordiantes.
    pub fn screen_to_world(&self, x_screen: f32, y_screen: f32) -> (f32, f32) {
        // The screen coordinates have the y axis pointed down, and so does the 2d world
        // coordinates. So we do not flip the y axis.
        let x_ndc = 2. * x_screen / self.globals.resolution[0] - 1.;
        let y_ndc = if self.bottom {
            2. * (y_screen - self.globals.resolution[1]) / self.globals.resolution[1] - 1.
        } else {
            2. * y_screen / self.globals.resolution[1] - 1.
        };
        (
            x_ndc * self.globals.resolution[0] / (2. * self.globals.zoom)
                + self.globals.scroll_offset[0],
            y_ndc * self.globals.resolution[1] / (2. * self.globals.zoom)
                + self.globals.scroll_offset[1],
        )
    }

    /// Convert a *point* in world coordinates to a point in normalized screen ([0, 1] * [0, 1]) coordinates
    pub fn world_to_norm_screen(&self, x_world: f32, y_world: f32) -> (f32, f32) {
        // The screen coordinates have the y axis pointed down, and so does the 2d world
        // coordinates. So we do not flip the y axis.
        let temp = (
            x_world - self.globals.scroll_offset[0],
            y_world - self.globals.scroll_offset[1],
        );
        let coord_ndc = (
            temp.0 * 2. * self.globals.zoom / self.globals.resolution[0],
            temp.1 * 2. * self.globals.zoom / self.globals.resolution[1],
        );
        ((coord_ndc.0 + 1.) / 2., (coord_ndc.1 + 1.) / 2.)
    }

    pub fn fit(&mut self, rectangle: FitRectangle) {
        let FitRectangle {
            min_x,
            max_x,
            min_y,
            max_y,
        } = rectangle;
        let zoom_x = self.globals.resolution[0] / (max_x - min_x);
        let zoom_y = self.globals.resolution[1] / (max_y - min_y);
        if zoom_x < zoom_y {
            self.globals.zoom = zoom_x;
        } else {
            self.globals.zoom = zoom_y;
        }
        self.globals.scroll_offset[0] = self.globals.resolution[0] / 2. / self.globals.zoom + min_x;
        self.globals.scroll_offset[1] = self.globals.resolution[1] / 2. / self.globals.zoom + min_x;
        self.was_updated = true;
        self.end_movement();
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Globals {
    pub resolution: [f32; 2],
    pub scroll_offset: [f32; 2],
    pub zoom: f32,
    pub _padding: f32,
}

unsafe impl bytemuck::Zeroable for Globals {}
unsafe impl bytemuck::Pod for Globals {}

#[derive(Debug, Clone, Copy)]
pub struct FitRectangle {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

impl FitRectangle {
    pub fn add_point(&mut self, point: ultraviolet::Vec2) {
        self.min_x = self.min_x.min(point.x);
        self.max_x = self.max_x.max(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_y = self.max_y.max(point.y);
    }
}
