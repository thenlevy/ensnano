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
type Xover = (Nucl, Nucl);
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
