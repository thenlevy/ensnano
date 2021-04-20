use ahash::RandomState;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug, Default)]
pub struct IdGenerator<K: Eq + Hash + Clone> {
    next_id: usize,
    ids: HashMap<K, usize, RandomState>,
    elements: HashMap<usize, K, RandomState>,
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

    pub fn insert(&mut self, key: K) {
        self.elements.insert(self.next_id, key.clone());
        self.ids.insert(key, self.next_id);
        self.next_id += 1;
    }

    pub fn get_element(&mut self, id: usize) -> Option<K> {
        self.elements.get(&id).cloned()
    }

    pub fn get_id(&mut self, element: &K) -> Option<usize> {
        self.ids.get(element).cloned()
    }

    /// Replace old_key by new_key
    pub fn update(&mut self, old_key: K, new_key: K) {
        if let Some(id) = self.ids.get(&old_key).cloned() {
            self.ids.insert(new_key.clone(), id);
            self.ids.remove(&old_key);
            self.elements.insert(id, new_key);
        }
    }
}
