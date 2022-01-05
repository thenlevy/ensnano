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
    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) -> (f32, f32) {
        let (x, y) = self.transform_vec(delta_x, delta_y);
        self.translate_by_vec(x, y);
        (x, y)
    }

    pub fn translate_by_vec(&mut self, x: f32, y: f32) {
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

    pub fn zoom_closer(&mut self) {
        self.globals.zoom = self.globals.zoom.max(MAX_ZOOM_2D / 2.);
    }

    /// Descrete zoom on the scene
    #[allow(dead_code)]
    pub fn zoom_in(&mut self) {
        self.globals.zoom *= 1.25;
        self.was_updated = true;
    }

    /// Descrete zoom out of the scene
    #[allow(dead_code)]
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

    pub fn set_zoom(&mut self, zoom: f32) {
        self.globals.zoom = zoom;
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

    pub fn norm_screen_to_world(&self, x_normed: f32, y_normed: f32) -> (f32, f32) {
        if self.bottom {
            self.screen_to_world(
                x_normed * self.globals.resolution[0],
                (y_normed + 1.) * self.globals.resolution[1],
            )
        } else {
            self.screen_to_world(
                x_normed * self.globals.resolution[0],
                y_normed * self.globals.resolution[1],
            )
        }
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

    pub fn fit(&mut self, mut rectangle: FitRectangle) {
        rectangle.finish();
        rectangle.adjust_height(1.1);
        let zoom_x = self.globals.resolution[0] / rectangle.width().unwrap();
        let zoom_y = self.globals.resolution[1] / rectangle.height().unwrap();
        if zoom_x < zoom_y {
            self.globals.zoom = zoom_x;
        } else {
            self.globals.zoom = zoom_y;
        }
        let (center_x, center_y) = rectangle.center().unwrap();
        self.globals.scroll_offset[0] = center_x;
        self.globals.scroll_offset[1] = center_y;
        self.was_updated = true;
        self.end_movement();
    }

    pub fn init_fit(&mut self, mut rectangle: FitRectangle) {
        let zoom_x = self.globals.resolution[0] / rectangle.width().unwrap();
        let zoom_y = self.globals.resolution[1] / rectangle.height().unwrap();
        if zoom_x < zoom_y {
            self.globals.zoom = zoom_x;
            // adjust the height of the rectangle to force the top position
            rectangle.max_y = Some(
                rectangle.min_y.unwrap()
                    + self.globals.resolution[1] * rectangle.width().unwrap()
                        / self.globals.resolution[0],
            );
        } else {
            self.globals.zoom = zoom_y;
        }
        let (center_x, center_y) = rectangle.center().unwrap();
        self.globals.scroll_offset[0] = center_x;
        self.globals.scroll_offset[1] = center_y;
        self.was_updated = true;
        self.end_movement();
    }

    pub fn can_see_world_point(&self, point: Vec2) -> bool {
        let normalized_coord = self.world_to_norm_screen(point.x, point.y);
        normalized_coord.0 >= 0.015
            && normalized_coord.0 <= 1. - 0.015
            && normalized_coord.1 >= 0.015
            && normalized_coord.1 <= 1. - 0.015
    }

    pub fn get_visible_rectangle(&self) -> FitRectangle {
        let top_left: Vec2 = self.screen_to_world(0., 0.).into();
        let bottom_right: Vec2 = self.norm_screen_to_world(1., 1.).into();

        FitRectangle {
            min_x: Some(top_left.x),
            min_y: Some(top_left.y),
            max_x: Some(bottom_right.x),
            max_y: Some(bottom_right.y),
        }
    }

    pub fn swap(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.globals, &mut other.globals);
        self.was_updated = true;
        other.was_updated = true;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub resolution: [f32; 2],
    pub scroll_offset: [f32; 2],
    pub zoom: f32,
    /// For word alignement
    pub _padding: f32,
}

impl Globals {
    pub fn default(resolution: [f32; 2]) -> Self {
        Self {
            resolution,
            scroll_offset: [10.0, 40.0],
            zoom: 16.0,
            _padding: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FitRectangle {
    pub min_x: Option<f32>,
    pub max_x: Option<f32>,
    pub min_y: Option<f32>,
    pub max_y: Option<f32>,
}

impl FitRectangle {
    pub const INITIAL_RECTANGLE: Self = Self {
        min_x: Some(-7.),
        max_x: Some(50.),
        min_y: Some(-1.),
        max_y: Some(8.),
    };

    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_point(&mut self, point: ultraviolet::Vec2) {
        self.min_x = self.min_x.map(|x| x.min(point.x)).or(Some(point.x));
        self.max_x = self.max_x.map(|x| x.max(point.x)).or(Some(point.x));
        self.min_y = self.min_y.map(|y| y.min(point.y)).or(Some(point.y));
        self.max_y = self.max_y.map(|y| y.max(point.y)).or(Some(point.y));
    }

    pub fn finish(&mut self) {
        let width = self.width().unwrap_or(0.);
        let height = self.height().unwrap_or(0.);

        if width <= Self::min_width() {
            let diff = Self::min_width() - width;
            self.min_x = self.min_x.map(|x| x - diff / 4.).or(Some(-5.));
            self.max_x = self.max_x.map(|x| x + 3. * diff / 4.).or(Some(15.))
        }

        if height <= Self::min_height() {
            let diff = Self::min_height() - height;
            self.min_y = self.min_y.map(|y| y - diff / 7.).or(Some(-5.));
            self.max_y = self.max_y.map(|y| y + 6. * diff / 7.).or(Some(30.));
        }
    }

    pub fn adjust_height(&mut self, factor: f32) {
        let height = self.height().unwrap_or(0.);
        let delta = (factor - 1.) / 2.;
        self.min_y.as_mut().map(|y| *y -= delta * height);
        self.max_y.as_mut().map(|y| *y += delta * height);
    }

    fn width(&self) -> Option<f32> {
        let max_x = self.max_x?;
        let min_x = self.min_x?;
        Some(max_x - min_x)
    }

    fn height(&self) -> Option<f32> {
        let max_y = self.max_y?;
        let min_y = self.min_y?;
        Some(max_y - min_y)
    }

    fn center(&self) -> Option<(f32, f32)> {
        let max_x = self.max_x?;
        let min_x = self.min_x?;
        let max_y = self.max_y?;
        let min_y = self.min_y?;
        Some(((max_x + min_x) / 2., (max_y + min_y) / 2.))
    }

    fn min_width() -> f32 {
        20f32
    }

    fn min_height() -> f32 {
        35f32
    }

    pub fn splited_vertically(mut self) -> (Self, Self) {
        self.finish();
        let mut top = self.clone();
        let mut bottom = self.clone();

        let middle = Some((self.max_y.unwrap() + self.min_y.unwrap()) / 2.);
        top.max_y = middle.clone();
        bottom.min_y = middle.clone();
        (top, bottom)
    }

    pub fn with_double_height(mut self) -> Self {
        self.finish();
        let height = self.height().unwrap();
        self.max_y = Some(self.min_y.unwrap() + height * 2.);
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_rectangle() {
        let rect = FitRectangle::new();
        assert!(rect.width().is_none());
        assert!(rect.height().is_none());
    }

    #[test]
    fn minimum_height_after_finish() {
        let mut rect = FitRectangle::new();
        rect.finish();
        let height = rect.height().unwrap();
        assert!(height >= FitRectangle::min_height())
    }

    #[test]
    fn minimum_width_after_finish() {
        let mut rect = FitRectangle::new();
        rect.finish();
        let width = rect.width().unwrap();
        assert!(width >= FitRectangle::min_width())
    }

    #[test]
    fn correct_width() {
        let mut rect = FitRectangle::new();
        rect.add_point(Vec2::new(-3., 4.));
        rect.add_point(Vec2::new(-2., 5.));
        rect.add_point(Vec2::new(-1., -2.));
        let width = rect.width().unwrap();
        assert!((width - (2.)).abs() < 1e-5);
    }

    #[test]
    fn correct_height() {
        let mut rect = FitRectangle::new();
        rect.add_point(Vec2::new(-3., 4.));
        rect.add_point(Vec2::new(-2., 5.));
        rect.add_point(Vec2::new(-1., -2.));
        let height = rect.height().unwrap();
        assert!((height - 7.).abs() < 1e-5);
    }
}
