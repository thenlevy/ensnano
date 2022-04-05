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
use super::*;
use crate::DesignReader;
use ensnano_design::ultraviolet::Vec2;
use ensnano_design::{
    grid::{GridId, GridObject},
    BezierPlaneId,
};
use ensnano_interactor::{ActionMode, CursorIcon};
use std::borrow::Cow;
use std::cell::RefCell;
use std::time::Instant;

use super::AppState;

mod dragging_state;
use dragging_state::ClickInfo;
mod point_and_click_state;
use point_and_click_state::PointAndClicking;
mod event_context;
pub use event_context::EventContext;
use event_context::*;

pub(super) type State<S> = RefCell<Box<dyn ControllerState<S>>>;

pub(super) fn initial_state<S: AppState>() -> State<S> {
    RefCell::new(Box::new(NormalState {
        mouse_position: PhysicalPosition::new(-1., -1.),
    }))
}

pub(super) struct Transition<S: AppState> {
    pub new_state: Option<Box<dyn ControllerState<S>>>,
    pub consequences: Consequence,
}

impl<S: AppState> Transition<S> {
    pub fn nothing() -> Self {
        Self {
            new_state: None,
            consequences: Consequence::Nothing,
        }
    }

    pub fn consequence(consequences: Consequence) -> Self {
        Self {
            new_state: None,
            consequences,
        }
    }

    pub fn init_building(nucls: Vec<Nucl>, clicked: bool) -> Self {
        Self {
            new_state: Some(Box::new(BuildingStrand {
                clicked,
                last_scroll: Instant::now(),
            })),
            consequences: Consequence::InitBuild(nucls),
        }
    }
}

pub(super) trait ControllerState<S: AppState> {
    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S>;

    #[allow(dead_code)]
    fn display(&self) -> Cow<'static, str>;

    fn transition_from(&self, _controller: &Controller<S>) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn transition_to(&self, _controller: &Controller<S>) -> TransistionConsequence {
        TransistionConsequence::Nothing
    }

    fn check_timers(&mut self, _controller: &Controller<S>) -> Transition<S> {
        Transition::nothing()
    }

    fn handles_color_system(&self) -> Option<HandleColors> {
        None
    }

    fn element_being_selected(&self) -> Option<SceneElement> {
        None
    }

    fn notify_scroll(&mut self) {
        ()
    }
    fn cursor(&self) -> Option<ensnano_interactor::CursorIcon> {
        None
    }
}

pub struct NormalState {
    pub mouse_position: PhysicalPosition<f64>,
}

impl<S: AppState> ControllerState<S> for NormalState {
    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        let path_id = context.get_id_of_bezier_bath_being_eddited();
        match event {
            WindowEvent::CursorMoved { .. } if context.is_pasting() => {
                self.mouse_position = context.cursor_position;
                let element = context.get_element_under_cursor();
                let element = context.convert_grid_to_grid_disc(element);
                Transition::consequence(Consequence::PasteCandidate(element))
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = context.cursor_position;
                let element = context.get_element_under_cursor();
                let element = context.convert_grid_to_grid_disc(element);
                Transition::consequence(Consequence::Candidate(element))
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if context.get_modifiers().alt() => {
                let click_info = ClickInfo::new(MouseButton::Left, context.cursor_position);
                Transition {
                    new_state: Some(Box::new(dragging_state::translating_camera(click_info))),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if context.is_pasting() => {
                let element = context.get_element_under_cursor();
                let element = context.convert_grid_to_grid_disc(element);
                Transition {
                    new_state: Some(Box::new(Pasting {
                        clicked_position: context.cursor_position,
                        element,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let element = context.get_element_under_cursor();
                log::info!("Clicked on {:?}", element);
                if let Some(SceneElement::PlaneCorner {
                    plane_id,
                    corner_type,
                }) = element
                {
                    let fixed_corner_position =
                        context.get_position_of_opposite_plane_corner(plane_id, corner_type);
                    return Transition {
                        new_state: Some(Box::new(MovingBezierCorner {
                            plane_id,
                            clicked_position: context.cursor_position,
                            fixed_corner_position,
                        })),
                        consequences: Consequence::Nothing,
                    };
                } else if let Some(SceneElement::BezierVertex { vertex_id, path_id }) = element {
                    return Transition {
                        new_state: Some(Box::new(MovingBezierVertex {
                            plane_id: None,
                            vertex_id: Some(vertex_id),
                            path_id: Some(path_id),
                        })),
                        consequences: Consequence::Nothing,
                    };
                } else if path_id.is_some() {
                    if let Some((plane_id, intersection)) = context.get_plane_under_cursor() {
                        return Transition {
                            new_state: Some(Box::new(MovingBezierVertex {
                                plane_id: Some(plane_id),
                                vertex_id: None,
                                path_id: None,
                            })),
                            consequences: Consequence::CreateBezierVertex {
                                vertex: BezierVertex::new(
                                    plane_id,
                                    Vec2::new(intersection.x, intersection.y),
                                ),
                                path: path_id.unwrap(),
                            },
                        };
                    }
                }
                match element {
                    Some(SceneElement::GridCircle(d_id, grid_position)) => {
                        if let ActionMode::BuildHelix {
                            position: position_helix,
                            length,
                        } = context.get_action_mode()
                        {
                            Transition {
                                new_state: Some(Box::new(BuildingHelix {
                                    position_helix,
                                    length_helix: length,
                                    x_helix: grid_position.x,
                                    y_helix: grid_position.y,
                                    grid_id: grid_position.grid,
                                    design_id: d_id,
                                    clicked_position: context.cursor_position,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            let adding =
                                context.get_modifiers().shift() || ctrl(context.get_modifiers());
                            let new_state = PointAndClicking::selecting(
                                context.cursor_position,
                                element,
                                adding,
                            );
                            Transition {
                                new_state: Some(Box::new(new_state)),
                                consequences: Consequence::Nothing,
                            }
                        }
                    }
                    Some(SceneElement::Grid(d_id, _)) => {
                        let grid_intersection = context.get_grid_intersection_with_cursor();
                        if let ActionMode::BuildHelix {
                            position: helix_position,
                            length,
                        } = context.get_action_mode()
                        {
                            if let Some(intersection) = grid_intersection {
                                if let Some(object) =
                                    context.get_object_at_grid_pos(intersection.grid_position())
                                {
                                    Transition {
                                        new_state: Some(Box::new(TranslatingGridObject {
                                            grid_id: intersection.grid_id,
                                            object: object.clone(),
                                            x: intersection.x,
                                            y: intersection.y,
                                        })),
                                        consequences: Consequence::HelixSelected(object.helix()),
                                    }
                                } else {
                                    Transition {
                                        new_state: Some(Box::new(BuildingHelix {
                                            position_helix: helix_position,
                                            length_helix: length,
                                            x_helix: intersection.x,
                                            y_helix: intersection.y,
                                            grid_id: intersection.grid_id,
                                            design_id: d_id,
                                            clicked_position: context.cursor_position,
                                        })),
                                        consequences: Consequence::Nothing,
                                    }
                                }
                            } else {
                                let adding = context.get_modifiers().shift()
                                    || ctrl(context.get_modifiers());
                                let new_state = PointAndClicking::selecting(
                                    context.cursor_position,
                                    element,
                                    adding,
                                );
                                Transition {
                                    new_state: Some(Box::new(new_state)),
                                    consequences: Consequence::Nothing,
                                }
                            }
                        } else {
                            let clicked_element;
                            let object;
                            if let Some(intersection) = grid_intersection.as_ref() {
                                clicked_element = Some(SceneElement::GridCircle(
                                    d_id,
                                    intersection.grid_position(),
                                ));
                                object =
                                    context.get_object_at_grid_pos(intersection.grid_position());
                            } else {
                                clicked_element = element;
                                object = None;
                            };
                            if let Some(obj) = object {
                                // if helix is Some, intersection is also Some
                                let intersection = grid_intersection.unwrap();
                                Transition {
                                    new_state: Some(Box::new(TranslatingGridObject {
                                        grid_id: intersection.grid_id,
                                        object: obj.clone(),
                                        x: intersection.x,
                                        y: intersection.y,
                                    })),
                                    consequences: Consequence::HelixSelected(obj.helix()),
                                }
                            } else {
                                let adding = context.get_modifiers().shift()
                                    || ctrl(context.get_modifiers());
                                let new_state = PointAndClicking::selecting(
                                    context.cursor_position,
                                    clicked_element,
                                    adding,
                                );
                                Transition {
                                    new_state: Some(Box::new(new_state)),
                                    consequences: Consequence::Nothing,
                                }
                            }
                        }
                    }
                    Some(SceneElement::WidgetElement(widget_id)) => {
                        let normalized_cursor_position = context.normalized_cursor_position();
                        let translation_target = if context.get_modifiers().shift() {
                            WidgetTarget::Pivot
                        } else {
                            WidgetTarget::Object
                        };
                        match widget_id {
                            UP_HANDLE_ID | DIR_HANDLE_ID | RIGHT_HANDLE_ID => Transition {
                                new_state: Some(Box::new(TranslatingWidget {
                                    direction: HandleDir::from_widget_id(widget_id),
                                    translation_target,
                                })),
                                consequences: Consequence::InitTranslation(
                                    normalized_cursor_position.x,
                                    normalized_cursor_position.y,
                                    translation_target,
                                ),
                            },
                            RIGHT_CIRCLE_ID | FRONT_CIRCLE_ID | UP_CIRCLE_ID => {
                                let target = if context.get_modifiers().shift() {
                                    WidgetTarget::Pivot
                                } else {
                                    WidgetTarget::Object
                                };

                                Transition {
                                    new_state: Some(Box::new(RotatingWidget { target })),
                                    consequences: Consequence::InitRotation(
                                        RotationMode::from_widget_id(widget_id),
                                        normalized_cursor_position.x,
                                        normalized_cursor_position.y,
                                        target,
                                    ),
                                }
                            }
                            _ => {
                                println!("WARNING UNEXPECTED WIDGET ID");
                                Transition::nothing()
                            }
                        }
                    }
                    Some(SceneElement::DesignElement(_, _))
                        if ctrl(context.get_modifiers())
                            && context.element_to_nucl(&element, true).is_some() =>
                    {
                        if let Some(nucl) = context.element_to_nucl(&element, true) {
                            Transition::consequence(Consequence::QuickXoverAttempt {
                                nucl,
                                doubled: context.get_modifiers().shift(),
                            })
                        } else {
                            Transition::nothing()
                        }
                    }
                    _ => {
                        let adding =
                            context.get_modifiers().shift() || ctrl(context.get_modifiers());
                        let new_state =
                            PointAndClicking::selecting(context.cursor_position, element, adding);
                        Transition {
                            new_state: Some(Box::new(new_state)),
                            consequences: Consequence::Nothing,
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } if ctrl(context.get_modifiers()) => {
                let click_info = ClickInfo::new(MouseButton::Middle, context.cursor_position);
                Transition {
                    new_state: Some(Box::new(dragging_state::rotating_camera(click_info))),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } => {
                let click_info = ClickInfo::new(MouseButton::Middle, context.cursor_position);
                Transition {
                    new_state: Some(Box::new(dragging_state::translating_camera(click_info))),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                let element = context.get_pivot_element();
                Transition {
                    new_state: Some(Box::new(PointAndClicking::setting_pivot(
                        context.cursor_position,
                        element,
                    ))),
                    consequences: Consequence::Nothing,
                }
            }
            _ => Transition::nothing(),
        }
    }

    fn display(&self) -> Cow<'static, str> {
        "Normal".into()
    }
}

struct TranslatingWidget {
    direction: HandleDir,
    translation_target: WidgetTarget,
}

/// What is being affected by the translation
#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub enum WidgetTarget {
    /// The selected elements
    Object,
    /// The selection's pivot
    Pivot,
}

impl<S: AppState> ControllerState<S> for TranslatingWidget {
    fn display(&self) -> Cow<'static, str> {
        format!("Translating widget, target {:?}", self.translation_target).into()
    }

    fn handles_color_system(&self) -> Option<HandleColors> {
        match self.translation_target {
            WidgetTarget::Pivot => Some(HandleColors::Cym),
            WidgetTarget::Object => Some(HandleColors::Rgb),
        }
    }

    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: context,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                Transition::consequence(Consequence::Translation(
                    self.direction,
                    mouse_x,
                    mouse_y,
                    self.translation_target,
                ))
            }
            _ => Transition::nothing(),
        }
    }

    fn cursor(&self) -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::Grabbing)
    }
}

struct TranslatingGridObject {
    object: GridObject,
    grid_id: GridId,
    x: isize,
    y: isize,
}

impl<S: AppState> ControllerState<S> for TranslatingGridObject {
    fn display(&self) -> Cow<'static, str> {
        format!(
            "Translating object {:?} on grid {:?}",
            self.object, self.grid_id
        )
        .into()
    }

    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                if let Some(intersection) = controller.view.borrow().specific_grid_intersection(
                    mouse_x as f32,
                    mouse_y as f32,
                    self.grid_id,
                ) {
                    if intersection.x != self.x || intersection.y != self.y {
                        self.x = intersection.x;
                        self.y = intersection.y;
                        Transition::consequence(Consequence::ObjectTranslated {
                            object: self.object.clone(),
                            grid: self.grid_id,
                            x: intersection.x,
                            y: intersection.y,
                        })
                    } else {
                        Transition::nothing()
                    }
                } else {
                    Transition::nothing()
                }
            }
            _ => Transition::nothing(),
        }
    }

    fn cursor(&self) -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::Grabbing)
    }
}

struct RotatingWidget {
    target: WidgetTarget,
}

impl<S: AppState> ControllerState<S> for RotatingWidget {
    fn display(&self) -> Cow<'static, str> {
        "Rotating widget".into()
    }

    fn handles_color_system(&self) -> Option<HandleColors> {
        match self.target {
            WidgetTarget::Pivot => Some(HandleColors::Cym),
            WidgetTarget::Object => Some(HandleColors::Rgb),
        }
    }

    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::MovementEnded,
            },
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                Transition::consequence(Consequence::Rotation(mouse_x, mouse_y, self.target))
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_to(&self, controller: &Controller<S>) -> TransistionConsequence {
        if self.target == WidgetTarget::Pivot {
            controller.data.borrow_mut().notify_rotating_pivot();
        }
        TransistionConsequence::Nothing
    }

    fn transition_from(&self, controller: &Controller<S>) -> TransistionConsequence {
        if self.target == WidgetTarget::Pivot {
            controller.data.borrow_mut().stop_rotating_pivot();
        }
        TransistionConsequence::Nothing
    }

    fn cursor(&self) -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::Grabbing)
    }
}

struct BuildingStrand {
    clicked: bool,
    last_scroll: Instant,
}

impl<S: AppState> ControllerState<S> for BuildingStrand {
    fn display(&self) -> Cow<'static, str> {
        "Building Strand".into()
    }

    fn notify_scroll(&mut self) {
        self.last_scroll = Instant::now()
    }

    fn check_timers(&mut self, _controller: &Controller<S>) -> Transition<S> {
        let now = Instant::now();
        if !self.clicked && (now - self.last_scroll).as_millis() > 1000 {
            Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: PhysicalPosition::new(0., 0.),
                })),
                consequences: Consequence::BuildEnded,
            }
        } else {
            Transition::nothing()
        }
    }

    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::BuildEnded,
            },
            WindowEvent::CursorMoved { .. } if self.clicked => {
                if let Some(builder) = app_state.get_strand_builders().get(0) {
                    let mouse_x = position.x / controller.area_size.width as f64;
                    let mouse_y = position.y / controller.area_size.height as f64;
                    let element = pixel_reader.set_selected_id(position);
                    let nucl = controller
                        .data
                        .borrow()
                        .element_to_nucl(&element, false)
                        .filter(|p| p.0.helix == builder.get_initial_nucl().helix);
                    let initial_position =
                        nucl.and_then(|nucl| controller.data.borrow().get_nucl_position(nucl.0, 0));

                    let position = nucl.map(|n| n.0.position).or_else(|| {
                        controller.view.borrow().compute_projection_axis(
                            builder.get_axis(),
                            mouse_x,
                            mouse_y,
                            initial_position,
                            controller.stereography.is_some(),
                        )
                    });
                    let consequence = if let Some(position) = position {
                        Consequence::Building(position)
                    } else {
                        Consequence::Nothing
                    };
                    Transition::consequence(consequence)
                } else {
                    Transition::nothing()
                }
            }
            _ => Transition::nothing(),
        }
    }

    fn cursor(&self) -> Option<ensnano_interactor::CursorIcon> {
        Some(CursorIcon::Grabbing)
    }
}

struct BuildingHelix {
    design_id: u32,
    grid_id: GridId,
    x_helix: isize,
    y_helix: isize,
    length_helix: usize,
    position_helix: isize,
    clicked_position: PhysicalPosition<f64>,
}

impl<S: AppState> ControllerState<S> for BuildingHelix {
    fn display(&self) -> Cow<'static, str> {
        "Building Helix".into()
    }

    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        match event {
            WindowEvent::CursorMoved { .. } => {
                if position_difference(self.clicked_position, position) > 5. {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::BuildHelix {
                    design_id: self.design_id,
                    grid_id: self.grid_id,
                    length: self.length_helix,
                    x: self.x_helix,
                    y: self.y_helix,
                    position: self.position_helix,
                },
            },
            _ => Transition::nothing(),
        }
    }
}

struct Pasting {
    clicked_position: PhysicalPosition<f64>,
    element: Option<SceneElement>,
}

impl<S: AppState> ControllerState<S> for Pasting {
    fn display(&self) -> Cow<'static, str> {
        "Pasting".into()
    }

    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        match event {
            WindowEvent::CursorMoved { .. } => {
                if position_difference(self.clicked_position, position) > 5. {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::Paste(self.element),
            },
            _ => Transition::nothing(),
        }
    }
}

struct MovingBezierVertex {
    plane_id: Option<BezierPlaneId>,
    vertex_id: Option<usize>,
    path_id: Option<BezierPathId>,
}

impl<S: AppState> ControllerState<S> for MovingBezierVertex {
    fn display(&self) -> Cow<'static, str> {
        "Moving bezier vertex".into()
    }

    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        if let Some(plane_id) = self.plane_id.or(self
            .path_id
            .zip(self.vertex_id)
            .and_then(|(path_id, vertex_id)| {
                app_state
                    .get_design_reader()
                    .get_bezier_vertex(path_id, vertex_id)
            })
            .map(|v| v.plane_id))
        {
            match event {
                WindowEvent::CursorMoved { .. } => {
                    let mouse_x = position.x / controller.area_size.width as f64;
                    let mouse_y = position.y / controller.area_size.height as f64;
                    let ray = controller
                        .camera_controller
                        .ray(mouse_x as f32, mouse_y as f32);
                    if let Some(intersection) = app_state
                        .get_design_reader()
                        .get_bezier_planes()
                        .get(&plane_id)
                        .and_then(|plane| plane.ray_intersection(ray.0, ray.1))
                    {
                        let path_id = self.path_id.or_else(|| {
                            if let ActionMode::EditBezierPath { path_id, .. } =
                                app_state.get_action_mode().0
                            {
                                path_id
                            } else {
                                None
                            }
                        });
                        let vertex_id = self.vertex_id.or_else(|| {
                            if let ActionMode::EditBezierPath { vertex_id, .. } =
                                app_state.get_action_mode().0
                            {
                                vertex_id
                            } else {
                                None
                            }
                        });
                        if let Some((path_id, vertex_id)) = path_id.zip(vertex_id) {
                            Transition::consequence(Consequence::MoveBezierVertex {
                                x: intersection.x,
                                y: intersection.y,
                                path_id,
                                vertex_id,
                            })
                        } else {
                            log::error!("Bad action mode {:?}", app_state.get_action_mode().0);
                            Transition::nothing()
                        }
                    } else {
                        Transition::nothing()
                    }
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state: ElementState::Released,
                    ..
                } => Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: position,
                    })),
                    consequences: Consequence::ReleaseBezierVertex,
                },
                _ => Transition::nothing(),
            }
        } else {
            log::error!("Could not get self.plane");
            Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::ReleaseBezierVertex,
            }
        }
    }
}

struct MovingBezierCorner {
    plane_id: BezierPlaneId,
    clicked_position: PhysicalPosition<f64>,
    fixed_corner_position: Vec2,
}

impl<S: AppState> ControllerState<S> for MovingBezierCorner {
    fn display(&self) -> Cow<'static, str> {
        "Moving bezier vertex".into()
    }

    fn input<'a>(
        &mut self,
        event: &WindowEvent,
        context: &'a mut EventContext<'a, S>,
    ) -> Transition<S> {
        match event {
            WindowEvent::CursorMoved { .. } => {
                let mouse_x = position.x / controller.area_size.width as f64;
                let mouse_y = position.y / controller.area_size.height as f64;
                let ray = controller
                    .camera_controller
                    .ray(mouse_x as f32, mouse_y as f32);
                let ray_origin = controller.camera_controller.ray(
                    self.clicked_position.x as f32 / controller.area_size.width as f32,
                    self.clicked_position.y as f32 / controller.area_size.height as f32,
                );
                if let Some((moving_corner, original_corner_position)) = app_state
                    .get_design_reader()
                    .get_bezier_planes()
                    .get(&self.plane_id)
                    .and_then(|plane| {
                        plane
                            .ray_intersection(ray.0, ray.1)
                            .zip(plane.ray_intersection(ray_origin.0, ray_origin.1))
                    })
                {
                    Transition::consequence(Consequence::MoveBezierCorner {
                        moving_corner: moving_corner.position(),
                        original_corner_position: original_corner_position.position(),
                        plane_id: self.plane_id,
                        fixed_corner_position: self.fixed_corner_position,
                    })
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => Transition {
                new_state: Some(Box::new(NormalState {
                    mouse_position: position,
                })),
                consequences: Consequence::ReleaseBezierCorner,
            },
            _ => Transition::nothing(),
        }
    }
}

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

fn ctrl(modifiers: &ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.logo()
    } else {
        modifiers.ctrl()
    }
}
