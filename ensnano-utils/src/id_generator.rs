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
use ahash::RandomState;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct IdGenerator<K: Eq + Hash + Clone> {
    next_id: usize,
    ids: HashMap<K, usize, RandomState>,
    elements: HashMap<usize, K, RandomState>,
}

impl<K: Eq + Hash + Clone> Default for IdGenerator<K> {
    fn default() -> Self {
        Self {
            next_id: 0,
            ids: Default::default(),
            elements: Default::default(),
        }
    }
}

impl<K: Eq + Hash + Clone> IdGenerator<K> {
    #[allow(dead_code)]
    pub fn import_existing(existing: Vec<(usize, K)>) -> Self {
        use std::collections::HashSet;
        let mut used = HashSet::new();
        let mut ids: HashMap<K, usize, RandomState> = Default::default();
        let mut elements: HashMap<usize, K, RandomState> = Default::default();
        let mut next_id = 0;
        ids.reserve(existing.len());
        elements.reserve(existing.len());
        for (id, k) in existing.into_iter() {
            if !used.insert(id) {
                panic!(
                    "Error while loading ids, the id {} is used more that once",
                    id
                );
            }
            elements.insert(id, k.clone());
            ids.insert(k, id);
            next_id = next_id.max(id + 1);
        }
        Self {
            next_id,
            ids,
            elements,
        }
    }

    pub fn insert(&mut self, key: K) -> usize {
        let ret = self.next_id;
        self.elements.insert(self.next_id, key.clone());
        self.ids.insert(key, self.next_id);
        self.next_id += 1;
        ret
    }

    pub fn insert_at(&mut self, key: K, id: usize) {
        self.elements.insert(id, key.clone());
        self.ids.insert(key, id);
        self.next_id = self.next_id.max(id + 1);
    }

    pub fn get_element(&self, id: usize) -> Option<K> {
        self.elements.get(&id).cloned()
    }

    pub fn get_id(&self, element: &K) -> Option<usize> {
        self.ids.get(element).cloned()
    }

    /// Replace old_key by new_key
    #[allow(dead_code)] //used in tests
    pub fn update(&mut self, old_key: K, new_key: K) {
        if let Some(id) = self.ids.get(&old_key).cloned() {
            self.ids.insert(new_key.clone(), id);
            self.ids.remove(&old_key);
            self.elements.insert(id, new_key);
        }
    }

    #[allow(dead_code)] //used in tests
    pub fn remove(&mut self, id: usize) {
        let elt = self.get_element(id).expect("Removing unexisting id");
        self.ids.remove(&elt);
        self.elements.remove(&id);
    }

    #[allow(dead_code)] //used in tests
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty() && self.elements.is_empty()
    }

    pub fn get_all_elements(&self) -> Vec<(usize, K)> {
        self.elements.clone().into_iter().collect()
    }

    pub fn agree_on_next_id(&self, next: &mut Self) {
        next.next_id = self.next_id;
    }
}
