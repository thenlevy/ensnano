use super::super::data::ClickResult;
use super::super::view::CircleInstance;
use super::*;
use crate::design::StrandBuilder;

const WHEEL_RADIUS: f32 = 1.5;
use crate::consts::*;

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
                modifiers,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Pressed,
                    "Released mouse button in normal mode"
                );*/
                if *state == ElementState::Released {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result = controller.data.borrow().get_click(x, y, &controller.camera);
                match click_result {
                    ClickResult::Nucl(nucl) => {
                        if controller.action_mode == ActionMode::Cut {
                            Transition {
                                new_state: Some(Box::new(Cutting {
                                    nucl,
                                    mouse_position: self.mouse_position,
                                    whole_strand: modifiers.shift(),
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            let stick = if let ActionMode::Build(b) = controller.action_mode {
                                b
                            } else {
                                false
                            };
                            if let Some(builder) = controller.data.borrow().get_builder(nucl, stick)
                            {
                                if builder.created_de_novo() {
                                    Transition {
                                        new_state: Some(Box::new(Building {
                                            mouse_position: self.mouse_position,
                                            nucl,
                                            builder,
                                        })),
                                        consequences: Consequence::Nothing,
                                    }
                                } else {
                                    Transition {
                                        new_state: Some(Box::new(InitBuilding {
                                            mouse_position: self.mouse_position,
                                            nucl,
                                            builder,
                                            end: controller.data.borrow().is_strand_end(nucl),
                                        })),
                                        consequences: Consequence::Nothing,
                                    }
                                }
                            } else if controller.data.borrow().has_nucl(nucl)
                                && controller.data.borrow().is_xover_end(&nucl).is_none()
                            {
                                Transition {
                                    new_state: Some(Box::new(InitCutting {
                                        mouse_position: self.mouse_position,
                                        nucl,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            } else {
                                Transition {
                                    new_state: Some(Box::new(MovingCamera {
                                        mouse_position: self.mouse_position,
                                        clicked_position_screen: self.mouse_position,
                                        translation_pivot: None,
                                        rotation_pivot: None,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            }
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot } if ctrl(modifiers) => {
                        Transition {
                            new_state: Some(Box::new(FlipVisibility {
                                mouse_position: self.mouse_position,
                                helix: translation_pivot.helix,
                                apply_to_other: modifiers.alt(),
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot } if modifiers.alt() => {
                        Transition {
                            new_state: Some(Box::new(FlipGroup {
                                mouse_position: self.mouse_position,
                                helix: translation_pivot.helix,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot } => {
                        if controller.action_mode == ActionMode::Cut {
                            Transition {
                                new_state: Some(Box::new(RmHelix {
                                    mouse_position: self.mouse_position,
                                    helix: translation_pivot.helix,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            let original_pivot_position = controller
                                .data
                                .borrow()
                                .get_pivot_position(
                                    translation_pivot.helix,
                                    translation_pivot.position,
                                )
                                .unwrap();
                            let (clicked_x, clicked_y) =
                                controller.camera.borrow().screen_to_world(
                                    self.mouse_position.x as f32,
                                    self.mouse_position.y as f32,
                                );
                            let world_delta =
                                Vec2::new(clicked_x, clicked_y) - original_pivot_position;
                            Transition {
                                new_state: Some(Box::new(Translating {
                                    mouse_position: self.mouse_position,
                                    world_delta,
                                    translation_pivot,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        }
                    }
                    ClickResult::Nothing => Transition {
                        new_state: Some(Box::new(MovingCamera {
                            mouse_position: self.mouse_position,
                            clicked_position_screen: self.mouse_position,
                            translation_pivot: None,
                            rotation_pivot: None,
                        })),
                        consequences: Consequence::Nothing,
                    },
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let pivot_opt = if let ClickResult::Nucl(nucl) =
                    controller.data.borrow().get_click(x, y, &controller.camera)
                {
                    Some(nucl)
                } else {
                    None
                };
                Transition::consequence(Consequence::NewCandidate(pivot_opt))
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
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
    world_delta: Vec2,
    translation_pivot: Nucl,
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
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                controller.data.borrow_mut().end_movement();
                if let Some(rotation_pivot) = controller
                    .data
                    .borrow()
                    .get_rotation_pivot(self.translation_pivot.helix, &controller.camera)
                {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: self.mouse_position,
                            translation_pivot: self.translation_pivot,
                            rotation_pivot,
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
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(position.x as f32, position.y as f32);
                /*controller
                .data
                .borrow_mut()
                .translate_helix(Vec2::new(mouse_dx, mouse_dy));*/
                controller
                    .data
                    .borrow_mut()
                    .snap_helix(self.translation_pivot, Vec2::new(x, y) - self.world_delta);
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
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
            .set_selected_helix(Some(self.translation_pivot.helix))
    }
}

pub struct MovingCamera {
    mouse_position: PhysicalPosition<f64>,
    clicked_position_screen: PhysicalPosition<f64>,
    translation_pivot: Option<Nucl>,
    rotation_pivot: Option<Vec2>,
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in translating mode"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                if let Some(translation_pivot) = self.translation_pivot {
                    Transition {
                        new_state: Some(Box::new(ReleasedPivot {
                            mouse_position: self.mouse_position,
                            translation_pivot,
                            rotation_pivot: self.rotation_pivot.unwrap(),
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
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
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
    translation_pivot: Nucl,
    rotation_pivot: Vec2,
}

impl ControllerState for ReleasedPivot {
    fn transition_to(&self, controller: &Controller) {
        controller
            .data
            .borrow_mut()
            .set_selected_helix(Some(self.translation_pivot.helix));
        controller
            .view
            .borrow_mut()
            .set_wheel(Some(CircleInstance::new(
                self.rotation_pivot,
                WHEEL_RADIUS,
                -1,
                CIRCLE2D_GREY,
            )));
    }

    fn transition_from(&self, controller: &Controller) {
        controller.view.borrow_mut().set_wheel(None);
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
                modifiers,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Pressed,
                    "Released mouse button in ReleasedPivot state"
                );*/
                if *state == ElementState::Released {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result = controller.data.borrow().get_click(x, y, &controller.camera);
                match click_result {
                    ClickResult::CircleWidget { translation_pivot } if ctrl(modifiers) => {
                        Transition {
                            new_state: Some(Box::new(FlipVisibility {
                                mouse_position: self.mouse_position,
                                helix: translation_pivot.helix,
                                apply_to_other: modifiers.alt(),
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::Nucl(nucl) => {
                        if controller.action_mode == ActionMode::Cut {
                            Transition {
                                new_state: Some(Box::new(Cutting {
                                    nucl,
                                    mouse_position: self.mouse_position,
                                    whole_strand: modifiers.shift(),
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            let stick = if let ActionMode::Build(b) = controller.action_mode {
                                b
                            } else {
                                false
                            };
                            if let Some(builder) = controller.data.borrow().get_builder(nucl, stick)
                            {
                                Transition {
                                    new_state: Some(Box::new(InitBuilding {
                                        mouse_position: self.mouse_position,
                                        nucl,
                                        builder,
                                        end: controller.data.borrow().is_strand_end(nucl),
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            } else {
                                Transition {
                                    new_state: Some(Box::new(MovingCamera {
                                        mouse_position: self.mouse_position,
                                        clicked_position_screen: self.mouse_position,
                                        translation_pivot: None,
                                        rotation_pivot: None,
                                    })),
                                    consequences: Consequence::Nothing,
                                }
                            }
                        }
                    }
                    ClickResult::CircleWidget { translation_pivot } => {
                        let original_pivot_position = controller
                            .data
                            .borrow()
                            .get_pivot_position(translation_pivot.helix, translation_pivot.position)
                            .unwrap();
                        let (clicked_x, clicked_y) = controller.camera.borrow().screen_to_world(
                            self.mouse_position.x as f32,
                            self.mouse_position.y as f32,
                        );
                        let world_delta = Vec2::new(clicked_x, clicked_y) - original_pivot_position;
                        Transition {
                            new_state: Some(Box::new(Translating {
                                translation_pivot,
                                world_delta,
                                mouse_position: self.mouse_position,
                            })),
                            consequences: Consequence::Nothing,
                        }
                    }
                    ClickResult::Nothing => Transition {
                        new_state: Some(Box::new(LeavingPivot {
                            clicked_position_screen: self.mouse_position,
                            mouse_position: self.mouse_position,
                            translation_pivot: self.translation_pivot,
                            rotation_pivot: self.rotation_pivot,
                        })),
                        consequences: Consequence::Nothing,
                    },
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                /*assert!(
                    *state == ElementState::Pressed,
                    "Released right mouse button in ReleasedPivot state"
                );*/
                if *state == ElementState::Released {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(Rotating {
                        translation_pivot: self.translation_pivot,
                        rotation_pivot: self.rotation_pivot,
                        clicked_position_screen: self.mouse_position,
                        mouse_position: self.mouse_position,
                        button: MouseButton::Right,
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
                let pivot_opt = if let ClickResult::Nucl(nucl) =
                    controller.data.borrow().get_click(x, y, &controller.camera)
                {
                    Some(nucl)
                } else {
                    None
                };
                Transition::consequence(Consequence::NewCandidate(pivot_opt))
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

/// This state in entered when use user has clicked after realising a pivot. If the user moves
/// their mouse, go in moving camera mode without unselecting the helix. If the user release their
/// click without moving their mouse, clear selection
pub struct LeavingPivot {
    translation_pivot: Nucl,
    rotation_pivot: Vec2,
    clicked_position_screen: PhysicalPosition<f64>,
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for LeavingPivot {
    fn transition_to(&self, controller: &Controller) {
        controller
            .view
            .borrow_mut()
            .set_wheel(Some(CircleInstance::new(
                self.rotation_pivot,
                WHEEL_RADIUS,
                -1,
                CIRCLE2D_GREY,
            )));
    }

    fn transition_from(&self, controller: &Controller) {
        controller.view.borrow_mut().set_wheel(None);
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
                /*
                assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in LeavingPivot state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
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
                /*assert!(
                    *state == ElementState::Pressed,
                    "Released right mouse button in ReleasedPivot state"
                );*/
                if *state == ElementState::Released {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(Rotating {
                        translation_pivot: self.translation_pivot,
                        rotation_pivot: self.rotation_pivot,
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
                            translation_pivot: Some(self.translation_pivot),
                            rotation_pivot: Some(self.rotation_pivot),
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
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

pub struct Rotating {
    translation_pivot: Nucl,
    rotation_pivot: Vec2,
    clicked_position_screen: PhysicalPosition<f64>,
    button: MouseButton,
    mouse_position: PhysicalPosition<f64>,
}

impl ControllerState for Rotating {
    fn transition_to(&self, controller: &Controller) {
        controller
            .data
            .borrow_mut()
            .set_selected_helix(Some(self.translation_pivot.helix));
        controller
            .view
            .borrow_mut()
            .set_wheel(Some(CircleInstance::new(
                self.rotation_pivot,
                WHEEL_RADIUS,
                -1,
                CIRCLE2D_GREY,
            )));
    }

    fn transition_from(&self, controller: &Controller) {
        controller.data.borrow_mut().end_movement();
        controller.view.borrow_mut().set_wheel(None);
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Rotating state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(ReleasedPivot {
                        translation_pivot: self.translation_pivot,
                        rotation_pivot: self.rotation_pivot,
                        mouse_position: self.mouse_position,
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
                    (y - self.rotation_pivot.y).atan2(x - self.rotation_pivot.x)
                        - (old_y - self.rotation_pivot.y).atan2(old_x - self.rotation_pivot.x)
                };
                controller
                    .data
                    .borrow_mut()
                    .rotate_helix(self.rotation_pivot, angle);
                controller
                    .view
                    .borrow_mut()
                    .set_wheel(Some(CircleInstance::new(
                        self.rotation_pivot,
                        WHEEL_RADIUS,
                        -1,
                        CIRCLE2D_GREY,
                    )));
                Transition::nothing()
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct InitCutting {
    mouse_position: PhysicalPosition<f64>,
    nucl: Nucl,
}

impl ControllerState for InitCutting {
    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("Init Cutting")
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Init Building state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Cut(self.nucl),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result = controller.data.borrow().get_click(x, y, &controller.camera);
                match click_result {
                    ClickResult::Nucl(nucl2) if nucl2 == self.nucl => Transition::nothing(),
                    _ => {
                        let strand_id = controller.data.borrow().get_strand_id(self.nucl).unwrap();
                        Transition {
                            new_state: Some(Box::new(MovingFreeEnd {
                                mouse_position: self.mouse_position,
                                from: self.nucl,
                                prime3: true,
                                strand_id,
                            })),
                            consequences: Consequence::CutFreeEnd(
                                self.nucl,
                                Some(FreeEnd {
                                    strand_id,
                                    point: Vec2::new(x, y),
                                    prime3: true,
                                }),
                            ),
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct InitBuilding {
    mouse_position: PhysicalPosition<f64>,
    builder: StrandBuilder,
    nucl: Nucl,
    end: Option<bool>,
}

impl ControllerState for InitBuilding {
    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("Init Building")
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Init Building state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Cut(self.nucl),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result = controller.data.borrow().get_click(x, y, &controller.camera);
                match click_result {
                    ClickResult::Nucl(Nucl {
                        helix,
                        position,
                        forward,
                    }) if helix == self.nucl.helix && forward == self.nucl.forward => {
                        if position != self.nucl.position {
                            self.builder.move_to(position);
                            controller.data.borrow_mut().notify_update();
                            Transition {
                                new_state: Some(Box::new(Building {
                                    mouse_position: self.mouse_position,
                                    builder: self.builder.clone(),
                                    nucl: self.nucl,
                                })),
                                consequences: Consequence::Nothing,
                            }
                        } else {
                            Transition::nothing()
                        }
                    }
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().can_cross_to(self.nucl, nucl) =>
                    {
                        self.builder.reset();
                        controller.data.borrow_mut().notify_update();
                        Transition {
                            new_state: Some(Box::new(Crossing {
                                mouse_position: self.mouse_position,
                                from: self.nucl,
                                to: nucl,
                                strand_id: self.builder.get_strand_id(),
                                from3prime: self.end.expect("from3prime"),
                                cut: false,
                            })),
                            consequences: Consequence::FreeEnd(self.end.map(|b| FreeEnd {
                                strand_id: self.builder.get_strand_id(),
                                point: Vec2::new(x, y),
                                prime3: b,
                            })),
                        }
                    }
                    _ => {
                        if let Some(prime3) = self.end {
                            Transition {
                                new_state: Some(Box::new(MovingFreeEnd {
                                    mouse_position: self.mouse_position,
                                    from: self.nucl,
                                    prime3,
                                    strand_id: self.builder.get_strand_id(),
                                })),
                                consequences: Consequence::FreeEnd(Some(FreeEnd {
                                    strand_id: self.builder.get_strand_id(),
                                    point: Vec2::new(x, y),
                                    prime3,
                                })),
                            }
                        } else {
                            let prime3 = controller
                                .data
                                .borrow()
                                .is_xover_end(&self.nucl)
                                .expect("prime 3");
                            Transition {
                                new_state: Some(Box::new(MovingFreeEnd {
                                    mouse_position: self.mouse_position,
                                    from: self.nucl,
                                    prime3,
                                    strand_id: self.builder.get_strand_id(),
                                })),
                                consequences: Consequence::CutFreeEnd(
                                    self.nucl,
                                    Some(FreeEnd {
                                        strand_id: self.builder.get_strand_id(),
                                        point: Vec2::new(x, y),
                                        prime3,
                                    }),
                                ),
                            }
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct MovingFreeEnd {
    mouse_position: PhysicalPosition<f64>,
    from: Nucl,
    strand_id: usize,
    prime3: bool,
}

impl ControllerState for MovingFreeEnd {
    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("Init Building")
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Moving Free End state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::FreeEnd(None),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let click_result = controller.data.borrow().get_click(x, y, &controller.camera);
                match click_result {
                    ClickResult::Nucl(nucl) if nucl == self.from => Transition::nothing(),
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().can_cross_to(self.from, nucl) =>
                    {
                        controller.data.borrow_mut().notify_update();
                        Transition {
                            new_state: Some(Box::new(Crossing {
                                mouse_position: self.mouse_position,
                                from: self.from,
                                to: nucl,
                                from3prime: self.prime3,
                                strand_id: self.strand_id,
                                cut: false,
                            })),
                            consequences: Consequence::FreeEnd(Some(FreeEnd {
                                strand_id: self.strand_id,
                                point: Vec2::new(x, y),
                                prime3: self.prime3,
                            })),
                        }
                    }
                    ClickResult::Nucl(nucl)
                        if controller.data.borrow().can_cut_cross_to(self.from, nucl) =>
                    {
                        controller.data.borrow_mut().notify_update();
                        Transition {
                            new_state: Some(Box::new(Crossing {
                                mouse_position: self.mouse_position,
                                from: self.from,
                                to: nucl,
                                from3prime: self.prime3,
                                strand_id: self.strand_id,
                                cut: true,
                            })),
                            consequences: Consequence::FreeEnd(Some(FreeEnd {
                                strand_id: self.strand_id,
                                point: Vec2::new(x, y),
                                prime3: self.prime3,
                            })),
                        }
                    }
                    _ => Transition::consequence(Consequence::FreeEnd(Some(FreeEnd {
                        strand_id: self.strand_id,
                        point: Vec2::new(x, y),
                        prime3: self.prime3,
                    }))),
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Building state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: Consequence::Built(Box::new(self.builder.clone())),
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl =
                    controller
                        .data
                        .borrow()
                        .get_click_unbounded_helix(x, y, self.nucl.helix);
                match nucl {
                    Nucl {
                        helix, position, ..
                    } if helix == self.nucl.helix => {
                        self.builder.move_to(position);
                        controller.data.borrow_mut().notify_update();
                        Transition::consequence(Consequence::FreeEnd(None))
                    }
                    _ => Transition::nothing(),
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
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
    from3prime: bool,
    strand_id: usize,
    cut: bool,
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Crossing state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                Transition {
                    new_state: Some(Box::new(NormalState {
                        mouse_position: self.mouse_position,
                    })),
                    consequences: if self.cut {
                        Consequence::CutCross(self.from, self.to)
                    } else {
                        Consequence::Xover(self.from, self.to)
                    },
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.mouse_position = position;
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl = controller.data.borrow().get_click(x, y, &controller.camera);
                if nucl != ClickResult::Nucl(self.to) {
                    Transition {
                        new_state: Some(Box::new(MovingFreeEnd {
                            mouse_position: self.mouse_position,
                            from: self.from,
                            prime3: self.from3prime,
                            strand_id: self.strand_id,
                        })),
                        consequences: Consequence::Nothing,
                    }
                } else {
                    Transition::consequence(Consequence::FreeEnd(Some(FreeEnd {
                        strand_id: self.strand_id,
                        point: Vec2::new(x, y),
                        prime3: self.from3prime,
                    })))
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                controller.process_keyboard(event);
                Transition::nothing()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct Cutting {
    mouse_position: PhysicalPosition<f64>,
    nucl: Nucl,
    whole_strand: bool,
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl = controller.data.borrow().get_click(x, y, &controller.camera);
                let consequences = if nucl == ClickResult::Nucl(self.nucl) {
                    if self.whole_strand {
                        Consequence::RmStrand(self.nucl)
                    } else {
                        Consequence::Cut(self.nucl)
                    }
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
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct RmHelix {
    mouse_position: PhysicalPosition<f64>,
    helix: usize,
}

impl ControllerState for RmHelix {
    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("RmHelix")
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl = controller.data.borrow().get_click(x, y, &controller.camera);
                let consequences = if let ClickResult::CircleWidget { translation_pivot } = nucl {
                    if translation_pivot.helix == self.helix {
                        Consequence::RmHelix(self.helix)
                    } else {
                        Consequence::Nothing
                    }
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
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct FlipGroup {
    mouse_position: PhysicalPosition<f64>,
    helix: usize,
}

impl ControllerState for FlipGroup {
    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("FlipGroup")
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl = controller.data.borrow().get_click(x, y, &controller.camera);
                let consequences = if let ClickResult::CircleWidget { translation_pivot } = nucl {
                    if translation_pivot.helix == self.helix {
                        Consequence::FlipGroup(self.helix)
                    } else {
                        Consequence::Nothing
                    }
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
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
            _ => Transition::nothing(),
        }
    }
}

struct FlipVisibility {
    mouse_position: PhysicalPosition<f64>,
    helix: usize,
    apply_to_other: bool,
}

impl ControllerState for FlipVisibility {
    fn transition_from(&self, _controller: &Controller) {
        ()
    }

    fn transition_to(&self, _controller: &Controller) {
        ()
    }

    fn display(&self) -> String {
        String::from("RmHelix")
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
                /*assert!(
                    *state == ElementState::Released,
                    "Pressed mouse button in Cutting state"
                );*/
                if *state == ElementState::Pressed {
                    return Transition::nothing();
                }
                let (x, y) = controller
                    .camera
                    .borrow()
                    .screen_to_world(self.mouse_position.x as f32, self.mouse_position.y as f32);
                let nucl = controller.data.borrow().get_click(x, y, &controller.camera);
                let consequences = if let ClickResult::CircleWidget { translation_pivot } = nucl {
                    if translation_pivot.helix == self.helix {
                        Consequence::FlipVisibility(self.helix, self.apply_to_other)
                    } else {
                        Consequence::Nothing
                    }
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
            WindowEvent::MouseWheel { delta, .. } => {
                controller
                    .camera
                    .borrow_mut()
                    .process_scroll(delta, self.mouse_position);
                Transition::nothing()
            }
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
