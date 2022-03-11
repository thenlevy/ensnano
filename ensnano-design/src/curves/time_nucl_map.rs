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
#[derive(Clone)]
pub struct HelixTimeMap {
    nucl_time: Arc<Vec<f64>>,
    nb_negative_nucl: usize,
    normalisation_time: f64,
}

impl HelixTimeMap {
    pub fn x_to_nucl_conversion(&self, x: f64) -> f64 {
        let my_time = if self.nucl_time.len() < 2 {
            x
        } else {
            let nucl_per_time_unit = (self.nucl_time.last().unwrap()
                - self.nucl_time.first().unwrap())
                / (self.nucl_time.len() as f64);
            if x < *self.nucl_time.first().unwrap() {
                let remainder = x - *self.nucl_time.first().unwrap() * nucl_per_time_unit;
                self.nb_negative_nucl as f64 - remainder
            } else if x > *self.nucl_time.last().unwrap() {
                let remainder = x - *self.nucl_time.last().unwrap() * nucl_per_time_unit;
                (self.nucl_time.len() - self.nb_negative_nucl) as f64 + remainder
            } else {
                let n = self.find_x(x);
                let remainder = x - self.nucl_time[n] * nucl_per_time_unit;
                n as f64 + remainder * nucl_per_time_unit
            }
        };
        my_time / self.normalisation_time
    }

    fn find_x(&self, x: f64) -> usize {
        let mut a = 0;
        let mut b = self.nucl_time.len() - 1;
        while b - a > 1 {
            let c = (a + b) / 2;
            if self.nucl_time[c] < x {
                a = c;
            } else {
                b = c;
            }
        }
        a
    }
}

#[derive(Clone)]
enum AbscissaConverter_ {
    Real(HelixTimeMap),
    Fake(f64),
}

#[derive(Clone)]
pub struct AbscissaConverter(AbscissaConverter_);

impl AbscissaConverter {
    pub fn x_to_nucl_conversion(&self, x: f64) -> f64 {
        match &self.0 {
            AbscissaConverter_::Real(time_map) => time_map.x_to_nucl_conversion(x),
            AbscissaConverter_::Fake(normalisation_time) => x * normalisation_time,
        }
    }
}

pub(crate) struct PathTimeMaps {
    time_maps: BTreeMap<usize, HelixTimeMap>,
    normalisation_time: f64,
}

impl PathTimeMaps {
    fn new(path_id: BezierPathId, helices: &[(usize, &Helix)]) -> Self {
        let mut time_maps = BTreeMap::new();

        let mut normalisation_time: f64 = 1.;
        for (_, h) in helices.iter().filter(|(_, h)| h.path_id == Some(path_id)) {
            if let Some(curve) = h.instanciated_curve.as_ref() {
                let time_points = &curve.curve.t_nucl;
                for (a, b) in time_points.iter().zip(time_points.iter().skip(1)) {
                    normalisation_time = normalisation_time.min(b - a);
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
                        normalisation_time,
                    },
                );
            }
        }
        Self {
            time_maps,
            normalisation_time,
        }
    }

    pub fn get_abscissa_converter(&self, h_id: usize) -> AbscissaConverter {
        if let Some(map) = self.time_maps.get(&h_id) {
            AbscissaConverter(AbscissaConverter_::Real(map.clone()))
        } else {
            AbscissaConverter(AbscissaConverter_::Fake(self.normalisation_time))
        }
    }
}
