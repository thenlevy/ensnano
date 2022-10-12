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

#![allow(dead_code)]

const STARTING_NUMBER_OF_TURN: f64 = 10.;
const ADDITIONAL_NB_TURN: f64 = 2.;

use super::*;

#[derive(Clone)]
pub(super) struct OpenSurfaceTopology {
    nb_section_per_turn: usize,
    nb_turn_per_helix: f64,
    nb_helices: usize,
    prev_section: Vec<usize>,
    next_section: Vec<usize>,
    other_spring_end: Vec<usize>,
    section_with_successor: Vec<usize>,
    section_with_predecessor: Vec<usize>,
    section_with_both_predecessor_and_sucessor: Vec<usize>,
    target: RevolutionSurface,
    section_with_other_spring_end: Vec<usize>,
    surface_descritization: SurfaceDescritization,
}

impl OpenSurfaceTopology {
    pub fn new(desc: RevolutionSurfaceSystemDescriptor) -> Self {
        let nb_helices = desc.target.nb_helix_per_half_section * 2;

        // We want the number of section per turn to be dividable by the number of helices
        let nb_section_per_turn = if desc.nb_section_per_segment % nb_helices == 0 {
            desc.nb_section_per_segment
        } else {
            desc.nb_section_per_segment + nb_helices - (desc.nb_section_per_segment % nb_helices)
        };

        let nb_turn_to_reach_t1 = STARTING_NUMBER_OF_TURN;
        let total_nb_turn_per_helix = nb_turn_to_reach_t1 + ADDITIONAL_NB_TURN;

        let mut target = RevolutionSurface::new(desc.target);

        // Due to how RevolutionSurface::new is implemented, the scaling starts with enough room
        // for one turn
        target.curve_scale_factor *= STARTING_NUMBER_OF_TURN;

        let surface_descritization = SurfaceDescritization {
            nb_section_per_turn,
            nb_helices,
            total_nb_turn_per_helix,
            nb_turn_to_reach_t1,
        };

        let total_nb_section = surface_descritization.total_nb_section();
        let prev_section = (0..total_nb_section)
            .map(|n| surface_descritization.prev_section(n))
            .collect();
        let next_section = (0..total_nb_section)
            .map(|n| surface_descritization.next_section(n))
            .collect();

        let section_with_successor = (0..total_nb_section)
            .filter(|n| *n != surface_descritization.next_section(*n))
            .collect();
        let section_with_predecessor = (0..total_nb_section)
            .filter(|n| *n != surface_descritization.prev_section(*n))
            .collect();

        let section_with_both_predecessor_and_sucessor = (0..total_nb_section)
            .filter(|n| {
                *n != surface_descritization.next_section(*n)
                    && *n != surface_descritization.prev_section(*n)
            })
            .collect();

        let other_spring_end = (0..total_nb_section)
            .map(|n| surface_descritization.other_spring_end(n).unwrap_or(n))
            .collect();
        let section_with_other_spring_end = (0..total_nb_section)
            .filter(|n| surface_descritization.other_spring_end(*n).is_some())
            .collect();

        Self {
            nb_section_per_turn,
            nb_helices,
            nb_turn_per_helix: nb_turn_to_reach_t1,
            prev_section,
            next_section,
            section_with_successor,
            section_with_predecessor,
            section_with_both_predecessor_and_sucessor,
            other_spring_end,
            section_with_other_spring_end,
            target,
            surface_descritization,
        }
    }
}

impl SpringTopology for OpenSurfaceTopology {
    fn nb_balls(&self) -> usize {
        self.surface_descritization.total_nb_section()
    }

    fn balls_with_successor(&self) -> &[usize] {
        &self.section_with_successor
    }

    fn balls_with_predecessor(&self) -> &[usize] {
        &self.section_with_predecessor
    }

    fn balls_involved_in_spring(&self) -> &[usize] {
        &self.section_with_other_spring_end
    }

    fn successor(&self, ball_id: usize) -> usize {
        self.next_section[ball_id]
    }

    fn predecessor(&self, ball_id: usize) -> usize {
        self.prev_section[ball_id]
    }

    fn axis(&self, revolution_angle: f64) -> DVec3 {
        self.target.axis(revolution_angle)
    }

    fn dpos_dtheta(&self, revolution_angle: f64, theta: f64) -> DVec3 {
        self.target.dpos_dtheta(revolution_angle, theta)
    }

    fn rescale_radius(&mut self, scaling_factor: f64) {
        ()
    }

    fn theta_ball_init(&self) -> Vec<f64> {
        let nb_balls = self.nb_balls();

        (0..nb_balls)
            .map(|n| {
                let coordinate = self.surface_descritization.initial_ball_coordinate(n);
                coordinate.section_parameter
            })
            .collect()
    }

    fn rescale_section(&mut self, scaling_factor: f64) {
        ()
    }

    fn other_spring_end(&self, ball_id: usize) -> usize {
        self.other_spring_end[ball_id]
    }

    fn surface_position(&self, revolution_angle: f64, theta: f64) -> DVec3 {
        self.target.position(revolution_angle, theta)
    }

    fn revolution_angle_ball(&self, ball_id: usize) -> f64 {
        self.surface_descritization
            .initial_ball_coordinate(ball_id)
            .revolution_angle
    }

    fn balls_with_predecessor_and_successor(&self) -> &[usize] {
        &self.section_with_both_predecessor_and_sucessor
    }

    fn cloned(&self) -> Box<dyn SpringTopology> {
        Box::new(self.clone())
    }

    fn to_curve_descriptor(&self, thetas: Vec<f64>) -> Vec<CurveDescriptor> {
        let mut ret = Vec::with_capacity(self.nb_helices);

        for i in 0..self.nb_helices {
            let nb_section_per_helix = self.surface_descritization.nb_section_per_helix();
            let nb_section_to_t1 = self.surface_descritization.nb_section_to_t1();
            let ts = (0..nb_section_per_helix)
                .map(|n| n as f64 / nb_section_to_t1 as f64)
                .collect();
            let values = (0..nb_section_per_helix)
                .map(|n| {
                    let section_idx = i * self.surface_descritization.nb_section_per_helix() + n;
                    thetas[section_idx]
                })
                .collect();

            let interpolator = InterpolationDescriptor::PointsValues { points: ts, values };
            ret.push(CurveDescriptor::InterpolatedCurve(
                InterpolatedCurveDescriptor {
                    curve: self.target.curve.clone(),
                    half_turns_count: 0,
                    revolution_radius: 0.,
                    curve_scale_factor: self.target.curve_scale_factor,
                    interpolation: vec![interpolator],
                    chevyshev_smoothening: self.target.junction_smoothening,
                },
            ))
        }

        ret
    }
}

struct BallCoordinate {
    revolution_angle: f64,
    section_parameter: f64,
}

#[derive(Clone)]
struct SurfaceDescritization {
    nb_section_per_turn: usize,
    nb_helices: usize,
    /// The total number of turn done by each helices.
    ///
    /// This value is a bit larger than `nb_turn_to_reach_t1` to ensure that the helices cover all
    /// the surface.
    total_nb_turn_per_helix: f64,
    /// The number of turn to reach the end of the surface in the initial configuration.
    nb_turn_to_reach_t1: f64,
}

impl SurfaceDescritization {
    fn nb_section_per_helix(&self) -> usize {
        (self.nb_section_per_turn as f64 * self.total_nb_turn_per_helix).floor() as usize
    }

    fn nb_section_to_t1(&self) -> usize {
        (self.nb_section_per_turn as f64 * self.nb_turn_to_reach_t1).floor() as usize
    }

    fn total_nb_section(&self) -> usize {
        self.nb_helices * self.nb_section_per_helix()
    }

    fn initial_ball_coordinate(&self, ball_id: usize) -> BallCoordinate {
        let helix_id = ball_id / self.nb_section_per_helix();
        let section_id = ball_id % self.nb_section_per_helix();

        let init_angle = helix_id as f64 * TAU / self.nb_helices as f64;
        let revolution_angle =
            init_angle + section_id as f64 * TAU / self.nb_section_per_turn as f64;

        let section_parameter = section_id as f64 / self.nb_turn_to_reach_t1;

        BallCoordinate {
            revolution_angle,
            section_parameter,
        }
    }

    fn other_spring_end(&self, ball_id: usize) -> Option<usize> {
        let helix_id = ball_id / self.nb_section_per_helix();
        let section_id = ball_id % self.nb_section_per_helix();

        let other_helix_id = (helix_id + 1) % self.nb_helices;
        let other_section_id = section_id + self.nb_section_per_turn / self.nb_helices;

        if other_section_id < self.nb_section_per_helix() {
            Some(other_helix_id * self.nb_section_per_helix() + other_section_id)
        } else {
            None
        }
    }

    fn next_section(&self, ball_id: usize) -> usize {
        let section_id = ball_id % self.nb_section_per_helix();

        let next_section_id = section_id + 1;
        if next_section_id < self.nb_section_per_helix() {
            ball_id + 1
        } else {
            ball_id
        }
    }

    fn prev_section(&self, ball_id: usize) -> usize {
        let section_id = ball_id % self.nb_section_per_helix();
        if section_id >= 1 {
            ball_id - 1
        } else {
            ball_id
        }
    }
}
