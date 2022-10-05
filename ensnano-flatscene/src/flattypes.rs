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
use ensnano_design::grid::GridId;
use ensnano_interactor::PhantomElement;
use std::collections::BTreeMap;

/// An helix identifier in the flatscene data structures.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct FlatIdx(pub usize);

#[derive(Debug, Clone, Copy)]
pub struct FlatHelix {
    /// The identifier of the helix in the flatscene data strucutres.
    pub flat: FlatIdx,
    /// The segment that the helix represents
    pub segment: HelixSegment,
    pub segment_left: Option<isize>,
}

impl std::cmp::PartialEq for FlatHelix {
    fn eq(&self, other: &Self) -> bool {
        self.flat == other.flat
    }
}

use std::hash::{Hash, Hasher};

impl Hash for FlatHelix {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.flat.hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HelixSegment {
    pub helix_idx: usize,
    pub segment_idx: usize,
}

#[derive(Clone, Default)]
pub struct FlatHelixMaps {
    flat_to_real: BTreeMap<FlatIdx, HelixSegment>,
    real_to_flat: HashMap<HelixSegment, FlatIdx>,
    segments: HashMap<usize, Vec<isize>>,
}

impl FlatHelixMaps {
    pub fn clear_maps(&mut self) {
        self.flat_to_real.clear();
        self.real_to_flat.clear();
    }

    pub fn insert_segments(&mut self, helix_id: usize, segments: Vec<isize>) {
        self.segments.insert(helix_id, segments);
    }

    pub fn contains_segment(&self, segment: HelixSegment) -> bool {
        self.real_to_flat.contains_key(&segment)
    }

    pub fn insert_segment_key(&mut self, flat_idx: FlatIdx, segment: HelixSegment) {
        self.flat_to_real.insert(flat_idx, segment);
        self.real_to_flat.insert(segment, flat_idx);
    }

    pub fn get_segment_idx(&self, segment: HelixSegment) -> Option<FlatIdx> {
        self.real_to_flat.get(&segment).cloned()
    }

    pub fn get_segment(&self, idx: FlatIdx) -> Option<HelixSegment> {
        self.flat_to_real.get(&idx).cloned()
    }

    pub fn get_max_right(&self, segment: HelixSegment) -> Option<isize> {
        self.segments
            .get(&segment.helix_idx)
            .and_then(|segments| segments.get(segment.segment_idx).cloned())
    }

    pub fn get_min_left(&self, segment: HelixSegment) -> Option<isize> {
        self.segments.get(&segment.helix_idx).and_then(|segments| {
            if segment.segment_idx > 0 {
                segments.get(segment.segment_idx - 1).cloned()
            } else {
                None
            }
        })
    }

    pub fn flat_nucl_to_real(&self, flat_nucl: FlatNucl) -> Option<Nucl> {
        let segment_idx = self.flat_to_real.get(&flat_nucl.helix.flat)?;
        let segment_left = self
            .segments
            .get(&segment_idx.helix_idx)
            .and_then(|segments| segments.get(segment_idx.segment_idx))?;
        Some(Nucl {
            helix: segment_idx.helix_idx,
            position: flat_nucl.flat_position.to_real(Some(*segment_left)),
            forward: flat_nucl.forward,
        })
    }

    pub fn real_nucl_to_flat(&self, nucl: Nucl) -> Option<FlatNucl> {
        let segment_idx = self.get_segment_containing_pos(nucl.helix, nucl.position)?;

        let segment_left = if segment_idx == 0 {
            None
        } else {
            self.segments
                .get(&nucl.helix)
                .and_then(|segments| segments.get(segment_idx - 1))
                .cloned()
        };
        let flat = self.get_segment_idx(HelixSegment {
            helix_idx: nucl.helix,
            segment_idx,
        })?;
        Some(FlatNucl {
            helix: FlatHelix {
                flat,
                segment: HelixSegment {
                    helix_idx: nucl.helix,
                    segment_idx,
                },
                segment_left,
            },
            flat_position: FlatPosition::from_real(nucl.position, segment_left),
            forward: nucl.forward,
        })
    }

    pub fn get_segment_containing_pos(&self, helix_id: usize, position: isize) -> Option<usize> {
        let segment = self.segments.get(&helix_id)?;
        for (i, left) in segment.iter().enumerate() {
            if position < *left {
                return Some(i);
            }
        }
        Some(segment.len())
    }

    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&HelixSegment, &FlatIdx)> + 'a> {
        Box::new(self.real_to_flat.iter())
    }

    pub fn len(&self) -> usize {
        self.flat_to_real.len()
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
    pub fn from_real(segment: HelixSegment, helix_map: &FlatHelixMaps) -> Option<Self> {
        let flat = *helix_map.real_to_flat.get(&segment)?;

        let segment_left = if segment.segment_idx == 0 {
            None
        } else {
            helix_map
                .segments
                .get(&segment.helix_idx)
                .and_then(|segments| segments.get(segment.segment_idx - 1))
                .cloned()
        };
        Some(Self {
            flat,
            segment,
            segment_left,
        })
    }
}

/// This trait is a marker, indicating that if T:Flat, then [T] can be indexed by a FlatHelix.
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

/// The position of a flat nucleotide. If the flat nucleotide belongs to an helix2d representing a
/// a segment of a real helix, this position is relative to the leftmost extrimity of the segment.
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord, Hash)]
pub struct FlatPosition(pub isize);

/// The nucleotide type manipulated by the flatscene
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord, Hash)]
pub struct FlatNucl {
    pub helix: FlatHelix,
    pub flat_position: FlatPosition,
    pub forward: bool,
}

impl FlatPosition {
    pub fn from_real(real: isize, segment_left: Option<isize>) -> Self {
        Self(real - segment_left.unwrap_or(0))
    }

    pub fn to_real(self, segment_left: Option<isize>) -> isize {
        self.0 + segment_left.unwrap_or(0)
    }

    pub fn left(self) -> Self {
        Self(self.0 - 1)
    }

    pub fn right(self) -> Self {
        Self(self.0 + 1)
    }
}

impl FlatNucl {
    pub fn to_real(self) -> Nucl {
        Nucl {
            helix: self.helix.segment.helix_idx,
            position: self.flat_position.to_real(self.helix.segment_left),
            forward: self.forward,
        }
    }

    pub fn from_real(real: &Nucl, id_map: &FlatHelixMaps) -> Option<Self> {
        id_map.real_nucl_to_flat(*real)
    }

    pub fn prime3(&self) -> Self {
        if self.forward {
            self.right()
        } else {
            self.left()
        }
    }

    pub fn prime5(&self) -> Self {
        if self.forward {
            self.left()
        } else {
            self.right()
        }
    }

    pub fn left(&self) -> Self {
        Self {
            flat_position: self.flat_position.left(),
            ..*self
        }
    }

    pub fn right(&self) -> Self {
        Self {
            flat_position: self.flat_position.right(),
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
    Grid(usize, GridId),
    Phantom(PhantomElement),
    Nothing,
}

impl FlatSelection {
    pub fn from_real(selection: Option<&Selection>, id_map: &FlatHelixMaps) -> FlatSelection {
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
                Selection::Helix {
                    design_id,
                    helix_id,
                    ..
                } => {
                    if let Some(flat_helix) = FlatHelix::from_real(
                        HelixSegment {
                            helix_idx: *helix_id,
                            segment_idx: 0,
                        },
                        id_map,
                    ) {
                        Self::Helix(*design_id as usize, flat_helix)
                    } else {
                        Self::Nothing
                    }
                }
                Selection::Grid(d, g_id) => Self::Grid(*d as usize, *g_id),
                Selection::Phantom(pe) => Self::Phantom(*pe),
                Selection::Nothing => Self::Nothing,
                Selection::BezierControlPoint { .. } => Self::Nothing,
                Selection::BezierTengent { .. } => Self::Nothing,
                Selection::BezierVertex(_) => Self::Nothing,
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

    pub fn get_mut(&mut self, idx: FlatIdx) -> Option<&mut T> {
        self.0.get_mut(idx.0)
    }
}
