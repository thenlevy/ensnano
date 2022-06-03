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

use std::collections::BTreeMap;
use std::sync::Arc;

pub trait Collection {
    type Key;
    type Item;
    fn get(&self, id: &Self::Key) -> Option<&Self::Item>;
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a Self::Key, &'a Self::Item)> + 'a>;
    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Item> + 'a>;
    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Key> + 'a>;
    fn len(&self) -> usize;
    fn contains_key(&self, id: &Self::Key) -> bool;
}

pub trait HasMap {
    type Key: Ord + Eq;
    type Item;
    fn get_map(&self) -> &BTreeMap<Self::Key, Arc<Self::Item>>;
}

impl<T> Collection for T
where
    T: HasMap,
{
    type Key = <T as HasMap>::Key;
    type Item = <T as HasMap>::Item;

    fn get(&self, id: &T::Key) -> Option<&Self::Item> {
        self.get_map().get(id).map(|arc| arc.as_ref())
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a Self::Key, &'a Self::Item)> + 'a> {
        Box::new(self.get_map().iter().map(|(id, arc)| (id, arc.as_ref())))
    }

    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Key> + 'a> {
        Box::new(self.get_map().keys())
    }

    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Item> + 'a> {
        Box::new(self.get_map().values().map(|arc| arc.as_ref()))
    }

    fn len(&self) -> usize {
        self.get_map().len()
    }

    fn contains_key(&self, id: &Self::Key) -> bool {
        self.get_map().contains_key(id)
    }
}
