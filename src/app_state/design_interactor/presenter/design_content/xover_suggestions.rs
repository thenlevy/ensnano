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

use super::{Design, Nucl, SuggestionParameters};
use ahash::RandomState;
use std::collections::{BTreeMap, HashMap, HashSet};
use ultraviolet::Vec3;

type CubeMap = HashMap<(isize, isize, isize), Vec<Nucl>, RandomState>;

#[derive(Default, Debug, Clone)]
pub(super) struct XoverSuggestions {
    helices_groups: BTreeMap<usize, Vec<Nucl>>,
    helices_cubes: BTreeMap<usize, CubeMap>,
    blue_nucl: Vec<Nucl>,
    red_cubes: CubeMap,
}

impl XoverSuggestions {
    pub(super) fn add_nucl(&mut self, nucl: Nucl, space_pos: Vec3, groups: &BTreeMap<usize, bool>) {
        let cube = space_to_cube(space_pos.x, space_pos.y, space_pos.z);

        self.helices_groups
            .entry(nucl.helix)
            .or_default()
            .push(nucl.clone());
        self.helices_cubes
            .entry(nucl.helix)
            .or_default()
            .entry(cube)
            .or_default()
            .push(nucl);

        match groups.get(&nucl.helix) {
            Some(true) => {
                self.blue_nucl.push(nucl);
            }
            Some(false) => {
                self.red_cubes
                    .entry(cube)
                    .or_insert(vec![])
                    .push(nucl.clone());
            }
            None => (),
        }
    }

    /// Return the list of all suggested crossovers
    pub(super) fn get_suggestions(
        &self,
        design: &Design,
        suggestion_parameters: &SuggestionParameters,
    ) -> Vec<(Nucl, Nucl)> {
        let mut ret = vec![];
        for blue_nucl in self.blue_nucl.iter() {
            let neighbour = self
                .get_possible_cross_over(design, blue_nucl, suggestion_parameters)
                .unwrap_or_default();
            for (red_nucl, dist) in neighbour {
                ret.push((*blue_nucl, red_nucl, dist))
            }
        }
        ret.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        self.trimm_suggestion(&ret)
    }

    /// Trimm a list of crossovers so that each nucleotide appears at most once in the suggestion
    /// list.
    fn trimm_suggestion(&self, suggestion: &[(Nucl, Nucl, f32)]) -> Vec<(Nucl, Nucl)> {
        let mut used = HashSet::new();
        let mut ret = vec![];
        for (a, b, _) in suggestion {
            if !used.contains(a) && !used.contains(b) {
                ret.push((*a, *b));
                used.insert(a);
                used.insert(b);
            }
        }
        ret
    }

    /// Return all the crossovers of length less than `len_crit` involving `nucl`, and their length.
    fn get_possible_cross_over(
        &self,
        design: &Design,
        nucl: &Nucl,
        suggestion_parameters: &SuggestionParameters,
    ) -> Option<Vec<(Nucl, f32)>> {
        let mut ret = Vec::new();
        let positions = design.get_nucl_position(*nucl)?;
        let cube0 = space_to_cube(positions[0], positions[1], positions[2]);

        let len_crit = 1.2;
        for i in vec![-1, 0, 1].iter() {
            for j in vec![-1, 0, 1].iter() {
                for k in vec![-1, 0, 1].iter() {
                    let cube = (cube0.0 + i, cube0.1 + j, cube0.2 + k);
                    if let Some(v) = self.red_cubes.get(&cube) {
                        for red_nucl in v {
                            if red_nucl.helix != nucl.helix {
                                if let Some(red_position) = design.get_nucl_position(*red_nucl) {
                                    let dist = (0..3)
                                        .map(|i| (positions[i], red_position[i]))
                                        .map(|(x, y)| (x - y) * (x - y))
                                        .sum::<f32>()
                                        .sqrt();
                                    if dist < len_crit
                                        && (suggestion_parameters.include_scaffold
                                            || design.get_strand_nucl(nucl) != design.scaffold_id)
                                        && (suggestion_parameters.include_scaffold
                                            || design.get_strand_nucl(red_nucl)
                                                != design.scaffold_id)
                                        && (suggestion_parameters.include_intra_strand
                                            || design.get_strand_nucl(nucl)
                                                != design.get_strand_nucl(red_nucl))
                                    {
                                        ret.push((*red_nucl, dist));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Some(ret)
    }
}

fn space_to_cube(x: f32, y: f32, z: f32) -> (isize, isize, isize) {
    let cube_len = 1.2;
    (
        x.div_euclid(cube_len) as isize,
        y.div_euclid(cube_len) as isize,
        z.div_euclid(cube_len) as isize,
    )
}
