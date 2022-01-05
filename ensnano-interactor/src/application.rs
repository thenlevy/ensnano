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

use super::graphics::*;
use super::Selection;
use ensnano_design::group_attributes::GroupPivot;
use ensnano_design::Nucl;
use iced_wgpu::wgpu;
use iced_winit::winit;
use std::time::Duration;
use ultraviolet::{Rotor3, Vec3};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ModifiersState, WindowEvent},
};

pub trait Application {
    type AppState;
    /// For notification about the data
    fn on_notify(&mut self, notification: Notification);
    /// The method must be called when the window is resized or when the drawing area is modified
    fn on_resize(&mut self, window_size: PhysicalSize<u32>, area: DrawArea);
    /// The methods is used to forwards the window events to applications
    fn on_event(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        app_state: &Self::AppState,
    );
    /// The method is used to forwards redraw_requests to applications
    fn on_redraw_request(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        dt: Duration,
    );
    fn needs_redraw(&mut self, dt: Duration, app_state: Self::AppState) -> bool;
    fn get_position_for_new_grid(&self) -> Option<(Vec3, Rotor3)> {
        None
    }

    fn get_camera(&self) -> Option<(Vec3, Rotor3)> {
        None
    }
    fn get_current_selection_pivot(&self) -> Option<GroupPivot> {
        None
    }

    fn is_splited(&self) -> bool;
}

#[derive(Clone, Debug)]
/// A notification that must be send to the application
pub enum Notification {
    /// The application must show/hide the sequences
    ToggleText(bool),
    /// The scroll sensitivity has been modified
    NewSensitivity(f32),
    FitRequest,
    /// The designs have been deleted
    ClearDesigns,
    /// A save request has been filled
    Save(usize),
    /// The 3d camera must face a given target
    CameraTarget((Vec3, Vec3)),
    TeleportCamera(Vec3, Rotor3),
    CameraRotation(f32, f32, f32),
    Centering(Nucl, usize),
    CenterSelection(Selection, AppId),
    ShowTorsion(bool),
    ModifersChanged(ModifiersState),
    Split2d,
    Redim2dHelices(bool),
    Background3D(Background3D),
    RenderingMode(RenderingMode),
    Fog(FogParameters),
    WindowFocusLost,
    FlipSplitViews,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum AppId {
    FlatScene,
    Scene,
    Organizer,
    Mediator,
}
