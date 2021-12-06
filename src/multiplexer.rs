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
//! This modules handles the separation of the window into different regions.
//!
//! The layout manager split the window into different regions and attribute each region to an
//! an application or a gui component.
//!
//! In addition, the multiplexer holds a Vec of overlays which are floating regions.
//!
//! When an event is recieved by the window, the multiplexer is in charge of forwarding it to the
//! appropriate application, gui component or overlay. The multiplexer also handles some events
//! like resizing events of keyboard input that should be handled independently of the foccussed
//! region.
//!
//!
//!
//! The multiplexer is also in charge of drawing to the frame.
use super::{Action, Requests};
use crate::gui::UiSize;
use crate::utils::texture::SampledTexture;
use crate::PhySize;
use ensnano_interactor::{ActionMode, SelectionMode};
use iced_wgpu::wgpu;
use iced_winit::winit;
use iced_winit::winit::event::*;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wgpu::Device;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, WindowEvent},
    window::CursorIcon,
};

mod layout_manager;
use ensnano_interactor::graphics::{DrawArea, ElementType, SplitMode};
use layout_manager::{LayoutTree, PixelRegion};

/// A structure that handles the division of the window into different `DrawArea`
pub struct Multiplexer {
    /// The *physical* size of the window
    pub window_size: PhySize,
    /// The scale factor of the window
    pub scale_factor: f64,
    /// The object mapping pixels to drawing areas
    layout_manager: LayoutTree,
    /// The Element on which the mouse cursor is currently on.
    focus: Option<ElementType>,
    /// The *physical* position of the cursor on the focus area
    cursor_position: PhysicalPosition<f64>,
    /// The area that are drawn on top of the application
    overlays: Vec<Overlay>,
    /// The texture on which the scene is rendered
    scene_texture: Option<MultiplexerTexture>,
    /// The texture on which the top bar gui is rendered
    top_bar_texture: Option<MultiplexerTexture>,
    /// The texture on which the left pannel is rendered
    left_pannel_texture: Option<MultiplexerTexture>,
    /// The textures on which the overlays are rendered
    overlays_textures: Vec<MultiplexerTexture>,
    /// The texture on wich the grid is rendered
    grid_panel_texture: Option<MultiplexerTexture>,
    /// The texutre on which the flat scene is rendered,
    status_bar_texture: Option<MultiplexerTexture>,
    flat_scene_texture: Option<MultiplexerTexture>,
    /// The pointer the node that separate the left pannel from the scene
    left_pannel_split: usize,
    /// The pointer to the node that separate the top bar from the scene
    top_bar_split: usize,
    /// The pointer to the node that separtate the status bar from the scene
    status_bar_split: usize,
    device: Rc<Device>,
    pipeline: Option<wgpu::RenderPipeline>,
    split_mode: SplitMode,
    requests: Arc<Mutex<Requests>>,
    state: State,
    modifiers: ModifiersState,
    ui_size: UiSize,
    pub invert_y_scroll: bool,
    pub icon: Option<CursorIcon>,
}

const MAX_LEFT_PANNEL_WIDTH: f64 = 200.;
const MAX_STATUS_BAR_HEIGHT: f64 = 50.;

impl Multiplexer {
    /// Create a new multiplexer for a window with size `window_size`.
    pub fn new(
        window_size: PhySize,
        scale_factor: f64,
        device: Rc<Device>,
        requests: Arc<Mutex<Requests>>,
    ) -> Self {
        let ui_size: UiSize = Default::default();
        let mut layout_manager = LayoutTree::new();
        let top_pannel_prop =
            exact_proportion(ui_size.top_bar() * scale_factor, window_size.height as f64);
        let top_bar_split = 0;
        let (top_bar, scene) = layout_manager.hsplit(0, top_pannel_prop, false);
        let left_pannel_split = scene;
        let left_pannel_prop = proportion(
            0.2,
            MAX_LEFT_PANNEL_WIDTH * scale_factor,
            window_size.width as f64,
        );
        let (left_pannel, scene) = layout_manager.vsplit(scene, left_pannel_prop, true);
        let scene_height = (1. - top_pannel_prop) * window_size.height as f64;
        let status_bar_prop = exact_proportion(MAX_STATUS_BAR_HEIGHT * scale_factor, scene_height);
        let status_bar_split = scene;
        let (scene, status_bar) = layout_manager.hsplit(scene, 1. - status_bar_prop, false);
        //let (scene, grid_panel) = layout_manager.hsplit(scene, 0.8);
        layout_manager.attribute_element(top_bar, ElementType::TopBar);
        layout_manager.attribute_element(scene, ElementType::Scene);
        layout_manager.attribute_element(status_bar, ElementType::StatusBar);
        layout_manager.attribute_element(left_pannel, ElementType::LeftPanel);
        //layout_manager.attribute_element(grid_panel, ElementType::GridPanel);
        let mut ret = Self {
            window_size,
            scale_factor,
            layout_manager,
            focus: None,
            cursor_position: PhysicalPosition::new(-1., -1.),
            scene_texture: None,
            flat_scene_texture: None,
            top_bar_texture: None,
            left_pannel_texture: None,
            grid_panel_texture: None,
            status_bar_texture: None,
            overlays: Vec::new(),
            overlays_textures: Vec::new(),
            device,
            pipeline: None,
            split_mode: SplitMode::Scene3D,
            requests,
            left_pannel_split,
            status_bar_split,
            top_bar_split,
            state: State::Normal {
                mouse_position: PhysicalPosition::new(-1., -1.),
            },
            modifiers: ModifiersState::empty(),
            ui_size,
            invert_y_scroll: false,
            icon: None,
        };
        ret.generate_textures();
        ret
    }

    /// Return a view of the texture on which the element must be rendered
    pub fn get_texture_view(&self, element_type: ElementType) -> Option<&wgpu::TextureView> {
        match element_type {
            ElementType::Scene => self.scene_texture.as_ref().map(|t| &t.texture.view),
            ElementType::LeftPanel => self.left_pannel_texture.as_ref().map(|t| &t.texture.view),
            ElementType::TopBar => self.top_bar_texture.as_ref().map(|t| &t.texture.view),
            ElementType::Overlay(n) => Some(&self.overlays_textures[n].texture.view),
            ElementType::GridPanel => self.grid_panel_texture.as_ref().map(|t| &t.texture.view),
            ElementType::FlatScene => self.flat_scene_texture.as_ref().map(|t| &t.texture.view),
            ElementType::StatusBar => self.status_bar_texture.as_ref().map(|t| &t.texture.view),
            ElementType::Unattributed => unreachable!(),
        }
    }

    fn get_texture_size(&self, element_type: ElementType) -> Option<DrawArea> {
        match element_type {
            ElementType::Scene => self.scene_texture.as_ref().map(|t| t.area),
            ElementType::LeftPanel => self.left_pannel_texture.as_ref().map(|t| t.area),
            ElementType::TopBar => self.top_bar_texture.as_ref().map(|t| t.area),
            ElementType::Overlay(n) => Some(self.overlays_textures[n].area),
            ElementType::GridPanel => self.grid_panel_texture.as_ref().map(|t| t.area),
            ElementType::FlatScene => self.flat_scene_texture.as_ref().map(|t| t.area),
            ElementType::StatusBar => self.status_bar_texture.as_ref().map(|t| t.area),
            ElementType::Unattributed => unreachable!(),
        }
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        window: &crate::Window,
    ) {
        if self.pipeline.is_none() {
            let bg_layout = &self.top_bar_texture.as_ref().unwrap().texture.bg_layout;
            self.pipeline = Some(create_pipeline(self.device.as_ref(), bg_layout));
        }
        let clear_color = wgpu::Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        };

        let msaa_texture = None;

        let attachment = if msaa_texture.is_some() {
            msaa_texture.as_ref().unwrap()
        } else {
            target
        };

        let resolve_target = if msaa_texture.is_some() {
            Some(target)
        } else {
            None
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: attachment,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        if self.window_size.width > 0 && self.window_size.height > 0 {
            for element in [
                ElementType::TopBar,
                ElementType::LeftPanel,
                ElementType::GridPanel,
                ElementType::Scene,
                ElementType::FlatScene,
                ElementType::StatusBar,
            ]
            .iter()
            {
                if let Some(area) = self.get_texture_size(*element) {
                    render_pass.set_bind_group(0, self.get_bind_group(element), &[]);

                    render_pass.set_viewport(
                        area.position.x as f32,
                        area.position.y as f32,
                        area.size.width as f32,
                        area.size.height as f32,
                        0.0,
                        1.0,
                    );
                    let width = area
                        .size
                        .width
                        .min(window.inner_size().width - area.position.x);
                    let height = area
                        .size
                        .height
                        .min(window.inner_size().height - area.position.y);
                    render_pass.set_scissor_rect(area.position.x, area.position.y, width, height);
                    render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
                    render_pass.draw(0..4, 0..1);
                }
            }
        }
    }

    fn get_bind_group(&self, element_type: &ElementType) -> &wgpu::BindGroup {
        match element_type {
            ElementType::TopBar => &self.top_bar_texture.as_ref().unwrap().texture.bind_group,
            ElementType::LeftPanel => {
                &self
                    .left_pannel_texture
                    .as_ref()
                    .unwrap()
                    .texture
                    .bind_group
            }
            ElementType::Scene => &self.scene_texture.as_ref().unwrap().texture.bind_group,
            ElementType::FlatScene => &self.flat_scene_texture.as_ref().unwrap().texture.bind_group,
            ElementType::GridPanel => &self.grid_panel_texture.as_ref().unwrap().texture.bind_group,
            ElementType::Overlay(n) => &self.overlays_textures[*n].texture.bind_group,
            ElementType::StatusBar => &self.status_bar_texture.as_ref().unwrap().texture.bind_group,
            ElementType::Unattributed => unreachable!(),
        }
    }

    /// Return the drawing area attributed to an element.
    pub fn get_draw_area(&self, element_type: ElementType) -> Option<DrawArea> {
        use ElementType::Overlay;
        let (position, size) = if let Overlay(n) = element_type {
            (self.overlays[n].position, self.overlays[n].size)
        } else {
            let (left, top, right, bottom) = self.layout_manager.get_area(element_type)?;
            let top = top * self.window_size.height as f64;
            let left = left * self.window_size.width as f64;
            let bottom = bottom * self.window_size.height as f64;
            let right = right * self.window_size.width as f64;

            (
                PhysicalPosition::new(left, top).cast::<u32>(),
                PhysicalSize::new(right - left, bottom - top).cast::<u32>(),
            )
        };
        Some(DrawArea { position, size })
    }

    pub fn check_scale_factor(&mut self, window: &crate::Window) -> bool {
        if self.scale_factor != window.scale_factor() {
            self.scale_factor = window.scale_factor();
            self.window_size = window.inner_size();
            self.resize(self.window_size, self.scale_factor);

            if self.window_size.width > 0 && self.window_size.height > 0 {
                self.generate_textures();
            }
            true
        } else {
            false
        }
    }

    /// Forwards event to the elment on which they happen.
    pub fn event(
        &mut self,
        mut event: WindowEvent<'static>,
        resized: &mut bool,
        scale_factor_changed: &mut bool,
    ) -> Option<(WindowEvent<'static>, ElementType)> {
        let mut captured = false;
        match &mut event {
            WindowEvent::CursorMoved { position, .. } => match &mut self.state {
                State::Resizing {
                    region,
                    mouse_position,
                    clicked_position,
                    old_proportion,
                } => {
                    *mouse_position = *position;
                    let mut position = position.clone();
                    position.x /= self.window_size.width as f64;
                    position.y /= self.window_size.height as f64;
                    *resized = true;
                    self.layout_manager.resize_click(
                        *region,
                        &position,
                        &clicked_position,
                        *old_proportion,
                    );
                    self.icon = Some(CursorIcon::EwResize);
                    captured = true;
                }

                State::Normal { mouse_position, .. } => {
                    *mouse_position = *position;
                    let &mut PhysicalPosition { x, y } = position;
                    if x > 0.0 || y > 0.0 {
                        let element = self.pixel_to_element(*position);
                        let area = match element {
                            PixelRegion::Resize(_) => {
                                self.icon = Some(CursorIcon::EwResize);
                                None
                            }
                            PixelRegion::Element(element) => {
                                self.icon = None;
                                self.focus = Some(element);
                                self.get_draw_area(element)
                            }
                            PixelRegion::Area(_) => unreachable!(),
                        }
                        .or(self.focus.and_then(|e| self.get_draw_area(e)));

                        if let Some(area) = area {
                            self.cursor_position.x = position.x - area.position.cast::<f64>().x;
                            self.cursor_position.y = position.y - area.position.cast::<f64>().y;
                        }
                    }
                }
                State::Interacting {
                    mouse_position,
                    element,
                } => {
                    *mouse_position = *position;
                    let element = element.clone();
                    let area = self.get_draw_area(element);
                    if let Some(area) = area {
                        self.cursor_position.x = position.x - area.position.cast::<f64>().x;
                        self.cursor_position.y = position.y - area.position.cast::<f64>().y;
                    }
                }
            },
            WindowEvent::Resized(new_size) => {
                self.window_size = *new_size;
                self.resize(*new_size, self.scale_factor);
                *resized = true;
                if self.window_size.width > 0 && self.window_size.height > 0 {
                    self.generate_textures();
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.scale_factor = *scale_factor;
                self.window_size = **new_inner_size;
                self.resize(self.window_size, self.scale_factor);
                *resized = true;
                *scale_factor_changed = true;
                if self.window_size.width > 0 && self.window_size.height > 0 {
                    self.generate_textures();
                }
            }
            WindowEvent::MouseInput { state, .. } => {
                let element = self.pixel_to_element(self.state.mouse_position());
                let mouse_position = self.state.mouse_position();
                match element {
                    PixelRegion::Resize(n) if *state == ElementState::Pressed => {
                        let mut clicked_position = mouse_position.clone();
                        clicked_position.x /= self.window_size.width as f64;
                        clicked_position.y /= self.window_size.height as f64;
                        let old_proportion = self.layout_manager.get_proportion(n).unwrap();
                        self.state = State::Resizing {
                            mouse_position,
                            clicked_position,
                            region: n,
                            old_proportion,
                        };
                    }
                    PixelRegion::Resize(_) => {
                        self.state = State::Normal { mouse_position };
                    }
                    PixelRegion::Element(element) => match state {
                        ElementState::Pressed => {
                            self.state = State::Interacting {
                                mouse_position,
                                element,
                            };
                        }
                        ElementState::Released => {
                            self.state = State::Normal { mouse_position };
                        }
                    },
                    _ => unreachable!(),
                }
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                captured = true;
                match *key {
                    VirtualKeyCode::Escape => {
                        self.requests.lock().unwrap().action_mode = Some(ActionMode::Normal)
                    }
                    VirtualKeyCode::Z if ctrl(&self.modifiers) => {
                        self.requests.lock().unwrap().undo = Some(());
                    }
                    VirtualKeyCode::R if ctrl(&self.modifiers) => {
                        self.requests.lock().unwrap().redo = Some(());
                    }
                    VirtualKeyCode::C if ctrl(&self.modifiers) => {
                        self.requests.lock().unwrap().copy = Some(());
                    }
                    VirtualKeyCode::V if ctrl(&self.modifiers) => {
                        self.requests.lock().unwrap().paste = Some(());
                    }
                    VirtualKeyCode::J if ctrl(&self.modifiers) => {
                        self.requests.lock().unwrap().duplication = Some(());
                    }
                    VirtualKeyCode::L if ctrl(&self.modifiers) => {
                        self.requests.lock().unwrap().anchor = Some(());
                    }
                    VirtualKeyCode::R if !ctrl(&self.modifiers) => {
                        self.requests.lock().unwrap().action_mode = Some(ActionMode::Rotate)
                    }
                    VirtualKeyCode::T => {
                        self.requests.lock().unwrap().action_mode = Some(ActionMode::Translate)
                    }
                    VirtualKeyCode::N => {
                        self.requests.lock().unwrap().selection_mode =
                            Some(SelectionMode::Nucleotide)
                    }
                    VirtualKeyCode::H => {
                        self.requests.lock().unwrap().selection_mode = Some(SelectionMode::Helix)
                    }
                    VirtualKeyCode::S if ctrl(&self.modifiers) => {
                        self.requests.lock().unwrap().save_shortcut = Some(());
                    }
                    VirtualKeyCode::O if ctrl(&self.modifiers) => {
                        self.requests
                            .lock()
                            .unwrap()
                            .keep_proceed
                            .push_back(Action::LoadDesign(None));
                    }
                    VirtualKeyCode::Q if ctrl(&self.modifiers) && cfg!(target_os = "macos") => {
                        self.requests
                            .lock()
                            .unwrap()
                            .keep_proceed
                            .push_back(Action::Exit);
                    }
                    keycode if keycode_to_num(keycode).is_some() => {
                        let n_camera = keycode_to_num(keycode).unwrap();
                        self.requests
                            .lock()
                            .unwrap()
                            .keep_proceed
                            .push_back(Action::SelectFavoriteCamera(n_camera));
                    }
                    VirtualKeyCode::S => {
                        self.requests.lock().unwrap().selection_mode = Some(SelectionMode::Strand)
                    }
                    VirtualKeyCode::K => {
                        self.requests.lock().unwrap().recolor_stapples = Some(());
                    }
                    VirtualKeyCode::Delete | VirtualKeyCode::Back => {
                        self.requests.lock().unwrap().delete_selection = Some(());
                    }
                    _ => captured = false,
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.invert_y_scroll {
                    match delta {
                        MouseScrollDelta::LineDelta(_, y) => {
                            *y *= -1.;
                        }
                        MouseScrollDelta::PixelDelta(position) => {
                            position.y *= -1.;
                        }
                    }
                }
            }
            _ => {}
        }

        if let Some(focus) = self.focus.filter(|_| !captured) {
            Some((event, focus))
        } else {
            None
        }
    }

    pub fn change_ui_size(&mut self, ui_size: UiSize, window: &iced_winit::winit::window::Window) {
        self.ui_size = ui_size;
        self.resize(window.inner_size(), window.scale_factor());
        self.generate_textures();
    }

    pub fn change_split(&mut self, split_mode: SplitMode) {
        if split_mode != self.split_mode {
            match self.split_mode {
                SplitMode::Both => {
                    let new_type = match split_mode {
                        SplitMode::Scene3D => ElementType::Scene,
                        SplitMode::Flat => ElementType::FlatScene,
                        SplitMode::Both => unreachable!(),
                    };
                    self.layout_manager.merge(ElementType::Scene, new_type);
                }
                SplitMode::Scene3D | SplitMode::Flat => {
                    let id = self
                        .layout_manager
                        .get_area_id(ElementType::Scene)
                        .or(self.layout_manager.get_area_id(ElementType::FlatScene))
                        .unwrap();
                    match split_mode {
                        SplitMode::Both => {
                            let (scene, flat_scene) = self.layout_manager.vsplit(id, 0.5, true);
                            self.layout_manager
                                .attribute_element(scene, ElementType::Scene);
                            self.layout_manager
                                .attribute_element(flat_scene, ElementType::FlatScene);
                        }
                        SplitMode::Scene3D => self
                            .layout_manager
                            .attribute_element(id, ElementType::Scene),
                        SplitMode::Flat => self
                            .layout_manager
                            .attribute_element(id, ElementType::FlatScene),
                    }
                }
            }
        }
        self.split_mode = split_mode;
        self.generate_textures();
    }

    pub fn resize(&mut self, window_size: PhySize, scale_factor: f64) -> bool {
        let ret = self.window_size != window_size;
        let top_pannel_prop = exact_proportion(
            self.ui_size.top_bar() * scale_factor,
            window_size.height as f64,
        );
        let left_pannel_prop = proportion(
            0.2,
            MAX_LEFT_PANNEL_WIDTH * scale_factor,
            window_size.width as f64,
        );
        let scene_height = (1. - top_pannel_prop) * window_size.height as f64;
        let status_bar_prop = exact_proportion(MAX_STATUS_BAR_HEIGHT * scale_factor, scene_height);
        self.layout_manager
            .resize(self.left_pannel_split, left_pannel_prop);
        self.layout_manager
            .resize(self.top_bar_split, top_pannel_prop);
        self.layout_manager
            .resize(self.status_bar_split, 1. - status_bar_prop);
        ret
    }

    fn texture(&mut self, element_type: ElementType) -> Option<MultiplexerTexture> {
        let area = self.get_draw_area(element_type)?;
        let texture = SampledTexture::create_target_texture(self.device.as_ref(), &area.size);
        Some(MultiplexerTexture { area, texture })
    }

    pub fn generate_textures(&mut self) {
        self.scene_texture = self.texture(ElementType::Scene);
        self.top_bar_texture = self.texture(ElementType::TopBar);
        self.left_pannel_texture = self.texture(ElementType::LeftPanel);
        self.grid_panel_texture = self.texture(ElementType::GridPanel);
        self.flat_scene_texture = self.texture(ElementType::FlatScene);
        self.status_bar_texture = self.texture(ElementType::StatusBar);

        self.overlays_textures.clear();
        for overlay in self.overlays.iter() {
            let size = overlay.size;
            let texture = SampledTexture::create_target_texture(self.device.as_ref(), &size);

            self.overlays_textures.push(MultiplexerTexture {
                texture,
                area: DrawArea {
                    size,
                    position: overlay.position,
                },
            });
        }
    }

    /// Maps *physical* pixels to an element
    fn pixel_to_element(&self, pixel: PhysicalPosition<f64>) -> PixelRegion {
        let pixel_u32 = pixel.cast::<u32>();
        for (n, overlay) in self.overlays.iter().enumerate() {
            if overlay.contains_pixel(pixel_u32) {
                return PixelRegion::Element(ElementType::Overlay(n));
            }
        }
        self.layout_manager.get_area_pixel(
            pixel.x / self.window_size.width as f64,
            pixel.y / self.window_size.height as f64,
        )
    }

    /// Get the drawing area attributed to an element.
    pub fn get_element_area(&self, element: ElementType) -> Option<DrawArea> {
        self.get_draw_area(element)
    }

    /// Return the *physical* position of the cursor, in the foccused element coordinates
    pub fn get_cursor_position(&self) -> PhysicalPosition<f64> {
        self.cursor_position
    }

    /// Return the foccused element
    pub fn foccused_element(&self) -> Option<ElementType> {
        self.focus
    }

    pub fn set_overlays(&mut self, overlays: Vec<Overlay>) {
        self.overlays = overlays;
        self.overlays_textures.clear();
        for overlay in self.overlays.iter_mut() {
            let size = overlay.size;
            let texture = SampledTexture::create_target_texture(self.device.as_ref(), &size);
            self.overlays_textures.push(MultiplexerTexture {
                texture,
                area: DrawArea {
                    size,
                    position: overlay.position,
                },
            });
        }
    }

    pub fn is_showing(&self, area: &ElementType) -> bool {
        match area {
            ElementType::LeftPanel | ElementType::TopBar | ElementType::StatusBar => true,
            ElementType::Scene => {
                self.split_mode == SplitMode::Scene3D || self.split_mode == SplitMode::Both
            }
            ElementType::FlatScene => {
                self.split_mode == SplitMode::Flat || self.split_mode == SplitMode::Both
            }
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct Overlay {
    pub position: PhysicalPosition<u32>,
    pub size: PhysicalSize<u32>,
}

impl Overlay {
    pub fn contains_pixel(&self, pixel: PhysicalPosition<u32>) -> bool {
        pixel.x >= self.position.x
            && pixel.y >= self.position.y
            && pixel.x < self.position.x + self.size.width
            && pixel.y < self.position.y + self.size.height
    }
}

fn create_pipeline(device: &Device, bg_layout: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    let vs_module =
        &device.create_shader_module(&wgpu::include_spirv!("multiplexer/draw.vert.spv"));
    let fs_module =
        &device.create_shader_module(&wgpu::include_spirv!("multiplexer/draw.frag.spv"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[bg_layout],
        push_constant_ranges: &[],
        label: None,
    });

    let targets = &[wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::ALL,
    }];

    let primitive = wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        strip_index_format: Some(wgpu::IndexFormat::Uint16),
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        ..Default::default()
    };

    let desc = wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "main",
            targets,
        }),
        primitive,
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        label: None,
    };

    device.create_render_pipeline(&desc)
}

fn proportion(min_prop: f64, max_size: f64, length: f64) -> f64 {
    let max_prop = max_size / length;
    max_prop.min(min_prop)
}

fn exact_proportion(size: f64, length: f64) -> f64 {
    size / length
}

enum State {
    Resizing {
        mouse_position: PhysicalPosition<f64>,
        clicked_position: PhysicalPosition<f64>,
        region: usize,
        old_proportion: f64,
    },
    Normal {
        mouse_position: PhysicalPosition<f64>,
    },
    Interacting {
        mouse_position: PhysicalPosition<f64>,
        element: ElementType,
    },
}

impl State {
    fn mouse_position(&self) -> PhysicalPosition<f64> {
        match self {
            Self::Resizing { mouse_position, .. }
            | Self::Normal { mouse_position }
            | Self::Interacting { mouse_position, .. } => *mouse_position,
        }
    }
}

fn ctrl(modifiers: &ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.logo()
    } else {
        modifiers.ctrl()
    }
}

use crate::gui::Multiplexer as GuiMultiplexer;

impl GuiMultiplexer for Multiplexer {
    fn get_draw_area(&self, element_type: ElementType) -> Option<DrawArea> {
        self.get_texture_size(element_type)
    }

    fn get_texture_view(&self, element_type: ElementType) -> Option<&wgpu::TextureView> {
        self.get_texture_view(element_type)
    }

    fn get_cursor_position(&self) -> PhysicalPosition<f64> {
        self.get_cursor_position()
    }

    fn foccused_element(&self) -> Option<ElementType> {
        self.foccused_element()
    }
}

fn keycode_to_num(keycode: VirtualKeyCode) -> Option<u32> {
    if keycode as u32 >= VirtualKeyCode::Key1 as u32
        && keycode as u32 <= VirtualKeyCode::Key0 as u32
    {
        Some(keycode as u32 - VirtualKeyCode::Key1 as u32)
    } else if keycode == VirtualKeyCode::Numpad0 {
        Some(9)
    } else if keycode as u32 >= VirtualKeyCode::Numpad1 as u32
        && keycode as u32 <= VirtualKeyCode::Numpad9 as u32
    {
        Some(keycode as u32 - VirtualKeyCode::Numpad1 as u32)
    } else {
        None
    }
}

struct MultiplexerTexture {
    area: DrawArea,
    texture: SampledTexture,
}
