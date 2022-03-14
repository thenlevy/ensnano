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
use crate::HasMap;

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default, Hash,
)]
/// Identifier of a free grid
pub struct FreeGridId(pub(super) usize);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Collection of free grids descriptor
pub struct FreeGrids(pub(super) Arc<BTreeMap<FreeGridId, Arc<GridDescriptor>>>);

impl HasMap for FreeGrids {
    type Key = FreeGridId;
    type Item = GridDescriptor;
    fn get_map(&self) -> &BTreeMap<Self::Key, Arc<Self::Item>> {
        &self.0
    }
}

impl FreeGrids {
    pub fn make_mut(&mut self) -> FreeGridsMut {
        FreeGridsMut {
            new_map: BTreeMap::clone(&self.0),
            source: self,
        }
    }

    pub fn from_vec(vec: Vec<GridDescriptor>) -> Self {
        Self(Arc::new(
            vec.into_iter()
                .enumerate()
                .map(|(id, grid)| (FreeGridId(id), Arc::new(grid)))
                .collect(),
        ))
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

pub struct FreeGridsMut<'a> {
    source: &'a mut FreeGrids,
    new_map: BTreeMap<FreeGridId, Arc<GridDescriptor>>,
}

impl<'a> FreeGridsMut<'a> {
    pub fn push(&mut self, desc: GridDescriptor) -> GridId {
        let new_key = self
            .new_map
            .keys()
            .max()
            .map(|m| FreeGridId(m.0 + 1))
            .unwrap_or_default();
        self.new_map.insert(new_key, Arc::new(desc));
        GridId::FreeGrid(new_key.0)
    }
}

impl<'a> Drop for FreeGridsMut<'a> {
    fn drop(&mut self) {
        *self.source = FreeGrids(Arc::new(std::mem::take(&mut self.new_map)))
    }
}
