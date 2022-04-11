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
    length_normalisation: f64,
}

impl HelixTimeMap {
    pub fn x_to_nucl_conversion(&self, x: f64) -> f64 {
        if self.nucl_time.len() < 2 {
            x / self.length_normalisation
        } else {
            let time_per_x = 1. / self.length_normalisation;

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
        }
    }

    pub fn nucl_to_x_convertion(&self, n: isize) -> f64 {
        if self.nucl_time.len() < 2 {
            n as f64 * self.length_normalisation
        } else if n < -(self.nb_negative_nucl as isize)
            || n >= (self.nucl_time.len() - self.nb_negative_nucl) as isize
        {
            n as f64 * self.length_normalisation
                / (self.nucl_time.last().unwrap() - self.nucl_time.first().unwrap())
        } else {
            let x_per_time = self.length_normalisation;
            self.nucl_time[(n + self.nb_negative_nucl as isize) as usize] * x_per_time
        }
    }

    pub fn x_conversion(&self, x: f64) -> f64 {
        if self.nucl_time.len() < 2 {
            x * self.length_normalisation
        } else if x < -(self.nb_negative_nucl as f64)
            || x >= (self.nucl_time.len() - self.nb_negative_nucl) as f64
        {
            x * self.length_normalisation
                / (self.nucl_time.last().unwrap() - self.nucl_time.first().unwrap())
        } else {
            let x_per_time = self.length_normalisation;
            let idx = (x.floor() as isize + self.nb_negative_nucl as isize) as usize;
            if let Some(next) = self.nucl_time.get(idx + 1) {
                (self.nucl_time[idx] + x.fract() * (*next - self.nucl_time[idx])) * x_per_time
            } else {
                self.nucl_time[idx] * x_per_time
                    + x.fract() / (self.nucl_time.last().unwrap() - self.nucl_time.first().unwrap())
                        * x_per_time
            }
        }
    }

    fn find_time(&self, time: f64) -> usize {
        let mut a = 0;
        let mut b = self.nucl_time.len() - 1;
        while b - a > 1 {
            let c = (a + b) / 2;
            if self.nucl_time[c] < time {
                a = c;
            } else {
                b = c;
            }
        }
        a
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

        let mut length_normalisation: f64 = 1.;

        for (_, h) in helices
            .iter()
            .filter(|(_, h)| h.get_revolution_curve_desc() == Some(curve))
        {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                let time_points = &curve.curve.t_nucl;
                if time_points.len() > 2 {
                    let x_per_time = (time_points.len() as f64 - 1.)
                        / (time_points.last().unwrap() - time_points.first().unwrap());
                    length_normalisation = length_normalisation.max(x_per_time);
                }
            }
        }

        for (h_id, h) in helices
            .iter()
            .filter(|(_, h)| h.get_revolution_curve_desc() == Some(&curve))
        {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                time_maps.insert(
                    *h_id,
                    HelixTimeMap {
                        nucl_time: curve.curve.t_nucl.clone(),
                        nb_negative_nucl: curve.curve.nucl_t0,
                        length_normalisation,
                    },
                );
            }
        }
        Self {
            time_maps,
            length_normalisation,
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

        let mut length_normalisation: f64 = 1.;
        for (_, h) in helices.iter().filter(|(_, h)| h.path_id == Some(path_id)) {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                let time_points = &curve.curve.t_nucl;
                if time_points.len() > 2 {
                    let x_per_time = (time_points.len() as f64 - 1.)
                        / (time_points.last().unwrap() - time_points.first().unwrap());
                    length_normalisation = length_normalisation.max(x_per_time);
                }
            }
        }

        for (h_id, h) in helices.iter().filter(|(_, h)| h.path_id == Some(path_id)) {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                time_maps.insert(
                    *h_id,
                    HelixTimeMap {
                        nucl_time: curve.curve.t_nucl.clone(),
                        nb_negative_nucl: curve.curve.nucl_t0,
                        length_normalisation,
                    },
                );
            }
        }
        Self {
            time_maps,
            length_normalisation,
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
