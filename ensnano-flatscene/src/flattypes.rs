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
//! This modules introduces types that are used in the flatscene's data structures.
//!
//! The motivation behind these types is that flatscene's representation of helices are stored in a
//! Vec as opposed to a HashMap in the design. This means that their identifier needs to be
//! converted. For both the flatscene and the design, usize could be used but having distinct types
//! reduces the confusion, since erros will be detected by the typechecker.

use super::{HashMap, Nucl, Selection};
use ensnano_interactor::PhantomElement;

/// An helix identifier in the flatscene data structures.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct FlatIdx(pub usize);

#[derive(Debug, Clone, Copy, Hash)]
pub struct FlatHelix {
    /// The identifier of the helix in the flatscene data strucutres.
    pub flat: FlatIdx,
    /// The identifier of the helix in the designs data strucutres.
    pub real: usize,
}

impl std::cmp::PartialEq for FlatHelix {
    fn eq(&self, other: &Self) -> bool {
        self.flat == other.flat
    }
}

impl Eq for FlatHelix {}

impl std::cmp::PartialOrd for FlatHelix {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.flat.partial_cmp(&other.flat)
    }
}

impl std::cmp::Ord for FlatHelix {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.flat.cmp(&other.flat)
    }
}

impl FlatHelix {
    pub fn from_real(real: usize, helix_map: &HashMap<usize, FlatIdx>) -> Option<Self> {
        let flat = *helix_map.get(&real)?;
        Some(Self { flat, real })
    }
}

/// This trait is a marker, incating that if T:Flat, then [T] can be indexed by a FlatHelix.
pub trait Flat {}

impl<T: Flat> std::ops::Index<FlatHelix> for [T] {
    type Output = T;
    fn index(&self, idx: FlatHelix) -> &Self::Output {
        &self[idx.flat.0]
    }
}

impl<T: Flat> std::ops::Index<FlatHelix> for Vec<T> {
    type Output = T;
    fn index(&self, idx: FlatHelix) -> &Self::Output {
        &self[idx.flat.0]
    }
}

impl<T: Flat> std::ops::Index<FlatIdx> for [T] {
    type Output = T;
    fn index(&self, idx: FlatIdx) -> &Self::Output {
        &self[idx.0]
    }
}

impl<T: Flat> std::ops::Index<FlatIdx> for Vec<T> {
    type Output = T;
    fn index(&self, idx: FlatIdx) -> &Self::Output {
        &self[idx.0]
    }
}

/// The nucleotide type manipulated by the flatscene
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord, Hash)]
pub struct FlatNucl {
    pub helix: FlatHelix,
    pub position: isize,
    pub forward: bool,
}

impl FlatNucl {
    pub fn to_real(&self) -> Nucl {
        Nucl {
            helix: self.helix.real,
            position: self.position,
            forward: self.forward,
        }
    }

    pub fn from_real(real: &Nucl, id_map: &HashMap<usize, FlatIdx>) -> Option<Self> {
        let helix = FlatHelix::from_real(real.helix, id_map)?;
        Some(Self {
            helix,
            position: real.position,
            forward: real.forward,
        })
    }

    pub fn prime3(&self) -> Self {
        Self {
            position: if self.forward {
                self.position + 1
            } else {
                self.position - 1
            },
            ..*self
        }
    }

    pub fn prime5(&self) -> Self {
        Self {
            position: if self.forward {
                self.position - 1
            } else {
                self.position + 1
            },
            ..*self
        }
    }

    #[allow(dead_code)]
    pub fn left(&self) -> Self {
        Self {
            position: self.position - 1,
            ..*self
        }
    }

    #[allow(dead_code)]
    pub fn right(&self) -> Self {
        Self {
            position: self.position + 1,
            ..*self
        }
    }
}

pub enum FlatSelection {
    Nucleotide(usize, FlatNucl),
    Bound(usize, FlatNucl, FlatNucl),
    Xover(usize, usize),
    Design(usize),
    Strand(usize, usize),
    Helix(usize, FlatHelix),
    Grid(usize, usize),
    Phantom(PhantomElement),
    Nothing,
}

impl FlatSelection {
    pub fn from_real(
        selection: Option<&Selection>,
        id_map: &HashMap<usize, FlatIdx>,
    ) -> FlatSelection {
        if let Some(selection) = selection {
            match selection {
                Selection::Nucleotide(d, nucl) => {
                    if let Some(flat_nucl) = FlatNucl::from_real(nucl, id_map) {
                        Self::Nucleotide(*d as usize, flat_nucl)
                    } else {
                        Self::Nothing
                    }
                }
                Selection::Bound(d, n1, n2) => {
                    let n1 = FlatNucl::from_real(n1, id_map);
                    let n2 = FlatNucl::from_real(n2, id_map);
                    if let Some((n1, n2)) = n1.zip(n2) {
                        Self::Bound(*d as usize, n1, n2)
                    } else {
                        Self::Nothing
                    }
                }
                Selection::Xover(d, xover_id) => Self::Xover(*d as usize, *xover_id),
                Selection::Design(d) => Self::Design(*d as usize),
                Selection::Strand(d, s_id) => Self::Strand(*d as usize, *s_id as usize),
                Selection::Helix(d, h_id) => {
                    if let Some(flat_helix) = FlatHelix::from_real(*h_id as usize, id_map) {
                        Self::Helix(*d as usize, flat_helix)
                    } else {
                        Self::Nothing
                    }
                }
                Selection::Grid(d, g_id) => Self::Grid(*d as usize, *g_id),
                Selection::Phantom(pe) => Self::Phantom(pe.clone()),
                Selection::Nothing => Self::Nothing,
            }
        } else {
            Self::Nothing
        }
    }
}

pub struct HelixVec<T: Flat>(Vec<T>);

impl<T: Flat> std::ops::Index<FlatIdx> for HelixVec<T> {
    type Output = T;

    fn index(&self, idx: FlatIdx) -> &T {
        &self.0[idx.0]
    }
}

impl<T: Flat> std::ops::IndexMut<FlatIdx> for HelixVec<T> {
    fn index_mut(&mut self, idx: FlatIdx) -> &mut Self::Output {
        &mut self.0[idx.0]
    }
}

impl<T: Flat> std::ops::Deref for HelixVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Flat> std::ops::DerefMut for HelixVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Flat> HelixVec<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn remove(&mut self, idx: FlatIdx) -> T {
        self.0.remove(idx.0)
    }

    pub fn push(&mut self, value: T) {
        self.0.push(value)
    }

    pub fn get(&self, idx: FlatIdx) -> Option<&T> {
        self.0.get(idx.0)
    }
}
