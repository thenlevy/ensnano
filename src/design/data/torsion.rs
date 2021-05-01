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
//! This modules defines the method that return the torsions in a design

use super::roller::cross_over_force;
use super::*;

type Xover = (Nucl, Nucl);
impl Data {
    /// Return a HashMap mapping each cross-over of the design to the torsion induced by this
    /// cross-over.
    pub fn get_torsions(&self) -> HashMap<Xover, Torsion> {
        let mut torsions: HashMap<Xover, Torsion> = HashMap::new();
        let helices: BTreeMap<usize, Helix> = self.design.helices.clone();
        let xovers = self.design.get_xovers();
        let parameters = self.design.parameters.unwrap_or_default();
        for xover in xovers.into_iter() {
            let torsion = xover_torsion(&helices, xover.0, xover.1, &parameters);
            let mut insert = true;
            for (candidate, torsion_friend) in torsions.iter_mut() {
                if torsion_friend.friend.is_some() {
                    continue;
                }
                if let Some(b) = are_friends(xover, *candidate) {
                    insert = false;
                    if b {
                        torsion_friend.strength_prime5 += torsion.strength_prime5;
                        torsion_friend.strength_prime3 += torsion.strength_prime3;
                        torsion_friend.friend = Some(xover);
                    } else {
                        torsion_friend.strength_prime5 += torsion.strength_prime3;
                        torsion_friend.strength_prime3 += torsion.strength_prime5;
                        torsion_friend.friend = Some((xover.1, xover.0));
                    }
                    break;
                }
            }
            if insert {
                torsions.insert(xover, torsion);
            }
        }
        torsions
    }
}

/// Represent the torsion applied on each helices implied in a cross_over.
///
/// The strength is defined as the cross-over's component in the radial acceleration of the helix
pub struct Torsion {
    /// The strength applied on the 5' helix of the cross over
    pub strength_prime5: f32,
    /// The strength applied on the 3' helix of the cross over
    pub strength_prime3: f32,
    /// Two cross-overs are fiends if their extremities are neighbour. In that case only one of
    /// of them should appear in the keys of the torsion map, and their strength are combined
    pub friend: Option<Xover>,
}

/// Return the torsion induced by a cross-over on each of the implied helices.
/// The strength is defined as the cross-over's component in the radial acceleration of the helix
fn xover_torsion(
    helices: &BTreeMap<usize, Helix>,
    source: Nucl,
    target: Nucl,
    parameters: &Parameters,
) -> Torsion {
    let strength = cross_over_force(
        &helices[&source.helix],
        &helices[&target.helix],
        parameters,
        source.position,
        source.forward,
        target.position,
        target.forward,
    );
    Torsion {
        strength_prime5: strength.0,
        strength_prime3: strength.1,
        friend: None,
    }
}

/// Return true iff the extremities of xover1 and xover2 are neighbour.
fn are_friends(xover1: Xover, xover2: Xover) -> Option<bool> {
    if xover1.0.is_neighbour(&xover2.0) && xover1.1.is_neighbour(&xover2.1) {
        Some(true)
    } else if xover1.1.is_neighbour(&xover2.0) && xover1.0.is_neighbour(&xover2.1) {
        Some(false)
    } else {
        None
    }
}
