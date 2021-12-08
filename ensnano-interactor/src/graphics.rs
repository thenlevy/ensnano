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

use iced_winit::winit;
use ultraviolet::Vec3;
use winit::dpi::{PhysicalPosition, PhysicalSize};
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum RenderingMode {
    Normal,
    Cartoon,
}

pub const ALL_RENDERING_MODE: [RenderingMode; 2] = [RenderingMode::Normal, RenderingMode::Cartoon];

impl Default for RenderingMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum Background3D {
    Sky,
    White,
}

pub const ALL_BACKGROUND3D: [Background3D; 2] = [Background3D::Sky, Background3D::White];

impl Default for Background3D {
    fn default() -> Self {
        Self::Sky
    }
}

impl std::fmt::Display for Background3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ret = match self {
            Self::White => "White",
            Self::Sky => "Sky",
        };
        write!(f, "{}", ret)
    }
}

impl std::fmt::Display for RenderingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ret = match self {
            Self::Normal => "Normal",
            Self::Cartoon => "Cartoon",
        };
        write!(f, "{}", ret)
    }
}

pub mod fog_kind {
    pub const NO_FOG: u32 = 0;
    pub const TRANSPARENT_FOG: u32 = 1;
    pub const DARK_FOG: u32 = 2;
}

#[derive(Debug, Clone)]
pub struct FogParameters {
    pub radius: f32,
    pub length: f32,
    pub fog_kind: u32,
    pub from_camera: bool,
    pub alt_fog_center: Option<Vec3>,
}

impl FogParameters {
    pub fn new() -> Self {
        Self {
            radius: 10.,
            length: 10.,
            fog_kind: fog_kind::NO_FOG,
            from_camera: true,
            alt_fog_center: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitMode {
    Flat,
    Scene3D,
    Both,
}

pub type PhySize = PhysicalSize<u32>;

/// A structure that represents an area on which an element can be drawn
#[derive(Clone, Copy, Debug)]
pub struct DrawArea {
    /// The top left corner of the element
    pub position: PhysicalPosition<u32>,
    /// The *physical* size of the element
    pub size: PhySize,
}

/// The different elements represented on the scene. Each element is instanciated once.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ElementType {
    /// The top menu bar
    TopBar,
    /// The 3D scene
    Scene,
    /// The flat Scene
    FlatScene,
    /// The Left Panel
    LeftPanel,
    /// The status bar
    StatusBar,
    GridPanel,
    /// An overlay area
    Overlay(usize),
    /// An area that has not been attributed to an element
    Unattributed,
}

impl ElementType {
    pub fn is_gui(&self) -> bool {
        match self {
            ElementType::TopBar | ElementType::LeftPanel | ElementType::StatusBar => true,
            _ => false,
        }
    }

    pub fn is_scene(&self) -> bool {
        match self {
            ElementType::Scene | ElementType::FlatScene => true,
            _ => false,
        }
    }
}
