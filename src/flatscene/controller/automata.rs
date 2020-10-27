use super::*;
use crate::design::StrandBuilder;

pub struct Transition {
    pub new_state: Option<Box<dyn ControllerState>>,
    pub consequences: Consequence,
}

impl Transition {
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
}

pub trait ControllerState {
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition;

    #[allow(dead_code)]
    fn display(&self) -> String;

    fn transition_from(&self, controller: &Controller) -> ();

    fn transition_to(&self, controller: &Controller) -> ();
}

pub struct NormalState {
    pub mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for NormalState {
    fn display(&self) -> String {
        String::from("Normal state")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Pressed,
                    "Released mouse button in normal mode"
                );
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let pivot_opt = controller.data.borrow().get_click(x, y);
                if let Some(pivot_nucl) = pivot_opt {
                    if let Some(builder) = controller
                        .data
                        .borrow()
                        .get_builder(pivot_nucl)
                        .filter(|_| controller.action_mode == ActionMode::Build)
                    {
                        Transition {
                            new_state: Some(Box::new(Building {
                                mouse_position: self.mouse_position,
                                nucl: pivot_nucl,
                                builder,
                                end: controller.data.borrow().is_strand_end(pivot_nucl)
                            })),
                            consequences: Consequence::Nothing,
                        }
                    } else if controller.action_mode == ActionMode::Cut {
                        Transition {
                            new_state: Some(Box::new(Cutting {
                                nucl: pivot_nucl,
                                mouse_position: self.mouse_position,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    } else {
                        Transition {
                            new_state: Some(Box::new(Translating {
                                mouse_position: self.mouse_position,
                                clicked_position_world: Vec2::new(x, y),
                                pivot_nucl,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(MovingCamera {
                            mouse_position: self.mouse_position,
                            clicked_position_screen: self.mouse_position,
                            pivot_nucl: None,
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_to(&self, controller: &Controller) {
        controller.data.borrow_mut().set_selected_helix(None);
        controller.data.borrow_mut().set_free_end(None);
    }

    fn transition_from(&self, _controller: &Controller) {
        ()
    }
}

pub struct Translating {
    mouse_position: PhysicalPosition<f64>,
    clicked_position_world: Vec2,
    pivot_nucl: Nucl,
}

impl ControllerState for Translating {
    fn display(&self) -> String {
        String::from("Translating state")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in translating mode"
                );
                controller.data.borrow_mut().end_movement();
                Transition {
                    new_state: Some(Box::new(ReleasedPivot {
                        mouse_position: self.mouse_position,
                        pivot_nucl: self.pivot_nucl,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (mouse_dx, mouse_dy) = {
                    let (x, y) = controller
                        .camera
                        .borrow()
                        .screen_to_world(position.x as f32, position.y as f32);
                    (
                        x - self.clicked_position_world.x,
                        y - self.clicked_position_world.y,
                    )
                };
                controller
                    .data
                    .borrow_mut()
                    .translate_helix(Vec2::new(mouse_dx, mouse_dy));
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_from(&self, controller: &Controller) {
        controller.data.borrow_mut().end_movement()
    }

    fn transition_to(&self, controller: &Controller) {
        controller
            .data
            .borrow_mut()
            .set_selected_helix(Some(self.pivot_nucl.helix))
    }
}

pub struct MovingCamera {
    mouse_position: PhysicalPosition<f64>,
    clicked_position_screen: PhysicalPosition<f64>,
    pivot_nucl: Option<Nucl>,
}

impl ControllerState for MovingCamera {
    fn display(&self) -> String {
        String::from("Moving camera")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in translating mode"
                );
                if let Some(pivot_nucl) = self.pivot_nucl {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: self.mouse_position,
                            pivot_nucl,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(NormalState {
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let mouse_dx = (position.x as f32 - self.clicked_position_screen.x as f32)
                    / controller.area_size.width as f32;
                let mouse_dy = (position.y as f32 - self.clicked_position_screen.y as f32)
                    / controller.area_size.height as f32;
                controller
                    .camera
                    .borrow_mut()
                    .process_mouse(mouse_dx, mouse_dy);
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }

    fn transition_from(&self, controller: &Controller) {
        controller.camera.borrow_mut().end_movement();
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }
}

pub struct ReleasedPivot {
    mouse_position: PhysicalPosition<f64>,
    pivot_nucl: Nucl,
}

impl ControllerState for ReleasedPivot {
    fn transition_to(&self, controller: &Controller) {
        controller
            .data
            .borrow_mut()
            .set_selected_helix(Some(self.pivot_nucl.helix));
    }

    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("Released Pivot")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Pressed,
                    "Released mouse button in ReleasedPivot state"
                );
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let pivot_opt = controller.data.borrow().get_click(x, y);
                if let Some(pivot) = pivot_opt {
                    Transition {
                        new_state: Some(Box::new(Translating {
                            pivot_nucl: pivot,
                            clicked_position_world: Vec2::new(x, y),
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition {
                        new_state: Some(Box::new(LeavingPivot {
                            pivot_nucl: self.pivot_nucl,
                            clicked_position_screen: self.mouse_position,
                            mouse_position: self.mouse_position,
                        })),
                        consequences: Consequence::Nothing,
                    }
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Pressed,
                    "Released right mouse button in ReleasedPivot state"
                );
                let pivot_coordinates = controller
                    .data
                    .borrow()
                    .get_pivot_position(self.pivot_nucl.helix, self.pivot_nucl.position)
                    .expect("pivot coordinates");
                Transition {
                    new_state: Some(Box::new(Rotating {
                        pivot_nucl: self.pivot_nucl,
                        pivot_coordinates,
                        clicked_position_screen: self.mouse_position,
                        mouse_position: self.mouse_position,
                        button: MouseButton::Right,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// This state in entered when use user has
pub struct LeavingPivot {
    pivot_nucl: Nucl,
    clicked_position_screen: PhysicalPosition<f64>,
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for LeavingPivot {
    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("Leaving Pivot")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in LeavingPivot state"
                );
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Pressed,
                    "Released right mouse button in ReleasedPivot state"
                );
                let pivot_coordinates = controller
                    .data
                    .borrow()
                    .get_pivot_position(self.pivot_nucl.helix, self.pivot_nucl.position)
                    .expect("pivot coordinates");
                Transition {
                    new_state: Some(Box::new(Rotating {
                        pivot_nucl: self.pivot_nucl,
                        pivot_coordinates,
                        clicked_position_screen: self.mouse_position,
                        mouse_position: self.mouse_position,
                        button: MouseButton::Right,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                if position_difference(self.clicked_position_screen, self.mouse_position) > 5. {
                    Transition {
                        new_state: Some(Box::new(MovingCamera {
                            pivot_nucl: Some(self.pivot_nucl),
                            mouse_position: self.mouse_position,
                            clicked_position_screen: self.clicked_position_screen,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::nothing()
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

pub struct Rotating {
    pivot_nucl: Nucl,
    pivot_coordinates: Vec2,
    clicked_position_screen: PhysicalPosition<f64>,
    button: MouseButton,
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for Rotating {
    fn transition_to(&self, controller: &Controller) {
        controller
            .data
            .borrow_mut()
            .set_selected_helix(Some(self.pivot_nucl.helix))
    }

    fn transition_from(&self, controller: &Controller) {
        controller.data.borrow_mut().end_movement()
    }

    fn display(&self) -> String {
        String::from("Rotating")
    }
    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput { button, state, .. } if *button == self.button => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Rotating state"
                );
                Transition {
                    new_state: Some(Box::new(ReleasedPivot {
                        mouse_position: self.mouse_position,
                        pivot_nucl: self.pivot_nucl,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let angle = {
                    let (x, y) = controller
                        .camera
                        .borrow()
                        .screen_to_world(position.x as f32, position.y as f32);
                    let (old_x, old_y) = controller.camera.borrow().screen_to_world(
                        self.clicked_position_screen.x as f32,
                        self.clicked_position_screen.y as f32,
                    );
                    (y - self.pivot_coordinates.y).atan2(x - self.pivot_coordinates.x)
                        - (old_y - self.pivot_coordinates.y).atan2(old_x - self.pivot_coordinates.x)
                };
                controller
                    .data
                    .borrow_mut()
                    .rotate_helix(self.pivot_coordinates, angle);
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct Building {
    mouse_position: PhysicalPosition<f64>,
    builder: StrandBuilder,
    nucl: Nucl,
    end: bool,
}

impl ControllerState for Building {
    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("Building")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Building state"
                );
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Nothing,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl = controller.data.borrow().get_click(x, y);
                match nucl {
                    Some(Nucl {
                        helix,
                        position,
                        forward,
                    }) if helix == self.nucl.helix && forward == self.nucl.forward => {
                        self.builder.move_to(position);
                        controller.data.borrow_mut().notify_update();
                        Transition::consequence(Consequence::FreeEnd(None))
                    }
                    Some(nucl) if controller.data.borrow().can_cross_to(self.nucl, nucl) => {
                        self.builder.reset();
                        controller.data.borrow_mut().notify_update();
                        Transition {
                            new_state: Some(Box::new(Crossing {
                                mouse_position: self.mouse_position,
                                from: self.nucl,
                                to: nucl,
                                builder: self.builder.clone(),
                            })),
                            consequences: Consequence::FreeEnd(Some(FreeEnd {
                                strand_id: self.builder.get_strand_id(),
                                point: Vec2::new(x, y)
                            }).filter(|_| self.end))
                        }
                    }
                    _ => {
                            self.builder.reset();
                            controller.data.borrow_mut().notify_update();
                            Transition::consequence(Consequence::FreeEnd(Some(FreeEnd {
                            strand_id: self.builder.get_strand_id(),
                            point: Vec2::new(x, y)
                        }).filter(|_| self.end)))
                    }
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

pub struct Crossing {
    mouse_position: PhysicalPosition<f64>,
    from: Nucl,
    to: Nucl,
    builder: StrandBuilder,
}

impl ControllerState for Crossing {
    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("Crossing")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Crossing state"
                );
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Xover(self.from, self.to),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl = controller.data.borrow().get_click(x, y);
                if nucl != Some(self.to) {
                    Transition {
                        new_state: Some(Box::new(Building {
                            mouse_position: self.mouse_position,
                            builder: self.builder.clone(),
                            nucl: self.from,
                            end: true,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::consequence(Consequence::FreeEnd(Some(FreeEnd {
                        strand_id: self.builder.get_strand_id(),
                        point: Vec2::new(x, y)
                    })))
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct Cutting {
    mouse_position: PhysicalPosition<f64>,
    nucl: Nucl,
}

impl ControllerState for Cutting {
    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("Cutting")
    }

    fn input(
        &mut self,
        event: &WindowEvent,
        position: PhysicalPosition<f64>,
        controller: &Controller,
    ) -> Transition {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl = controller.data.borrow().get_click(x, y);
                let consequences = if nucl == Some(self.nucl) {
                    Consequence::Cut(self.nucl)
                } else {
                    Consequence::Nothing
                };
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences,
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

fn position_difference(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}
