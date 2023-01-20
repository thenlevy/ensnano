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
use crate::*;

/// A structure that can map time points to nucleotide indices.
#[derive(Clone, Debug)]
pub struct HelixTimeMap {
    nucl_time: Arc<Vec<f64>>,
    nb_negative_nucl: usize,
    square_per_time: f64,
}

const MIN_WIDTH: f64 = 1. / 3.;

impl HelixTimeMap {
    pub fn x_to_nucl_conversion(&self, x: f64) -> f64 {
        if self.nucl_time.len() < 2 {
            x / self.square_per_time
        } else {
            /*
            let time_per_x = 1. / self.square_per_time;

            let time = time_per_x * x;

            if time < *self.nucl_time.first().unwrap() {
                let remainder_time = time - *self.nucl_time.first().unwrap();
                self.nb_negative_nucl as f64 + remainder_time / time_per_x
            } else if time > *self.nucl_time.last().unwrap() {
                let remainder_time = time - *self.nucl_time.last().unwrap();
                (self.nucl_time.len() - self.nb_negative_nucl) as f64 + remainder_time / time_per_x
            } else {
                let n = self.find_time(time);
                let remainder_time = time - self.nucl_time[n];
                n as f64 - self.nb_negative_nucl as f64 + remainder_time / time_per_x
            }
            */
            let time = x / self.square_per_time;
            let nucl_idx = self.find_time(time);
            let t_left = self.time_nucl(nucl_idx);
            let length_of_square = self.normalized_length_of_square(nucl_idx);
            let fract = (time - t_left) / length_of_square;
            nucl_idx as f64 + fract
        }
    }

    pub fn nucl_to_x_convertion(&self, n: isize) -> f64 {
        self.time_nucl(n) * self.square_per_time
    }

    /// Same as nucl_to_x_convertion but for "non integer nucl positions"
    pub fn x_conversion(&self, x: f64) -> f64 {
        let nucl_idx = x.floor() as isize;
        let left = self.time_nucl(nucl_idx) * self.square_per_time;
        let length_of_square = self.normalized_length_of_square(nucl_idx);
        let fract = length_of_square * x.fract();
        left + fract
    }

    fn find_time(&self, time: f64) -> isize {
        let mut a = -1;
        while self.time_nucl(a) > time {
            a *= 2;
        }
        let mut b = self.nucl_time.len() as isize - 1;
        while self.time_nucl(b) < time {
            b *= 2;
        }
        while b - a > 1 {
            let c = (a + b) / 2;
            if self.time_nucl(c) < time {
                a = c;
            } else {
                b = c;
            }
        }
        a
    }

    fn time_nucl(&self, nucl_idx: isize) -> f64 {
        let nucl_idx = nucl_idx + self.nb_negative_nucl as isize;
        if nucl_idx < 0 {
            let first_delta = self.nucl_time[1] - self.nucl_time[0];
            first_delta * nucl_idx as f64 + self.nucl_time[0]
        } else if nucl_idx as usize >= self.nucl_time.len() {
            let n = self.nucl_time.len();
            let last_delta = self.nucl_time[n - 1] - self.nucl_time[n - 2];
            self.nucl_time[n - 1] + (nucl_idx as f64 - (n - 1) as f64) * last_delta
        } else {
            self.nucl_time[nucl_idx as usize]
        }
    }

    fn normalized_length_of_square(&self, nucl_idx: isize) -> f64 {
        self.square_per_time * (self.time_nucl(nucl_idx + 1) - self.time_nucl(nucl_idx))
    }

    fn square_per_time_for_time_map(nucl_time: &[f64]) -> f64 {
        use ordered_float::OrderedFloat;
        if nucl_time.len() < 3 {
            1.
        } else {
            let t_tot = nucl_time.last().unwrap() - nucl_time.first().unwrap();

            let avg_time_per_nucl = t_tot / nucl_time.len() as f64; // sec/sq
            let smallest_delta = nucl_time
                .iter()
                .zip(nucl_time.iter().skip(1))
                .map(|(t0, t1)| OrderedFloat::from(*t1 - *t0))
                .min()
                .unwrap(); // sec

            let smallest_width_if_avg = smallest_delta / avg_time_per_nucl; // sq

            if smallest_width_if_avg >= MIN_WIDTH.into() {
                1. / avg_time_per_nucl
            } else {
                MIN_WIDTH / f64::from(smallest_delta)
            }
        }
    }
}

#[derive(Clone, Debug)]
enum AbscissaConverter_ {
    Real(HelixTimeMap),
    Fake(f64),
}

#[derive(Clone, Debug)]
pub struct AbscissaConverter(AbscissaConverter_);

impl Default for AbscissaConverter {
    fn default() -> Self {
        Self(AbscissaConverter_::Fake(1.))
    }
}

impl AbscissaConverter {
    pub fn x_to_nucl_conversion(&self, x: f64) -> f64 {
        match &self.0 {
            AbscissaConverter_::Real(time_map) => time_map.x_to_nucl_conversion(x),
            AbscissaConverter_::Fake(normalisation_time) => x / normalisation_time,
        }
    }

    pub fn nucl_to_x_convertion(&self, n: isize) -> f64 {
        match &self.0 {
            AbscissaConverter_::Real(time_map) => time_map.nucl_to_x_convertion(n),
            AbscissaConverter_::Fake(normalisation_time) => n as f64 / normalisation_time,
        }
    }

    pub fn x_conversion(&self, x: f64) -> f64 {
        match &self.0 {
            AbscissaConverter_::Real(time_map) => time_map.x_conversion(x),
            AbscissaConverter_::Fake(normalisation_time) => x / normalisation_time,
        }
    }

    pub(super) fn from_single_map(time_points: Arc<Vec<f64>>) -> Option<Self> {
        if time_points.len() < 2 {
            return None;
        }

        let square_per_time = HelixTimeMap::square_per_time_for_time_map(time_points.as_slice());
        log::info!("square per time = {square_per_time}");
        Some(Self(AbscissaConverter_::Real(HelixTimeMap {
            square_per_time,
            nb_negative_nucl: 0,
            nucl_time: time_points,
        })))
    }
}

#[derive(Debug)]
pub(crate) struct PathTimeMaps {
    time_maps: BTreeMap<usize, HelixTimeMap>,
    length_normalisation: f64,
}

#[derive(Debug)]
pub(crate) struct RevolutionCurveTimeMaps {
    time_maps: BTreeMap<usize, HelixTimeMap>,
    length_normalisation: f64,
}

impl RevolutionCurveTimeMaps {
    pub fn new(curve: &CurveDescriptor2D, helices: &[(usize, &Helix)]) -> Self {
        let mut time_maps = BTreeMap::new();

        let mut square_per_time: f64 = 1.;

        for (_, h) in helices
            .iter()
            .filter(|(_, h)| h.get_revolution_curve_desc() == Some(curve))
        {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                let mut positions = vec![0];
                for next_left in h
                    .additonal_isometries
                    .iter()
                    .map(|s| s.left)
                    .filter(|left| *left < curve.curve.t_nucl.len() as isize)
                {
                    positions.push(next_left);
                }
                if positions.last().cloned() != Some(curve.curve.t_nucl.len() as isize) {
                    positions.push(curve.curve.t_nucl.len() as isize);
                }
                for (a, b) in positions.iter().zip(positions.iter().skip(1)) {
                    let time_points = &curve.curve.t_nucl[(*a as usize)..(*b as usize)];
                    let square_per_time_for_a_b =
                        HelixTimeMap::square_per_time_for_time_map(time_points);
                    square_per_time = square_per_time.max(square_per_time_for_a_b);
                }
            }
        }

        for (h_id, h) in helices
            .iter()
            .filter(|(_, h)| h.get_revolution_curve_desc() == Some(&curve))
        {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                let nucl_time = Vec::clone(curve.curve.t_nucl.as_ref());
                time_maps.insert(
                    *h_id,
                    HelixTimeMap {
                        nucl_time: Arc::new(nucl_time),
                        nb_negative_nucl: curve.curve.nucl_t0,
                        square_per_time,
                    },
                );
            }
        }
        Self {
            time_maps,
            length_normalisation: square_per_time,
        }
    }

    pub fn get_abscissa_converter(&self, h_id: usize) -> AbscissaConverter {
        if let Some(map) = self.time_maps.get(&h_id) {
            AbscissaConverter(AbscissaConverter_::Real(map.clone()))
        } else {
            AbscissaConverter(AbscissaConverter_::Fake(self.length_normalisation))
        }
    }
}

impl PathTimeMaps {
    pub fn new(path_id: BezierPathId, helices: &[(usize, &Helix)]) -> Self {
        let mut time_maps = BTreeMap::new();

        let mut square_per_time: f64 = 1.;
        for (_, h) in helices.iter().filter(|(_, h)| h.path_id == Some(path_id)) {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                let time_points = &curve.curve.t_nucl;
                let square_per_time_for_h = HelixTimeMap::square_per_time_for_time_map(time_points);
                square_per_time = square_per_time.max(square_per_time_for_h);
            }
        }

        for (h_id, h) in helices.iter().filter(|(_, h)| h.path_id == Some(path_id)) {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                time_maps.insert(
                    *h_id,
                    HelixTimeMap {
                        nucl_time: curve.curve.t_nucl.clone(),
                        nb_negative_nucl: curve.curve.nucl_t0,
                        square_per_time,
                    },
                );
            }
        }
        Self {
            time_maps,
            length_normalisation: square_per_time,
        }
    }

    pub fn get_abscissa_converter(&self, h_id: usize) -> AbscissaConverter {
        if let Some(map) = self.time_maps.get(&h_id) {
            AbscissaConverter(AbscissaConverter_::Real(map.clone()))
        } else {
            AbscissaConverter(AbscissaConverter_::Fake(self.length_normalisation))
        }
    }
}
