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

pub struct SimulationTab<S: AppState> {
    rigid_body_factory: RequestFactory<RigidBodyFactory>,
    brownian_factory: RequestFactory<BrownianParametersFactory>,
    rigid_grid_button: GoStop<S>,
    rigid_helices_button: GoStop<S>,
    scroll: scrollable::State,
    physical_simulation: PhysicalSimulation,
    reset_state: button::State,
}

impl<S: AppState> SimulationTab<S> {
    pub fn new() -> Self {
        let init_brownian = BrownianParametersFactory {
            rate: 0.,
            amplitude: 0.08,
        };
        Self {
            rigid_body_factory: RequestFactory::new(
                FactoryId::RigidBody,
                RigidBodyFactory {
                    volume_exclusion: false,
                    brownian_motion: false,
                    brownian_parameters: init_brownian.clone(),
                },
            ),
            brownian_factory: RequestFactory::new(FactoryId::Brownian, init_brownian),
            rigid_helices_button: GoStop::new(
                String::from("Rigid Helices"),
                Message::RigidHelicesSimulation,
            ),
            rigid_grid_button: GoStop::new(
                String::from("Rigid Grids"),
                Message::RigidGridSimulation,
            ),
            scroll: Default::default(),
            physical_simulation: Default::default(),
            reset_state: Default::default(),
        }
    }

    pub fn view<'a>(&'a mut self, ui_size: UiSize, app_state: &S) -> Element<'a, Message<S>> {
        let sim_state = &app_state.get_simulation_state();
        let grid_active = sim_state.is_none() || sim_state.simulating_grid();
        let roll_active = sim_state.is_none() || sim_state.is_rolling();
        let mut ret = Column::new().spacing(5);
        section!(ret, ui_size, "Simulation (Beta)");
        ret = ret.push(self.physical_simulation.view(
            &ui_size,
            "Roll",
            roll_active,
            sim_state.is_rolling(),
        ));
        ret = ret
            .push(
                self.rigid_grid_button
                    .view(grid_active, sim_state.simulating_grid()),
            )
            .push(Self::helix_btns(
                &mut self.rigid_helices_button,
                &mut self.reset_state,
                app_state,
                ui_size.clone(),
            ));

        let volume_exclusion = self.rigid_body_factory.requestable.volume_exclusion;
        let brownian_motion = self.rigid_body_factory.requestable.brownian_motion;
        subsection!(ret, ui_size, "Parameters for helices simulation");
        for view in self
            .rigid_body_factory
            .view(true, ui_size.main_text())
            .into_iter()
        {
            ret = ret.push(view);
        }
        ret = ret.push(right_checkbox(
            volume_exclusion,
            "Volume exclusion",
            Message::VolumeExclusion,
            ui_size,
        ));
        ret = ret.push(right_checkbox(
            brownian_motion,
            "Unmatched nt jiggling",
            Message::BrownianMotion,
            ui_size,
        ));
        for view in self
            .brownian_factory
            .view(brownian_motion, ui_size.main_text())
            .into_iter()
        {
            ret = ret.push(view);
        }

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    fn helix_btns<'a>(
        go_stop: &'a mut GoStop<S>,
        reset_state: &'a mut button::State,
        app_state: &S,
        ui_size: UiSize,
    ) -> Element<'a, Message<S>> {
        let sim_state = app_state.get_simulation_state();
        if sim_state.is_paused() {
            Row::new()
                .push(go_stop.view(true, false))
                .spacing(3)
                .push(text_btn(reset_state, "Reset", ui_size).on_press(Message::ResetSimulation))
                .into()
        } else {
            let helices_active = sim_state.is_none() || sim_state.simulating_helices();
            go_stop
                .view(helices_active, sim_state.simulating_helices())
                .into()
        }
    }

    pub fn set_volume_exclusion(&mut self, volume_exclusion: bool) {
        self.rigid_body_factory.requestable.volume_exclusion = volume_exclusion;
    }

    pub fn set_brownian_motion(&mut self, brownian_motion: bool) {
        self.rigid_body_factory.requestable.brownian_motion = brownian_motion;
    }

    pub fn make_rigid_body_request(&mut self, request: &mut Option<RigidBodyParametersRequest>) {
        self.rigid_body_factory.make_request(request)
    }

    pub fn update_request(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<RigidBodyParametersRequest>,
    ) {
        self.rigid_body_factory
            .update_request(value_id, value, request)
    }

    pub fn update_brownian(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<RigidBodyParametersRequest>,
    ) {
        let new_brownian = self.brownian_factory.update_value(value_id, value);
        self.rigid_body_factory.requestable.brownian_parameters = new_brownian;
        self.rigid_body_factory.make_request(request)
    }

    pub fn get_physical_simulation_request(&self) -> RollRequest {
        self.physical_simulation.request()
    }

    pub fn leave_tab<R: Requests>(&mut self, requests: Arc<Mutex<R>>, app_state: &S) {
        if app_state.get_simulation_state() == SimulationState::RigidGrid {
            self.request_stop_rigid_body_simulation(requests);
            println!("stop grids");
        } else if app_state.get_simulation_state() == SimulationState::RigidHelices {
            self.request_stop_rigid_body_simulation(requests);
            println!("stop helices");
        }
    }

    fn request_stop_rigid_body_simulation<R: Requests>(&mut self, requests: Arc<Mutex<R>>) {
        let mut request = None;
        self.make_rigid_body_request(&mut request);
        if let Some(request) = request {
            requests
                .lock()
                .unwrap()
                .update_rigid_body_simulation_parameters(request)
        }
    }
}

#[derive(Default)]
struct PhysicalSimulation {
    go_stop_button: button::State,
}

impl PhysicalSimulation {
    fn view<'a, 'b, S: AppState>(
        &'a mut self,
        _ui_size: &'b UiSize,
        name: &'static str,
        active: bool,
        running: bool,
    ) -> Row<'a, Message<S>> {
        let button_str = if running { "Stop" } else { name };
        let mut button = Button::new(&mut self.go_stop_button, Text::new(button_str))
            .style(ButtonColor::red_green(running));
        if active {
            button = button.on_press(Message::SimRequest);
        }
        Row::new().push(button)
    }

    fn request(&self) -> RollRequest {
        RollRequest {
            roll: true,
            springs: false,
            target_helices: None,
        }
    }
}
